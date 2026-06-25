//! Storage structures for UFO data and images.

use std::{
    cell::RefCell,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::error::{StoreEntryError, StoreError};
use crate::font_source::{DirEntry, FontSource};

/// A generic file store for UFO [data][spec_data] and [images][spec_images],
/// mapping [`PathBuf`] keys to [`Vec<u8>`] values.
///
/// The store provides a basic HashMap-like interface for checking data in and out.
/// If initialized from a filesystem-backed source, data is loaded lazily on access.
/// Otherwise, data is loaded eagerly during construction.
/// Data is wrapped in a [`std::sync::Arc`] to help on-demand loading.
///
/// Note that it tracks files, not directories. Data paths you insert must not have
/// any existing path in the store as an ancestor, or you would nest a file under a
/// file. Images must always be in a flat directory. The paths are always relative to
/// a UFO's data directory.
///
/// This type supports partial equality testing that is based on path comparisons.
///
/// # Examples
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
pub struct Store<T> {
    items: HashMap<PathBuf, RefCell<Item>>,
    /// When `Some`, the source is filesystem-backed and lazy loading is used.
    /// When `None`, all items were eagerly loaded during construction.
    ufo_root: Option<PathBuf>,
    impl_type: T,
}

/// Implements custom behavior for the data store.
#[derive(Debug, Default, Clone)]
#[doc(hidden)]
pub struct Data;

/// Lazy access to the contents of the UFO's `data` directory.
pub type DataStore = Store<Data>;

/// Implements custom behavior for the images store.
#[derive(Debug, Default, Clone)]
#[doc(hidden)]
pub struct Image;

/// Lazy access to the contents of the UFO's `images` directory.
pub type ImageStore = Store<Image>;

/// Defines custom behavior for data and images stores.
#[doc(hidden)]
pub trait DataType: Default {
    fn try_list_contents(&self, source: &dyn FontSource) -> Result<Vec<PathBuf>, StoreEntryError>;
    fn try_load_item(&self, source: &dyn FontSource, path: &Path) -> Result<Vec<u8>, StoreError>;
    fn validate_entry(
        &self,
        path: &Path,
        items: &HashMap<PathBuf, RefCell<Item>>,
        data: &[u8],
    ) -> Result<(), StoreError>;
}

#[derive(Debug, Clone, Default)]
#[doc(hidden)]
pub enum Item {
    #[default]
    NotLoaded,
    Loaded(Arc<[u8]>),
    Error(StoreError),
}

// Implement custom Default for Store because automatically deriving it requires
// making the error type E implement Default as well, which makes no sense.
impl<T> Default for Store<T>
where
    T: Default,
{
    fn default() -> Self {
        Self { items: Default::default(), ufo_root: None, impl_type: T::default() }
    }
}

/// Implements path testing-based partial equality for `[Store<T>]`.
impl<T: DataType> PartialEq for Store<T> {
    fn eq(&self, other: &Self) -> bool {
        self.items.len() == other.items.len()
            && self.items.keys().all(|key| other.items.contains_key(key))
    }
}

impl DataType for Data {
    fn try_list_contents(&self, source: &dyn FontSource) -> Result<Vec<PathBuf>, StoreEntryError> {
        let source_root = Path::new(crate::font::DATA_DIR);
        let mut paths = Vec::new();

        let mut dir_queue: Vec<PathBuf> = vec![source_root.to_path_buf()];
        while let Some(dir_path) = dir_queue.pop() {
            let entries = source
                .list_dir(&dir_path)
                .map_err(|e| StoreEntryError::new(dir_path.clone(), e.into()))?;

            for entry in entries {
                match entry {
                    DirEntry::Dir(name) => dir_queue.push(dir_path.join(name)),
                    DirEntry::File(name) => {
                        let full_rel = dir_path.join(name);
                        let key = full_rel.strip_prefix(source_root).unwrap().to_path_buf();
                        paths.push(key);
                    }
                }
            }
        }

        Ok(paths)
    }

    fn try_load_item(&self, source: &dyn FontSource, path: &Path) -> Result<Vec<u8>, StoreError> {
        source.read(&Path::new(crate::font::DATA_DIR).join(path)).map_err(Into::into)
    }

    fn validate_entry(
        &self,
        path: &Path,
        items: &HashMap<PathBuf, RefCell<Item>>,
        _data: &[u8],
    ) -> Result<(), StoreError> {
        if path.as_os_str().is_empty() {
            return Err(StoreError::EmptyPath);
        }
        if path.is_absolute() {
            return Err(StoreError::PathIsAbsolute);
        }
        for ancestor in path.ancestors().skip(1) {
            if !ancestor.as_os_str().is_empty() && items.contains_key(ancestor) {
                return Err(StoreError::DirUnderFile);
            }
        }

        Ok(())
    }
}

impl DataType for Image {
    fn try_list_contents(&self, source: &dyn FontSource) -> Result<Vec<PathBuf>, StoreEntryError> {
        let source_root = Path::new(crate::font::IMAGES_DIR);
        let mut paths = Vec::new();

        let entries = source
            .list_dir(source_root)
            .map_err(|e| StoreEntryError::new(source_root.to_path_buf(), e.into()))?;

        for entry in entries {
            match entry {
                // The spec forbids directories.
                DirEntry::Dir(name) => return Err(StoreEntryError::new(name, StoreError::Subdir)),
                DirEntry::File(name) => paths.push(name),
            }
        }

        Ok(paths)
    }

    fn try_load_item(&self, source: &dyn FontSource, path: &Path) -> Result<Vec<u8>, StoreError> {
        source.read(&Path::new(crate::font::IMAGES_DIR).join(path)).map_err(Into::into)
    }

    fn validate_entry(
        &self,
        path: &Path,
        _items: &HashMap<PathBuf, RefCell<Item>>,
        data: &[u8],
    ) -> Result<(), StoreError> {
        if path.as_os_str().is_empty() {
            return Err(StoreError::EmptyPath);
        }
        if path.is_absolute() {
            return Err(StoreError::PathIsAbsolute);
        }
        if path.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
            return Err(StoreError::Subdir);
        }
        // Check for a valid PNG header signature.
        if !data.starts_with(&[137u8, 80, 78, 71, 13, 10, 26, 10]) {
            return Err(StoreError::InvalidImage);
        }

        Ok(())
    }
}

impl<T: DataType> Store<T> {
    pub(crate) fn new(source: &dyn FontSource) -> Result<Self, StoreEntryError> {
        let impl_type = T::default();
        let paths = impl_type.try_list_contents(source)?;

        if let Some(ufo_root) = source.as_path() {
            // Filesystem-backed: record paths as NotLoaded, defer reading to access time.
            let items = paths.into_iter().map(|p| (p, RefCell::new(Item::NotLoaded))).collect();
            Ok(Store { items, ufo_root: Some(ufo_root.to_path_buf()), impl_type })
        } else {
            // Non-filesystem: eagerly load everything now.
            let mut items = HashMap::new();
            for path in paths {
                let data = impl_type
                    .try_load_item(source, &path)
                    .map_err(|e| StoreEntryError::new(path.clone(), e))?;
                impl_type
                    .validate_entry(&path, &items, &data)
                    .map_err(|e| StoreEntryError::new(path.clone(), e))?;
                items.insert(path, RefCell::new(Item::Loaded(data.into())));
            }
            Ok(Store { items, ufo_root: None, impl_type })
        }
    }

    /// Returns `true` if the store contains data for the specified path.
    pub fn contains_key(&self, k: &Path) -> bool {
        self.items.contains_key(k)
    }

    /// Clears the store, removing all path-data pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Returns the number of elements in the store.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` if the store contains no elements.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns an iterator visiting all paths in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = &PathBuf> {
        self.items.keys()
    }

    /// Returns a reference to the data corresponding to the path.
    pub fn get(&self, path: &Path) -> Option<Result<Arc<[u8]>, StoreError>> {
        let cell = self.items.get(path)?;

        // If item isn't loaded, try to load it, saving the data or the error.
        // NOTE: Figure out whether the item is unloaded and immediately drop the
        //       read borrow so we can take the write borrow. Otherwise, we panic.
        if matches!(*cell.borrow(), Item::NotLoaded) {
            let ufo_root = self.ufo_root.as_deref().expect("NotLoaded item without ufo_root");
            *cell.borrow_mut() = Self::load_item(&self.impl_type, &ufo_root, path, &self.items);
        }

        match &*cell.borrow() {
            Item::Error(e) => Some(Err(e.clone())),
            Item::Loaded(data) => Some(Ok(data.clone())),
            Item::NotLoaded => unreachable!(),
        }
    }

    fn load_item(
        impl_type: &T,
        source: &dyn FontSource,
        path: &Path,
        items: &HashMap<PathBuf, RefCell<Item>>,
    ) -> Item {
        match impl_type.try_load_item(source, path) {
            Ok(data) => match impl_type.validate_entry(path, items, &data) {
                Ok(_) => Item::Loaded(data.into()),
                Err(e) => Item::Error(e),
            },
            Err(e) => Item::Error(e),
        }
    }

    /// Try to insert data for this path. Overwrites existing data.
    ///
    /// Does not return the overwritten data, use [`Self::get`] first to get it if you need
    /// it.
    ///
    /// In a data store, returns a [`StoreError`] if:
    /// 1. The path is empty.
    /// 2. The path is absolute.
    /// 3. Any of the path's ancestors is already tracked in the store, implying
    ///    the path to be nested under a file.
    ///
    /// In an images store, returns an [`StoreError`] if:
    /// 1. The path is empty.
    /// 2. The path is absolute.
    /// 3. The path contains an ancestor, implying subdirectories.
    /// 4. The image data does not start with the PNG header.
    pub fn insert(&mut self, path: PathBuf, data: Vec<u8>) -> Result<(), StoreError> {
        self.impl_type.validate_entry(&path, &self.items, &data)?;
        self.items.insert(path, RefCell::new(Item::Loaded(data.into())));
        Ok(())
    }

    /// Removes a path from the store.
    ///
    /// Does not return the removed data, use [`Self::get`] first to get it if you need
    /// it.
    pub fn remove(&mut self, k: &Path) {
        self.items.remove(k);
    }

    /// An iterator visiting all path-data pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, Result<Arc<[u8]>, StoreError>)> {
        self.items.keys().map(move |k| (k, self.get(k).unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
    fn datastore_errors() {
        let mut store = DataStore::default();

        assert!(matches!(store.insert(PathBuf::new(), vec![]), Err(StoreError::EmptyPath)));
        #[cfg(not(target_family = "windows"))]
        assert!(matches!(
            store.insert(PathBuf::from("/a"), vec![]),
            Err(StoreError::PathIsAbsolute)
        ));
        #[cfg(target_family = "windows")]
        assert!(matches!(
            store.insert(PathBuf::from("C:\\a"), vec![]),
            Err(StoreError::PathIsAbsolute)
        ));

        store.insert(PathBuf::from("a"), vec![]).unwrap();
        assert!(matches!(
            store.insert(PathBuf::from("a/b/zzz/c.txt"), vec![]),
            Err(StoreError::DirUnderFile)
        ));
    }

    #[test]
    fn imagestore_errors() {
        let mut store = ImageStore::default();

        assert!(matches!(store.insert(PathBuf::new(), vec![]), Err(StoreError::EmptyPath)));
        #[cfg(not(target_family = "windows"))]
        assert!(matches!(
            store.insert(PathBuf::from("/a"), vec![]),
            Err(StoreError::PathIsAbsolute)
        ));
        #[cfg(target_family = "windows")]
        assert!(matches!(
            store.insert(PathBuf::from("C:\\a"), vec![]),
            Err(StoreError::PathIsAbsolute)
        ));
        assert!(matches!(
            store.insert(PathBuf::from("a.png"), vec![1, 2, 3]),
            Err(StoreError::InvalidImage)
        ));
        assert!(matches!(
            store.insert(PathBuf::from("a/b/zzz/c.png"), vec![137u8, 80, 78, 71, 13, 10, 26, 10]),
            Err(StoreError::Subdir)
        ));
    }

    #[test]
    fn data_images_roundtripping() {
        let ufo = crate::Font::load(UFO_DATA_IMAGE_TEST_PATH).unwrap();

        // 1. Roundtrip font to different dir to ensure we save data and images to
        //    new destination.
        let roundtrip_dir = TempDir::new().unwrap();
        ufo.save(&roundtrip_dir).unwrap();
        std::mem::drop(ufo); // Avoid accidental use below.

        let ufo_rt = crate::Font::load(&roundtrip_dir).unwrap();

        let mut data_paths: Vec<_> = ufo_rt.data.keys().collect();
        data_paths.sort();
        assert_eq!(
            data_paths,
            vec![Path::new(PATH_A), PATH_B.as_ref(), PATH_C.as_ref(), PATH_Z.as_ref()]
        );
        assert_eq!(&*ufo_rt.data.get(PATH_A.as_ref()).unwrap().unwrap(), EXPECTED_A);
        assert_eq!(&*ufo_rt.data.get(PATH_B.as_ref()).unwrap().unwrap(), EXPECTED_B);
        assert_eq!(&*ufo_rt.data.get(PATH_C.as_ref()).unwrap().unwrap(), EXPECTED_C);
        assert_eq!(&*ufo_rt.data.get(PATH_Z.as_ref()).unwrap().unwrap(), EXPECTED_Z);

        let mut images_paths: Vec<_> = ufo_rt.images.keys().collect();
        images_paths.sort();
        assert_eq!(
            images_paths,
            vec![Path::new(PATH_IMAGE1), PATH_IMAGE2.as_ref(), PATH_IMAGE3.as_ref()]
        );

        // 2. Open font again so all data is unloaded again and save in same destination,
        //    to check that we load/unlazify the data before saving in-place.
        let ufo_rt = crate::Font::load(&roundtrip_dir).unwrap();
        ufo_rt.save(&roundtrip_dir).unwrap();
        std::mem::drop(ufo_rt); // Avoid accidental use below.

        // All data and images should still exist because Font was unlazified before saving.
        let ufo_rt = crate::Font::load(&roundtrip_dir).unwrap();

        let mut data_paths: Vec<_> = ufo_rt.data.keys().collect();
        data_paths.sort();
        assert_eq!(
            data_paths,
            vec![Path::new(PATH_A), PATH_B.as_ref(), PATH_C.as_ref(), PATH_Z.as_ref()]
        );
        assert_eq!(&*ufo_rt.data.get(PATH_A.as_ref()).unwrap().unwrap(), EXPECTED_A);
        assert_eq!(&*ufo_rt.data.get(PATH_B.as_ref()).unwrap().unwrap(), EXPECTED_B);
        assert_eq!(&*ufo_rt.data.get(PATH_C.as_ref()).unwrap().unwrap(), EXPECTED_C);
        assert_eq!(&*ufo_rt.data.get(PATH_Z.as_ref()).unwrap().unwrap(), EXPECTED_Z);

        let mut images_paths: Vec<_> = ufo_rt.images.keys().collect();
        images_paths.sort();
        assert_eq!(
            images_paths,
            vec![Path::new(PATH_IMAGE1), PATH_IMAGE2.as_ref(), PATH_IMAGE3.as_ref()]
        );
    }

    #[test]
    fn lazy_data_loading() {
        let source = Path::new(UFO_DATA_IMAGE_TEST_PATH);
        let mut store = DataStore::new(&source).unwrap();

        let mut paths: Vec<&Path> = store.keys().map(|p| p.as_ref()).collect();
        paths.sort();
        assert_eq!(
            paths,
            vec![Path::new(PATH_A), PATH_B.as_ref(), PATH_C.as_ref(), PATH_Z.as_ref()]
        );

        assert_eq!(&*store.get(PATH_A.as_ref()).unwrap().unwrap(), EXPECTED_A);
        assert_eq!(&*store.get(PATH_B.as_ref()).unwrap().unwrap(), EXPECTED_B);
        store.insert(PathBuf::from(PATH_B), b"123".to_vec()).unwrap();
        assert_eq!(*store.get(PATH_B.as_ref()).unwrap().unwrap(), b"123"[0..]);
        assert_eq!(&*store.get(PATH_C.as_ref()).unwrap().unwrap(), EXPECTED_C);
        assert_eq!(&*store.get(PATH_Z.as_ref()).unwrap().unwrap(), EXPECTED_Z);
        assert!(store.get(PATH_BOGUS.as_ref()).is_none());
        store.remove(PATH_BOGUS.as_ref());
        store.remove(PATH_B.as_ref());

        let mut paths2: Vec<(&Path, Arc<[u8]>)> =
            store.iter().map(|(k, v)| (k.as_ref(), v.unwrap())).collect();
        paths2.sort();
        assert_eq!(
            paths2,
            vec![
                (Path::new(PATH_A), EXPECTED_A.into()),
                (PATH_C.as_ref(), EXPECTED_C.into()),
                (PATH_Z.as_ref(), EXPECTED_Z.into())
            ]
        );
    }

    #[test]
    fn images_with_subdirectory() {
        let ufo = crate::Font::new();
        let dir = TempDir::new().unwrap();
        ufo.save(&dir).unwrap();

        let images_dir = dir.as_ref().join(crate::font::IMAGES_DIR);
        std::fs::create_dir(&images_dir).unwrap();
        let images_dir_subdir = images_dir.join("test");
        std::fs::create_dir(images_dir_subdir).unwrap();

        let ufo = crate::Font::load(&dir);
        assert!(ufo.is_err());
    }

    #[test]
    fn lazy_image_loading() {
        let source = Path::new(UFO_DATA_IMAGE_TEST_PATH);
        let mut store = ImageStore::new(&source).unwrap();

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
        assert_eq!(&*store.get(&path_new_image).unwrap().unwrap(), &path_new_bytes[0..]);
        assert!(store.get(PATH_BOGUS.as_ref()).is_none());
        store.remove(PATH_BOGUS.as_ref());
        store.remove(&path_new_image);
        assert!(store.get(&path_new_image).is_none());
    }

    #[test]
    fn store_equality() {
        let source = Path::new(UFO_DATA_IMAGE_TEST_PATH);
        let store1 = DataStore::new(&source).unwrap();
        let store2 = DataStore::new(&source).unwrap();

        assert_eq!(store1, store2);
    }

    // --- Helpers for synthetic on-disk stores ---

    const PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    fn png_data(extra: &[u8]) -> Vec<u8> {
        let mut v = PNG_HEADER.to_vec();
        v.extend_from_slice(extra);
        v
    }

    /// Create a temp dir with a `data/` subdirectory containing the given files.
    fn make_data_dir(files: &[(&str, &[u8])]) -> TempDir {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join(crate::font::DATA_DIR);
        std::fs::create_dir(&data_dir).unwrap();
        for (rel_path, content) in files {
            let full = data_dir.join(rel_path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        }
        dir
    }

    /// Create a temp dir with an `images/` subdirectory containing the given files.
    fn make_images_dir(files: &[(&str, &[u8])]) -> TempDir {
        let dir = TempDir::new().unwrap();
        let images_dir = dir.path().join(crate::font::IMAGES_DIR);
        std::fs::create_dir(&images_dir).unwrap();
        for (name, content) in files {
            std::fs::write(images_dir.join(name), content).unwrap();
        }
        dir
    }

    // --- Lazy loading verification ---

    #[test]
    fn data_items_not_loaded_until_accessed() {
        let dir = make_data_dir(&[("file1.txt", b"hello"), ("file2.txt", b"world")]);
        let store = DataStore::new(&dir.path()).unwrap();

        assert_eq!(store.len(), 2);
        for cell in store.items.values() {
            assert!(matches!(*cell.borrow(), Item::NotLoaded));
        }

        // Access one item — only it should become Loaded.
        let data = store.get(Path::new("file1.txt")).unwrap().unwrap();
        assert_eq!(&*data, b"hello");
        assert!(matches!(
            *store.items.get(Path::new("file1.txt")).unwrap().borrow(),
            Item::Loaded(_)
        ));
        assert!(matches!(
            *store.items.get(Path::new("file2.txt")).unwrap().borrow(),
            Item::NotLoaded
        ));
    }

    #[test]
    fn image_items_not_loaded_until_accessed() {
        let img1 = png_data(b"img1");
        let img2 = png_data(b"img2");
        let dir = make_images_dir(&[("a.png", &img1), ("b.png", &img2)]);
        let store = ImageStore::new(&dir.path()).unwrap();

        assert_eq!(store.len(), 2);
        for cell in store.items.values() {
            assert!(matches!(*cell.borrow(), Item::NotLoaded));
        }

        let data = store.get(Path::new("a.png")).unwrap().unwrap();
        assert_eq!(&*data, &img1[..]);
        assert!(matches!(*store.items.get(Path::new("a.png")).unwrap().borrow(), Item::Loaded(_)));
        assert!(matches!(*store.items.get(Path::new("b.png")).unwrap().borrow(), Item::NotLoaded));
    }

    #[test]
    fn contains_key_does_not_trigger_load() {
        let dir = make_data_dir(&[("file.txt", b"data")]);
        let store = DataStore::new(&dir.path()).unwrap();

        assert!(store.contains_key(Path::new("file.txt")));
        assert!(!store.contains_key(Path::new("nope.txt")));

        assert!(matches!(
            *store.items.get(Path::new("file.txt")).unwrap().borrow(),
            Item::NotLoaded
        ));
    }

    #[test]
    fn iter_forces_loading_all_items() {
        let dir = make_data_dir(&[("a.txt", b"aaa"), ("b.txt", b"bbb")]);
        let store = DataStore::new(&dir.path()).unwrap();

        for cell in store.items.values() {
            assert!(matches!(*cell.borrow(), Item::NotLoaded));
        }

        let results: Vec<_> = store.iter().collect();
        assert_eq!(results.len(), 2);

        for cell in store.items.values() {
            assert!(matches!(*cell.borrow(), Item::Loaded(_)));
        }
    }

    // --- Error caching ---

    #[test]
    fn data_error_cached_on_load_failure() {
        let dir = make_data_dir(&[("ephemeral.txt", b"will vanish")]);
        let store = DataStore::new(&dir.path()).unwrap();

        assert!(store.contains_key(Path::new("ephemeral.txt")));

        // Delete the file after the store listed it.
        std::fs::remove_file(dir.path().join(crate::font::DATA_DIR).join("ephemeral.txt")).unwrap();

        // First access: IO error.
        let r1 = store.get(Path::new("ephemeral.txt")).unwrap();
        assert!(r1.is_err());
        assert!(matches!(
            *store.items.get(Path::new("ephemeral.txt")).unwrap().borrow(),
            Item::Error(_)
        ));

        // verify that the error is persisted
        let r2 = store.items.get(Path::new("ephemeral.txt")).unwrap();
        assert!(matches!(*r2.borrow(), Item::Error(_)));
    }

    #[test]
    fn image_invalid_png_returns_error_on_access() {
        let dir = make_images_dir(&[("bad.png", b"not a png at all")]);
        let store = ImageStore::new(&dir.path()).unwrap();

        assert!(store.contains_key(Path::new("bad.png")));

        let result = store.get(Path::new("bad.png")).unwrap();
        assert!(matches!(result, Err(StoreError::InvalidImage)));
    }

    // --- Empty and missing directories ---

    #[test]
    fn empty_data_directory() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join(crate::font::DATA_DIR)).unwrap();

        let store = DataStore::new(&dir.path()).unwrap();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
        assert_eq!(store.keys().count(), 0);
    }

    #[test]
    fn empty_images_directory() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir(dir.path().join(crate::font::IMAGES_DIR)).unwrap();

        let store = ImageStore::new(&dir.path()).unwrap();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn missing_data_directory_is_error() {
        let dir = TempDir::new().unwrap();
        assert!(DataStore::new(&dir.path()).is_err());
    }

    #[test]
    fn missing_images_directory_is_error() {
        let dir = TempDir::new().unwrap();
        assert!(ImageStore::new(&dir.path()).is_err());
    }

    // --- Nested data directories ---

    #[test]
    fn nested_data_directories_discovered() {
        let dir = make_data_dir(&[
            ("top.txt", b"top"),
            ("a/middle.txt", b"middle"),
            ("a/b/c/deep.txt", b"deep"),
        ]);
        let store = DataStore::new(&dir.path()).unwrap();

        assert_eq!(store.len(), 3);
        assert!(store.contains_key(Path::new("top.txt")));
        assert!(store.contains_key(Path::new("a/middle.txt")));
        assert!(store.contains_key(Path::new("a/b/c/deep.txt")));

        assert_eq!(&*store.get(Path::new("top.txt")).unwrap().unwrap(), b"top");
        assert_eq!(&*store.get(Path::new("a/middle.txt")).unwrap().unwrap(), b"middle");
        assert_eq!(&*store.get(Path::new("a/b/c/deep.txt")).unwrap().unwrap(), b"deep");
    }

    // --- Mutation after lazy init ---

    #[test]
    fn data_store_clear_resets() {
        let dir = make_data_dir(&[("a.txt", b"aaa"), ("b.txt", b"bbb")]);
        let mut store = DataStore::new(&dir.path()).unwrap();

        assert_eq!(store.len(), 2);
        store.clear();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
        assert!(store.get(Path::new("a.txt")).is_none());
    }

    #[test]
    fn store_inequality() {
        let dir1 = make_data_dir(&[("a.txt", b"aaa")]);
        let dir2 = make_data_dir(&[("b.txt", b"bbb")]);

        let store1 = DataStore::new(&dir1.path()).unwrap();
        let store2 = DataStore::new(&dir2.path()).unwrap();
        assert_ne!(store1, store2);
    }

    #[test]
    fn equality_ignores_load_state() {
        let dir = make_data_dir(&[("x.txt", b"xxx")]);
        let store1 = DataStore::new(&dir.path()).unwrap();
        let store2 = DataStore::new(&dir.path()).unwrap();

        // Load in store1 but not store2.
        let _ = store1.get(Path::new("x.txt"));
        assert!(matches!(*store1.items.get(Path::new("x.txt")).unwrap().borrow(), Item::Loaded(_)));
        assert!(matches!(*store2.items.get(Path::new("x.txt")).unwrap().borrow(), Item::NotLoaded));

        // Equality is path-based, so they're still equal.
        assert_eq!(store1, store2);
    }

    // --- MemorySource for testing eager (non-filesystem) loading ---

    /// A FontSource backed by in-memory data, with `as_path() -> None`.
    /// This triggers the eager-loading branch in `Store::new`.
    #[derive(Default)]
    struct MemorySource {
        files: HashMap<PathBuf, Vec<u8>>,
        dirs: HashMap<PathBuf, Vec<DirEntry>>,
    }

    impl MemorySource {
        fn add_file(&mut self, path: impl Into<PathBuf>, data: Vec<u8>) {
            self.files.insert(path.into(), data);
        }

        fn add_dir(&mut self, path: impl Into<PathBuf>, entries: Vec<DirEntry>) {
            self.dirs.insert(path.into(), entries);
        }
    }

    impl FontSource for MemorySource {
        fn try_read(&self, path: &Path) -> Option<Result<Vec<u8>, std::io::Error>> {
            self.files.get(path).cloned().map(Ok)
        }

        fn list_dir(&self, path: &Path) -> Result<Vec<DirEntry>, std::io::Error> {
            self.dirs.get(path).cloned().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::NotFound, format!("{path:?} not found"))
            })
        }
    }

    // --- Eager loading tests ---

    #[test]
    fn data_eager_loading_from_memory_source() {
        let mut source = MemorySource::default();
        source
            .add_dir("data", vec![DirEntry::File("a.txt".into()), DirEntry::File("b.txt".into())]);
        source.add_file("data/a.txt", b"aaa".to_vec());
        source.add_file("data/b.txt", b"bbb".to_vec());

        let store = DataStore::new(&source).unwrap();

        // All items should be Loaded immediately (no lazy loading).
        assert_eq!(store.len(), 2);
        assert!(store.ufo_root.is_none());
        for cell in store.items.values() {
            assert!(matches!(*cell.borrow(), Item::Loaded(_)));
        }

        assert_eq!(&*store.get(Path::new("a.txt")).unwrap().unwrap(), b"aaa");
        assert_eq!(&*store.get(Path::new("b.txt")).unwrap().unwrap(), b"bbb");
    }

    #[test]
    fn image_eager_loading_from_memory_source() {
        let img = png_data(b"test");
        let mut source = MemorySource::default();
        source.add_dir("images", vec![DirEntry::File("x.png".into())]);
        source.add_file("images/x.png", img.clone());

        let store = ImageStore::new(&source).unwrap();

        assert_eq!(store.len(), 1);
        assert!(store.ufo_root.is_none());
        assert!(matches!(*store.items.get(Path::new("x.png")).unwrap().borrow(), Item::Loaded(_)));
        assert_eq!(&*store.get(Path::new("x.png")).unwrap().unwrap(), &img[..]);
    }

    #[test]
    fn eager_load_invalid_image_fails_at_construction() {
        let mut source = MemorySource::default();
        source.add_dir("images", vec![DirEntry::File("bad.png".into())]);
        source.add_file("images/bad.png", b"not a png".to_vec());

        // Eager loading validates during construction, so this should fail.
        let result = ImageStore::new(&source);
        assert!(result.is_err());
    }

    #[test]
    fn eager_load_nested_data_from_memory_source() {
        let mut source = MemorySource::default();
        source.add_dir("data", vec![DirEntry::File("top.txt".into()), DirEntry::Dir("sub".into())]);
        source.add_dir("data/sub", vec![DirEntry::File("deep.txt".into())]);
        source.add_file("data/top.txt", b"top".to_vec());
        source.add_file("data/sub/deep.txt", b"deep".to_vec());

        let store = DataStore::new(&source).unwrap();

        assert_eq!(store.len(), 2);
        assert_eq!(&*store.get(Path::new("top.txt")).unwrap().unwrap(), b"top");
        assert_eq!(&*store.get(Path::new("sub/deep.txt")).unwrap().unwrap(), b"deep");
    }

    #[test]
    fn closure_source_returns_unsupported() {
        let source = |_path: &Path| -> Option<Result<Vec<u8>, std::io::Error>> { None };
        let result = DataStore::new(&source);
        assert!(result.is_err());
    }
}
