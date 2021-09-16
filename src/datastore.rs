//! Storage structures for UFO data and images.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::error::{DataStoreError, ImageStoreError};
use crate::Error;

/// A store for [UFO data][spec], mapping [`PathBuf`]s to [`Vec<u8>`].
///
/// Note that it tracks files, not directories. Paths you insert must not have any
/// existing path in the store as an ancestor, or you would nest a file under a file.
/// The paths are always relative to a UFO's data directory.
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
/// ├── glyphs/
/// │   ├── a.glif
/// │   └── contents.plist
/// ├── layercontents.plist
/// └── metainfo.plist
/// ```
///
/// The `data` subfolder will be represented in the store like so:
///
/// * `PathBuf::from("a.txt")` → `b"<content>".to_vec()`
/// * `PathBuf::from("b.bin")` → `b"<content>".to_vec()`
/// * `PathBuf::from("com.testing.random/c.txt")` → `b"<content>".to_vec()`
/// * `PathBuf::from("com.testing.random/zzz/z.txt")` → `b"<content>".to_vec()`
///
/// [spec]: https://unifiedfontobject.org/versions/ufo3/data/
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataStore {
    store: HashMap<PathBuf, Vec<u8>>,
}

/// A store for [UFO images][spec], mapping [`PathBuf`]s to [`Vec<u8>`] containing PNG data.
///
/// Note that it tracks files, not directories. The paths are always relative to a
/// UFO's images directory. The images directory is flat. The images must be valid PNG files.
///
/// # Example
///
/// Consider a UFO on disk with the following structure:
///
/// ```text
/// Test.ufo/
/// ├── glyphs/
/// │   ├── a.glif
/// │   └── contents.plist
/// ├── images/
/// │   ├── image1.png
/// │   ├── image2.png
/// │   └── image3.png
/// ├── layercontents.plist
/// └── metainfo.plist
/// ```
///
/// The `images` subfolder will be represented in the store like so:
///
/// * `PathBuf::from("image1.png")` → `b"<content>".to_vec()`
/// * `PathBuf::from("image2.png")` → `b"<content>".to_vec()`
/// * `PathBuf::from("image3.png")` → `b"<content>".to_vec()`
///
/// [spec]: https://unifiedfontobject.org/versions/ufo3/images/
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImageStore {
    store: HashMap<PathBuf, Vec<u8>>,
}

impl DataStore {
    /// Create DataStore by recursively reading the `data` directory from the `ufo_root`.
    pub(crate) fn try_new_from_path(ufo_root: &Path) -> Result<Self, Error> {
        let data_path = ufo_root.join(crate::font::DATA_DIR);
        let mut store = Self::default();
        let mut dir_queue: Vec<PathBuf> = vec![data_path.clone()];

        while !dir_queue.is_empty() {
            let dir_path = dir_queue.pop().unwrap();
            for entry in std::fs::read_dir(dir_path)? {
                let entry = entry?;
                let path = entry.path();
                let attributes = entry.metadata()?; // "will not traverse symlinks"

                if attributes.is_file() {
                    let key = path.strip_prefix(&data_path).unwrap().to_path_buf();
                    let value = std::fs::read(&path)?;
                    store.try_insert(key, value).map_err(|e| Error::InvalidDataEntry(path, e))?;
                } else if attributes.is_dir() {
                    dir_queue.push(path);
                } else {
                    // The spec forbids symlinks.
                    return Err(Error::InvalidDataEntry(path, DataStoreError::NotPlainFileOrDir));
                }
            }
        }

        Ok(store)
    }

    /// Returns true if the store contains content for the specified path.
    pub fn contains_key(&self, k: &Path) -> bool {
        self.store.contains_key(k)
    }

    /// Clears the store, removing all path-content pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.store.clear()
    }

    /// Returns the number of elements in the store.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Returns `true` if the store contains no elements.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// An iterator visiting all paths in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = &PathBuf> {
        self.store.keys()
    }

    /// An iterator visiting all content in arbitrary order.
    pub fn values(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.store.values()
    }

    /// An iterator visiting all content mutably in arbitrary order.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Vec<u8>> {
        self.store.values_mut()
    }

    /// Tries to insert a path-content pair into the store.
    ///
    /// If the store did not have this path present, `None` is returned.
    ///
    /// If the store did have this path present, the content is updated, and the old
    /// content is returned. The path is not updated, though.
    ///
    /// Returns a [`DataStoreError`] if:
    /// 1. The path is empty.
    /// 2. The path is absolute.
    /// 3. Any of the path's ancestors is already tracked in the store, implying
    ///    the path to be nested under a file.
    pub fn try_insert(
        &mut self,
        k: PathBuf,
        v: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, DataStoreError> {
        if k.as_os_str().is_empty() {
            return Err(DataStoreError::EmptyPath);
        }
        if k.is_absolute() {
            return Err(DataStoreError::PathIsAbsolute);
        }
        for ancestor in k.ancestors() {
            if !ancestor.as_os_str().is_empty() && self.store.contains_key(ancestor) {
                return Err(DataStoreError::DirUnderFile);
            }
        }
        Ok(self.store.insert(k, v))
    }

    /// Removes a path from the store, returning the content at the path if the path
    /// was previously in the store.
    pub fn remove(&mut self, k: &Path) -> Option<Vec<u8>> {
        self.store.remove(k)
    }

    /// Returns a reference to the content corresponding to the path.
    pub fn get(&self, k: &Path) -> Option<&Vec<u8>> {
        self.store.get(k)
    }

    /// Returns a mutable reference to the content corresponding to the path.
    pub fn get_mut(&mut self, k: &Path) -> Option<&mut Vec<u8>> {
        self.store.get_mut(k)
    }

    /// An iterator visiting all path-content pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &Vec<u8>)> {
        self.store.iter()
    }

    /// An iterator visiting all path-content pairs in arbitrary order, with mutable
    /// references to the content.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&PathBuf, &mut Vec<u8>)> {
        self.store.iter_mut()
    }
}

/// There are no methods for getting mutable references to images to prevent circumvention of the PNG validity check.
impl ImageStore {
    /// Create ImageStore by reading the `images` directory from the `ufo_root`.
    pub(crate) fn try_new_from_path(ufo_root: &Path) -> Result<Self, Error> {
        let images_path = ufo_root.join(crate::font::IMAGES_DIR);
        let mut store = Self::default();

        for entry in std::fs::read_dir(&images_path)? {
            let entry = entry?;
            let path = entry.path();
            let attributes = entry.metadata()?; // "will not traverse symlinks"

            if attributes.is_file() {
                let key = path.strip_prefix(&images_path).unwrap().to_path_buf();
                let value = std::fs::read(&path)?;
                store.try_insert(key, value).map_err(|e| Error::InvalidImageEntry(path, e))?;
            } else if attributes.is_dir() {
                // The spec forbids directories...
                return Err(Error::InvalidImageEntry(path, ImageStoreError::Subdir));
            } else {
                // ... and symlinks.
                return Err(Error::InvalidImageEntry(path, ImageStoreError::NotPlainFile));
            }
        }

        Ok(store)
    }

    /// Returns `true` if the store contains an image for the specified path.
    pub fn contains_key(&self, k: &Path) -> bool {
        self.store.contains_key(k)
    }

    /// Clears the store, removing all path-image pairs. Keeps the allocated memory for reuse.
    pub fn clear(&mut self) {
        self.store.clear()
    }

    /// Returns the number of elements in the store.
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Returns `true` if the store contains no elements.
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// An iterator visiting all paths in arbitrary order.
    pub fn keys(&self) -> impl Iterator<Item = &PathBuf> {
        self.store.keys()
    }

    /// An iterator visiting all images in arbitrary order.
    pub fn values(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.store.values()
    }

    /// Tries to insert a path-image pair into the store.
    ///
    /// If the store did not have this path present, `None` is returned.
    ///
    /// If the store did have this path present, the image is updated, and the old
    /// image is returned. The path is not updated, though.
    ///
    /// Returns an [`ImageStoreError`] if:
    /// 1. The path is empty.
    /// 2. The path is absolute.
    /// 3. The path contains an ancestor, implying subdirectories.
    /// 4. The image does not start with the PNG header.
    pub fn try_insert(
        &mut self,
        k: PathBuf,
        v: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, ImageStoreError> {
        if k.as_os_str().is_empty() {
            return Err(ImageStoreError::EmptyPath);
        }
        if k.is_absolute() {
            return Err(ImageStoreError::PathIsAbsolute);
        }
        if k.parent().map_or(false, |p| !p.as_os_str().is_empty()) {
            return Err(ImageStoreError::Subdir);
        }
        if !Self::is_valid_png(&v) {
            return Err(ImageStoreError::InvalidImage);
        }
        Ok(self.store.insert(k, v))
    }

    /// Removes a path from the store, returning the image at the path if the path
    /// was previously in the map.
    pub fn remove(&mut self, k: &Path) -> Option<Vec<u8>> {
        self.store.remove(k)
    }

    /// Returns a reference to the image corresponding to the path.
    pub fn get(&self, k: &Path) -> Option<&Vec<u8>> {
        self.store.get(k)
    }

    /// An iterator visiting all path-image pairs in arbitrary order.
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &Vec<u8>)> {
        self.store.iter()
    }

    /// Checks for a valid PNG header signature.
    fn is_valid_png(bytes: &[u8]) -> bool {
        bytes[..8] == [137u8, 80, 78, 71, 13, 10, 26, 10]
    }
}
