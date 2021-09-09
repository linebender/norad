use std::fs;
use std::path::{Path, PathBuf};

use crate::Error;

/// Return a vector of file paths in the data dir, relative to the UFO's root dir.
///
/// Note that according to the [specification], the folder can be arbitrarily
/// nested. Only files are listed, not directories. Symlinks cause an error because their
/// handling is unclear.
///
/// [specification]: https://unifiedfontobject.org/versions/ufo3/data/
pub(crate) fn list_data_directory(ufo_root: &Path) -> Result<Vec<PathBuf>, Error> {
    let data_path = ufo_root.join(crate::font::DATA_DIR);
    let mut entries: Vec<PathBuf> = vec![];
    let mut dir_queue: Vec<PathBuf> = vec![data_path];

    while !dir_queue.is_empty() {
        let dir_path = dir_queue.pop().unwrap();
        for entry in fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            let attributes = entry.metadata()?;

            if attributes.is_file() {
                entries.push(path.strip_prefix(&ufo_root).unwrap().to_path_buf());
            } else if attributes.is_dir() {
                dir_queue.push(path);
            } else {
                // https://github.com/unified-font-object/ufo-spec/issues/188
                return Err(Error::InvalidDataEntry(path));
            }
        }
    }

    // The order in which `read_dir` returns entries is not guaranteed. Sort for reproducible
    // ordering.
    entries.sort();

    Ok(entries)
}

/// Return a vector of file paths in the images dir, relative to the UFO's root dir.
///
/// Note that according to the [specification], the folder must be flat.
/// Symlinks cause an error because their handling is unclear. The reference implementation
/// fontTools.ufoLib as of v4.26.2, when validation is enabled, silently skips directories
/// and files that don't start with the PNG header bytes, we instead always error out.
///
/// [specification]: https://unifiedfontobject.org/versions/ufo3/images/
pub(crate) fn list_images_directory(ufo_root: &Path) -> Result<Vec<PathBuf>, Error> {
    let images_path = ufo_root.join(crate::font::IMAGES_DIR);
    let mut entries: Vec<PathBuf> = vec![];

    for entry in fs::read_dir(images_path)? {
        let entry = entry?;
        let path = entry.path();
        let attributes = entry.metadata()?;

        if attributes.is_file() {
            entries.push(path.strip_prefix(&ufo_root).unwrap().to_path_buf());
        } else {
            // Reject directories (forbidden by spec) and symlinks.
            // https://github.com/unified-font-object/ufo-spec/issues/188
            return Err(Error::InvalidImageEntry(path));
        }
    }

    // The order in which `read_dir` returns entries is not guaranteed. Sort for reproducible
    // ordering.
    entries.sort();

    Ok(entries)
}
