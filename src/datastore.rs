use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::error::{DataStoreError, ImageStoreError};
use crate::Error;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DataStore {
    store: HashMap<PathBuf, Vec<u8>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImageStore {
    store: HashMap<PathBuf, Vec<u8>>,
}

impl DataStore {
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

    pub fn contains_key(&self, k: &Path) -> bool {
        self.store.contains_key(k)
    }

    pub fn clear(&mut self) {
        self.store.clear()
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &PathBuf> {
        self.store.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.store.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Vec<u8>> {
        self.store.values_mut()
    }

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

    pub fn remove(&mut self, k: &Path) -> Option<Vec<u8>> {
        self.store.remove(k)
    }

    pub fn get(&self, k: &Path) -> Option<&Vec<u8>> {
        self.store.get(k)
    }

    pub fn get_mut(&mut self, k: &Path) -> Option<&mut Vec<u8>> {
        self.store.get_mut(k)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &Vec<u8>)> {
        self.store.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&PathBuf, &mut Vec<u8>)> {
        self.store.iter_mut()
    }
}

impl ImageStore {
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

    pub fn contains_key(&self, k: &Path) -> bool {
        self.store.contains_key(k)
    }

    pub fn clear(&mut self) {
        self.store.clear()
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &PathBuf> {
        self.store.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.store.values()
    }

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

    pub fn remove(&mut self, k: &Path) -> Option<Vec<u8>> {
        self.store.remove(k)
    }

    pub fn get(&self, k: &Path) -> Option<&Vec<u8>> {
        self.store.get(k)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &Vec<u8>)> {
        self.store.iter()
    }

    /// Checks for valid PNG header signature.
    fn is_valid_png(bytes: &[u8]) -> bool {
        bytes[..8] == [137u8, 80, 78, 71, 13, 10, 26, 10]
    }
}
