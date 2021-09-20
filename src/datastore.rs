//! Storage structures for UFO data and images.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::error::{DataStoreError, ImageStoreError};
use crate::Error;

/// A generic file store for UFO [data][spec_data] and [images][spec_images],
/// mapping [`PathBuf`] keys to [`Vec<u8>`] values.
///
/// The store provides a basic HashMap-like interface for checking data in and out.
/// If initialized from disk, data can be loaded eagerly or lazily, as in, on access.
/// It will remember the root data directory for this purpose. This complicates the
/// accessor methods somewhat, because 1. access can fail with an IO error and 2.
/// insertion can fail.
///
/// Note that it tracks files, not directories. Data paths you insert must not have
/// any existing path in the store as an ancestor, or you would nest a file under a
/// file. Images must always be in a flat directory. The paths are always relative to
/// a UFO's data directory.
///
/// # Example
///
/// Consider a UFO on disk with the following structure:
///
/// ```text
/// Test.ufo/
/// ├── data/
/// │   ├── a.txt
/// │   ├── b.bin
/// │   └── com.testing.random/
/// │       ├── c.txt
/// │       └── zzz/
/// │           └── z.txt
/// ├── images/
/// │   ├── image1.png
/// │   ├── image2.png
/// │   └── image3.png
/// ├── glyphs/
/// │   ├── a.glif
/// │   └── contents.plist
/// ├── layercontents.plist
/// └── metainfo.plist
/// ```
///
/// The `data` subfolder will be represented in a [`DataStore`] like so:
///
/// * `PathBuf::from("a.txt")` → `b"<content>".to_vec()`
/// * `PathBuf::from("b.bin")` → `b"<content>".to_vec()`
/// * `PathBuf::from("com.testing.random/c.txt")` → `b"<content>".to_vec()`
/// * `PathBuf::from("com.testing.random/zzz/z.txt")` → `b"<content>".to_vec()`
///
/// The `images` subfolder will be represented in an [`ImageStore`] like so:
///
/// * `PathBuf::from("image1.png")` → `b"<content>".to_vec()`
/// * `PathBuf::from("image2.png")` → `b"<content>".to_vec()`
/// * `PathBuf::from("image3.png")` → `b"<content>".to_vec()`
///
/// [spec_data]: https://unifiedfontobject.org/versions/ufo3/data/
/// [spec_images]: https://unifiedfontobject.org/versions/ufo3/images/
#[derive(Debug, Clone)]
pub struct Store<T, E> {
    items: HashMap<PathBuf, Arc<RwLock<Item<E>>>>,
    ufo_root: PathBuf,
    impl_type: T,
}

/// Implements custom behavior for the data store.
#[derive(Debug, Default, Clone)]
pub struct Data;

pub type DataStore = Store<Data, DataStoreError>;

/// Implements custom behavior for the images store.
#[derive(Debug, Default, Clone)]
pub struct Image;

pub type ImageStore = Store<Image, ImageStoreError>;

/// Defines custom behavior for data and images stores.
pub trait DataType: Default {
    type Error: Clone;
    fn try_list_contents(&self, ufo_root: &Path) -> Result<Vec<PathBuf>, Error>;
    fn try_load_item(&self, ufo_root: &Path, path: &Path) -> Result<Vec<u8>, Self::Error>;
    fn validate_entry(
        &self,
        path: &Path,
        items: &HashMap<PathBuf, Arc<RwLock<Item<Self::Error>>>>,
        data: &[u8],
    ) -> Result<(), Self::Error>;
}

/// Internal placeholder enum for the data.
#[derive(Debug, Clone, PartialEq)]
pub enum Item<E> {
    Loaded(Arc<Vec<u8>>),
    NotLoaded,
    Error(E),
}

// Implement custom Default for Store because automatically deriving it requires
// making the error type E implement Default as well, which makes no sense.
impl<T, E> Default for Store<T, E>
where
    T: Default,
{
    fn default() -> Self {
        Self { items: Default::default(), ufo_root: Default::default(), impl_type: T::default() }
    }
}

/// Implements partial equality testing by loading data on demand for comparison.
impl<T: DataType> PartialEq for Store<T, T::Error> {
    fn eq(&self, other: &Self) -> bool {
        if self.items.len() != other.items.len() {
            return false;
        }

        self.iter().all(|(key, value)| {
            other.get(key).map_or(false, |value_other| match (value, value_other) {
                (Ok(s), Ok(o)) => *s == *o,
                _ => false,
            })
        })
    }
}

impl DataType for Data {
    type Error = DataStoreError;

    fn try_list_contents(&self, ufo_root: &Path) -> Result<Vec<PathBuf>, Error> {
        let source_root = ufo_root.join(crate::font::DATA_DIR);
        let mut paths = Vec::new();

        let mut dir_queue: Vec<PathBuf> = vec![source_root.clone()];
        while let Some(dir_path) = dir_queue.pop() {
            for entry in std::fs::read_dir(&dir_path)
                .map_err(|e| Error::InvalidDataEntry(dir_path.clone(), e.into()))?
            {
                let entry =
                    entry.map_err(|e| Error::InvalidDataEntry(dir_path.clone(), e.into()))?;
                let path = entry.path();
                let attributes = entry
                    .metadata() // "will not traverse symlinks"
                    .map_err(|e| Error::InvalidDataEntry(entry.path(), e.into()))?;

                if attributes.is_file() {
                    let key = path.strip_prefix(&source_root).unwrap().to_path_buf();
                    paths.push(key);
                } else if attributes.is_dir() {
                    dir_queue.push(path);
                } else {
                    // The spec forbids symlinks.
                    return Err(Error::InvalidDataEntry(path, DataStoreError::NotPlainFileOrDir));
                }
            }
        }

        Ok(paths)
    }

    fn try_load_item(&self, ufo_root: &Path, path: &Path) -> Result<Vec<u8>, Self::Error> {
        std::fs::read(ufo_root.join(crate::font::DATA_DIR).join(path)).map_err(|e| e.into())
    }

    fn validate_entry(
        &self,
        path: &Path,
        items: &HashMap<PathBuf, Arc<RwLock<Item<Self::Error>>>>,
        _data: &[u8],
    ) -> Result<(), Self::Error> {
        if path.as_os_str().is_empty() {
            return Err(Self::Error::EmptyPath);
        }
        if path.is_absolute() {
            return Err(Self::Error::PathIsAbsolute);
        }
        for ancestor in path.ancestors().skip(1) {
            if !ancestor.as_os_str().is_empty() && items.contains_key(ancestor) {
                return Err(Self::Error::DirUnderFile);
            }
        }

        Ok(())
    }
}

impl DataType for Image {
    type Error = ImageStoreError;

    fn try_list_contents(&self, ufo_root: &Path) -> Result<Vec<PathBuf>, Error> {
        let source_root = ufo_root.join(crate::font::IMAGES_DIR);
        let mut paths = Vec::new();

        for entry in std::fs::read_dir(&source_root)
            .map_err(|e| Error::InvalidImageEntry(source_root.clone(), e.into()))?
        {
            let entry =
                entry.map_err(|e| Error::InvalidImageEntry(source_root.clone(), e.into()))?;
            let path = entry.path();
            let attributes = entry
                .metadata() // "will not traverse symlinks"
                .map_err(|e| Error::InvalidImageEntry(path.clone(), e.into()))?;

            if attributes.is_file() {
                let key = path.strip_prefix(&source_root).unwrap().to_path_buf();
                paths.push(key);
            } else if attributes.is_dir() {
                // The spec forbids directories...
                return Err(Error::InvalidImageEntry(path, ImageStoreError::Subdir));
            } else {
                // ... and symlinks.
                return Err(Error::InvalidImageEntry(path, ImageStoreError::NotPlainFile));
            }
        }

        Ok(paths)
    }

    fn try_load_item(&self, ufo_root: &Path, path: &Path) -> Result<Vec<u8>, Self::Error> {
        std::fs::read(ufo_root.join(crate::font::IMAGES_DIR).join(path)).map_err(|e| e.into())
    }

    fn validate_entry(
        &self,
        path: &Path,
        _items: &HashMap<PathBuf, Arc<RwLock<Item<Self::Error>>>>,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        if path.as_os_str().is_empty() {
            return Err(ImageStoreError::EmptyPath);
        }
        if path.is_absolute() {
            return Err(ImageStoreError::PathIsAbsolute);
        }
        if path.parent().map_or(false, |p| !p.as_os_str().is_empty()) {
            return Err(ImageStoreError::Subdir);
        }
        // Check for a valid PNG header signature.
        if !data.starts_with(&[137u8, 80, 78, 71, 13, 10, 26, 10]) {
            return Err(ImageStoreError::InvalidImage);
        }

        Ok(())
    }
}

impl<T: DataType> Store<T, T::Error> {
    pub(crate) fn new(ufo_root: &Path, lazy: bool) -> Result<Self, Error> {
        let impl_type = T::default();
        let dir_contents = impl_type.try_list_contents(ufo_root)?;
        let items = dir_contents
            .into_iter()
            .map(|path| {
                (
                    path.clone(),
                    Arc::new(RwLock::new(if lazy {
                        Item::NotLoaded
                    } else {
                        match impl_type.try_load_item(&ufo_root, &path) {
                            Ok(data) => {
                                match impl_type.validate_entry(&path, &HashMap::new(), &data) {
                                    Ok(_) => Item::Loaded(Arc::new(data)),
                                    Err(e) => Item::Error(e),
                                }
                            }
                            Err(e) => Item::Error(e),
                        }
                    })),
                )
            })
            .collect();
        Ok(Store { items, ufo_root: ufo_root.to_path_buf(), impl_type })
    }

    /// Returns `true` if the store contains data for the specified path.
    pub fn contains_key(&self, k: &Path) -> bool {
        self.items.contains_key(k)
    }

    /// Clears the store, removing all path-data pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.items.clear()
    }

    /// Returns the number of elements in the store.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the store contains no elements.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// An iterator visiting all paths in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = &PathBuf> {
        self.items.keys()
    }

    /// Returns a reference to the data corresponding to the path.
    pub fn get(&self, path: &Path) -> Option<Result<Arc<Vec<u8>>, T::Error>> {
        let lock = match self.items.get(path) {
            Some(lock) => lock,
            None => return None,
        };

        // If item isn't loaded, try to load it, saving the data or the error
        // NOTE: Figure out whether the item is unloaded and immediately drop the
        //       read lock so we can take the write lock. Otherwise, deadlock.
        if matches!(*lock.read().unwrap(), Item::NotLoaded) {
            // Acquire exclusive access to the item so we can load and store data in peace.
            let mut guard = lock.write().unwrap();
            if let Item::NotLoaded = *guard {
                *guard = match self.impl_type.try_load_item(&self.ufo_root, path) {
                    Ok(data) => match self.impl_type.validate_entry(&path, &self.items, &data) {
                        Ok(_) => Item::Loaded(Arc::new(data)),
                        Err(e) => Item::Error(e),
                    },
                    Err(e) => Item::Error(e),
                };
            }
        }

        match &*lock.read().unwrap() {
            Item::Error(e) => Some(Err(e.clone())),
            Item::Loaded(data) => Some(Ok(data.clone())),
            Item::NotLoaded => unreachable!(),
        }
    }

    /// Try to insert data for this path. Overwrites an existing data.
    ///
    /// Does not return the overwritten data, use [`get`] first to get it if you need
    /// it.
    ///
    /// In a data store, returns a [`DataStoreError`] if:
    /// 1. The path is empty.
    /// 2. The path is absolute.
    /// 3. Any of the path's ancestors is already tracked in the store, implying
    ///    the path to be nested under a file.
    ///
    /// In an images store, returns an [`ImageStoreError`] if:
    /// 1. The path is empty.
    /// 2. The path is absolute.
    /// 3. The path contains an ancestor, implying subdirectories.
    /// 4. The image data does not start with the PNG header.
    pub fn insert(&mut self, path: PathBuf, data: Vec<u8>) -> Result<(), T::Error> {
        self.impl_type.validate_entry(&path, &self.items, &data)?;
        self.items.insert(path, Arc::new(RwLock::new(Item::Loaded(Arc::new(data)))));
        Ok(())
    }

    /// Removes a path from the store.
    ///
    /// Does not return the removed data, use [`get`] first to get it if you need
    /// it.
    pub fn remove(&mut self, k: &Path) {
        self.items.remove(k);
    }

    /// An iterator visiting all path-data pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, Result<Arc<Vec<u8>>, T::Error>)> {
        self.items.keys().map(move |k| (k, self.get(k).unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use crate::data_request;

    use super::*;

    const UFO_DATA_IMAGE_TEST_PATH: &str = "testdata/dataimagetest.ufo";
    const PATH_A: &str = "a.txt";
    const PATH_B: &str = "b.bin";
    const PATH_C: &str = "com.testing.random/c.txt";
    const PATH_Z: &str = "com.testing.random/zzz/z.txt";
    const PATH_BOGUS: &str = "non-existent";
    const EXPECTED_A: &[u8] = b"Hello World";
    const EXPECTED_B: &[u8] = b"\x1c\n\n~\n\x06\n\xe2\n\x96\n,\n,\n\x8c\nL\n";
    const EXPECTED_C: &[u8] = b"World Hello\r\n";
    const EXPECTED_Z: &[u8] = b"";
    const PATH_IMAGE1: &str = "image1.png";
    const PATH_IMAGE2: &str = "image2.png";
    const PATH_IMAGE3: &str = "image3.png";

    #[test]
    fn lazy_datastore_errors() {
        let mut store = DataStore::default();

        assert!(matches!(store.insert(PathBuf::new(), vec![]), Err(DataStoreError::EmptyPath)));
        #[cfg(not(target_family = "windows"))]
        assert!(matches!(
            store.insert(PathBuf::from("/a"), vec![]),
            Err(DataStoreError::PathIsAbsolute)
        ));
        #[cfg(target_family = "windows")]
        assert!(matches!(
            store.insert(PathBuf::from("C:\\a"), vec![]),
            Err(DataStoreError::PathIsAbsolute)
        ));

        store.insert(PathBuf::from("a"), vec![]).unwrap();
        assert!(matches!(
            store.insert(PathBuf::from("a/b/zzz/c.txt"), vec![]),
            Err(DataStoreError::DirUnderFile)
        ));
    }

    #[test]
    fn lazy_imagestore_errors() {
        let mut store = ImageStore::default();

        assert!(matches!(store.insert(PathBuf::new(), vec![]), Err(ImageStoreError::EmptyPath)));
        #[cfg(not(target_family = "windows"))]
        assert!(matches!(
            store.insert(PathBuf::from("/a"), vec![]),
            Err(ImageStoreError::PathIsAbsolute)
        ));
        #[cfg(target_family = "windows")]
        assert!(matches!(
            store.insert(PathBuf::from("C:\\a"), vec![]),
            Err(ImageStoreError::PathIsAbsolute)
        ));
        assert!(matches!(
            store.insert(PathBuf::from("a.png"), vec![1, 2, 3]),
            Err(ImageStoreError::InvalidImage)
        ));
        assert!(matches!(
            store.insert(PathBuf::from("a/b/zzz/c.png"), vec![137u8, 80, 78, 71, 13, 10, 26, 10]),
            Err(ImageStoreError::Subdir)
        ));
    }

    #[test]
    fn data_images_roundtripping() {
        let mut data_request = crate::DataRequest::default();
        data_request.data_eager(false).images_eager(false);

        let ufo = crate::Font::load_requested_data(UFO_DATA_IMAGE_TEST_PATH, data_request.clone())
            .unwrap();

        // 1. Roundtrip font to different dir to ensure we save data and images to
        //    new destination.
        let roundtrip_dir = tempdir::TempDir::new("Roundtrip.ufo").unwrap();
        ufo.save(&roundtrip_dir).unwrap();
        std::mem::drop(ufo); // Avoid accidental use below.

        let ufo_rt =
            crate::Font::load_requested_data(&roundtrip_dir, data_request.clone()).unwrap();

        let mut data_paths: Vec<_> = ufo_rt.data.keys().collect();
        data_paths.sort();
        assert_eq!(
            data_paths,
            vec![Path::new(PATH_A), PATH_B.as_ref(), PATH_C.as_ref(), PATH_Z.as_ref()]
        );
        assert_eq!(*ufo_rt.data.get(PATH_A.as_ref()).unwrap().unwrap(), EXPECTED_A);
        assert_eq!(*ufo_rt.data.get(PATH_B.as_ref()).unwrap().unwrap(), EXPECTED_B);
        assert_eq!(*ufo_rt.data.get(PATH_C.as_ref()).unwrap().unwrap(), EXPECTED_C);
        assert_eq!(*ufo_rt.data.get(PATH_Z.as_ref()).unwrap().unwrap(), EXPECTED_Z);

        let mut images_paths: Vec<_> = ufo_rt.images.keys().collect();
        images_paths.sort();
        assert_eq!(
            images_paths,
            vec![Path::new(PATH_IMAGE1), PATH_IMAGE2.as_ref(), PATH_IMAGE3.as_ref()]
        );

        // 2. Open font again so all data is unloaded again and save in same destination,
        //    to check that we load/unlazify the data before saving in-place.
        let ufo_rt =
            crate::Font::load_requested_data(&roundtrip_dir, data_request.clone()).unwrap();
        ufo_rt.save(&roundtrip_dir).unwrap();
        std::mem::drop(ufo_rt); // Avoid accidental use below.

        // All data and images should still exist because Font was unlazified before saving.
        let ufo_rt =
            crate::Font::load_requested_data(&roundtrip_dir, data_request.clone()).unwrap();

        let mut data_paths: Vec<_> = ufo_rt.data.keys().collect();
        data_paths.sort();
        assert_eq!(
            data_paths,
            vec![Path::new(PATH_A), PATH_B.as_ref(), PATH_C.as_ref(), PATH_Z.as_ref()]
        );
        assert_eq!(*ufo_rt.data.get(PATH_A.as_ref()).unwrap().unwrap(), EXPECTED_A);
        assert_eq!(*ufo_rt.data.get(PATH_B.as_ref()).unwrap().unwrap(), EXPECTED_B);
        assert_eq!(*ufo_rt.data.get(PATH_C.as_ref()).unwrap().unwrap(), EXPECTED_C);
        assert_eq!(*ufo_rt.data.get(PATH_Z.as_ref()).unwrap().unwrap(), EXPECTED_Z);

        let mut images_paths: Vec<_> = ufo_rt.images.keys().collect();
        images_paths.sort();
        assert_eq!(
            images_paths,
            vec![Path::new(PATH_IMAGE1), PATH_IMAGE2.as_ref(), PATH_IMAGE3.as_ref()]
        );
    }

    #[test]
    fn lazy_data_loading() {
        run_data_store_test(DataStore::new(UFO_DATA_IMAGE_TEST_PATH.as_ref(), true).unwrap());
    }

    #[test]
    fn eager_data_loading() {
        run_data_store_test(DataStore::new(UFO_DATA_IMAGE_TEST_PATH.as_ref(), false).unwrap());
    }

    fn run_data_store_test(mut store: Store<Data, DataStoreError>) {
        let mut paths: Vec<&Path> = store.keys().map(|p| p.as_ref()).collect();
        paths.sort();
        assert_eq!(
            paths,
            vec![Path::new(PATH_A), PATH_B.as_ref(), PATH_C.as_ref(), PATH_Z.as_ref()]
        );

        assert_eq!(*store.get(PATH_A.as_ref()).unwrap().unwrap(), EXPECTED_A);
        assert_eq!(*store.get(PATH_B.as_ref()).unwrap().unwrap(), EXPECTED_B);
        store.insert(PathBuf::from(PATH_B), b"123".to_vec()).unwrap();
        assert_eq!(*store.get(PATH_B.as_ref()).unwrap().unwrap(), b"123"[0..]);
        assert_eq!(*store.get(PATH_C.as_ref()).unwrap().unwrap(), EXPECTED_C);
        assert_eq!(*store.get(PATH_Z.as_ref()).unwrap().unwrap(), EXPECTED_Z);
        assert!(store.get(PATH_BOGUS.as_ref()).is_none());
        store.remove(PATH_BOGUS.as_ref());
        store.remove(PATH_B.as_ref());

        let mut paths2: Vec<(&Path, Arc<Vec<u8>>)> =
            store.iter().map(|(k, v)| (k.as_ref(), v.unwrap())).collect();
        paths2.sort();
        assert_eq!(
            paths2,
            vec![
                (Path::new(PATH_A), Arc::new(EXPECTED_A.to_vec())),
                (PATH_C.as_ref(), Arc::new(EXPECTED_C.to_vec())),
                (PATH_Z.as_ref(), Arc::new(EXPECTED_Z.to_vec()))
            ]
        );
    }

    #[test]
    fn images_with_subdirectory() {
        let ufo = crate::Font::new();
        let dir = tempdir::TempDir::new("Test.ufo").unwrap();
        ufo.save(&dir).unwrap();

        let images_dir = dir.as_ref().join(crate::font::IMAGES_DIR);
        std::fs::create_dir(&images_dir).unwrap();
        let images_dir_subdir = images_dir.join("test");
        std::fs::create_dir(&images_dir_subdir).unwrap();

        let ufo = crate::Font::load(&dir);
        assert!(ufo.is_err());
    }

    #[test]
    fn lazy_image_loading() {
        run_image_store_test(ImageStore::new(UFO_DATA_IMAGE_TEST_PATH.as_ref(), true).unwrap());
    }

    #[test]
    fn eager_image_loading() {
        run_image_store_test(ImageStore::new(UFO_DATA_IMAGE_TEST_PATH.as_ref(), false).unwrap());
    }

    fn run_image_store_test(mut store: Store<Image, ImageStoreError>) {
        assert!(!store.is_empty());
        let mut paths: Vec<_> = store.keys().collect();
        paths.sort();
        assert_eq!(paths, vec![Path::new(PATH_IMAGE1), PATH_IMAGE2.as_ref(), PATH_IMAGE3.as_ref()]);

        for (_, data) in store.iter() {
            assert!(data.is_ok());
        }

        let path_new_image = PathBuf::from("image4.png");
        let path_new_bytes = vec![137u8, 80, 78, 71, 13, 10, 26, 10, 1, 2, 3];
        assert!(store.get(&path_new_image).is_none());
        store.insert(path_new_image.clone(), path_new_bytes.clone()).unwrap();
        assert_eq!(*store.get(&path_new_image).unwrap().unwrap(), &path_new_bytes[0..]);
        assert!(store.get(PATH_BOGUS.as_ref()).is_none());
        store.remove(PATH_BOGUS.as_ref());
        store.remove(&path_new_image);
        assert!(store.get(&path_new_image).is_none());
    }

    #[cfg(feature = "rayon")]
    #[test]
    fn lazy_data_loading_parallel() {
        use rayon::iter::{IntoParallelIterator, ParallelIterator};

        let store = DataStore::new(UFO_DATA_IMAGE_TEST_PATH.as_ref(), true).unwrap();

        (1..100)
            .into_par_iter()
            .for_each(|_| assert_eq!(*store.get(PATH_A.as_ref()).unwrap().unwrap(), EXPECTED_A))

        // TODO: check the file was read from disk only once.
    }

    #[test]
    fn lazy_eager_equality() {
        let ufo_path = UFO_DATA_IMAGE_TEST_PATH.as_ref();
        let store1 = DataStore::new(ufo_path, true).unwrap();
        let store2 = DataStore::new(ufo_path, false).unwrap();

        assert_eq!(store1, store2);
    }
}
