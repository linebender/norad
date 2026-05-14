//! [`FontSource`] implementation for reading UFO data from zip archives.

use std::collections::{HashMap, HashSet};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use crate::error::FontLoadError;
use crate::font_source::FontSource;

/// A UFO source backed by a zip archive loaded into memory.
pub(crate) struct ZipSource {
    entries: HashMap<PathBuf, Vec<u8>>,
}

impl ZipSource {
    /// Open a zip archive at the given path and load all file entries into memory.
    pub fn open(path: &Path) -> Result<Self, FontLoadError> {
        let file = std::fs::File::open(path).map_err(FontLoadError::AccessUfoDir)?;
        let mut archive = zip::ZipArchive::new(file).map_err(FontLoadError::InvalidZipFile)?;

        let prefix = detect_zip_root(&mut archive);

        let mut entries = HashMap::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(FontLoadError::InvalidZipFile)?;

            if entry.is_dir() {
                continue;
            }

            let raw_path = PathBuf::from(entry.name().to_string());

            let rel_path = if let Some(ref pfx) = prefix {
                match raw_path.strip_prefix(pfx) {
                    Ok(stripped) => stripped.to_path_buf(),
                    Err(_) => continue,
                }
            } else {
                raw_path
            };

            if rel_path.as_os_str().is_empty() {
                continue;
            }

            let mut data = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut data).map_err(FontLoadError::AccessUfoDir)?;
            entries.insert(rel_path, data);
        }

        Ok(ZipSource { entries })
    }
}

impl FontSource for ZipSource {
    fn try_read(&self, path: &Path) -> Option<Result<Vec<u8>, io::Error>> {
        self.entries.get(path).cloned().map(Ok)
    }

    fn list_dir(&self, path: &Path) -> Result<Vec<(PathBuf, bool)>, io::Error> {
        let mut dirs = HashSet::new();
        let mut files = Vec::new();

        for key in self.entries.keys() {
            let rel = match key.strip_prefix(path) {
                Ok(r) => r,
                Err(_) => continue,
            };

            let mut components = rel.components();
            let first = match components.next() {
                Some(c) => c,
                None => continue,
            };

            if components.next().is_some() {
                // Has more components — first is a directory.
                dirs.insert(PathBuf::from(first.as_os_str()));
            } else {
                // Single component — it's a file.
                files.push((PathBuf::from(first.as_os_str()), false));
            }
        }

        let mut result: Vec<(PathBuf, bool)> = dirs.into_iter().map(|d| (d, true)).collect();
        result.append(&mut files);
        Ok(result)
    }
}

/// Detect if the zip has a single top-level directory wrapping all contents.
///
/// If so, return that directory name as the prefix to strip. Otherwise return
/// `None`, meaning the zip contents are at the root level.
fn detect_zip_root<R: io::Read + io::Seek>(archive: &mut zip::ZipArchive<R>) -> Option<PathBuf> {
    let mut top_level_dirs = HashSet::new();
    let mut has_root_files = false;

    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(i) else { continue };
        let name = entry.name();

        // Skip macOS metadata.
        if name.starts_with("__MACOSX") {
            continue;
        }

        let path = PathBuf::from(name);
        let mut components = path.components();
        if let Some(first) = components.next() {
            if components.next().is_none() && !entry.is_dir() {
                has_root_files = true;
            } else {
                top_level_dirs.insert(PathBuf::from(first.as_os_str()));
            }
        }
    }

    if !has_root_files && top_level_dirs.len() == 1 {
        top_level_dirs.into_iter().next()
    } else {
        None
    }
}
