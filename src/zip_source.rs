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
pub(crate) fn detect_zip_root<R: io::Read + io::Seek>(
    archive: &mut zip::ZipArchive<R>,
) -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;

    /// Write a zip archive containing the given entries to a temp file.
    fn write_zip_to_tempfile(entries: &[(&str, &[u8])]) -> NamedTempFile {
        let tmp = NamedTempFile::new().unwrap();
        let mut writer = zip::ZipWriter::new(std::fs::File::create(tmp.path()).unwrap());
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            std::io::Write::write_all(&mut writer, data).unwrap();
        }
        writer.finish().unwrap();
        tmp
    }

    fn minimal_metainfo() -> Vec<u8> {
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>creator</key><string>org.linebender.norad</string>
<key>formatVersion</key><integer>3</integer>
</dict></plist>"#
            .to_vec()
    }

    fn minimal_layercontents() -> Vec<u8> {
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><array>
<array><string>public.default</string><string>glyphs</string></array>
</array></plist>"#
            .to_vec()
    }

    fn minimal_contents() -> Vec<u8> {
        br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict></dict></plist>"#
            .to_vec()
    }

    #[test]
    fn open_minimal_zip() {
        let meta = minimal_metainfo();
        let lc = minimal_layercontents();
        let contents = minimal_contents();
        let tmp = write_zip_to_tempfile(&[
            ("metainfo.plist", &meta),
            ("layercontents.plist", &lc),
            ("glyphs/contents.plist", &contents),
        ]);

        let source = ZipSource::open(tmp.path()).unwrap();
        assert_eq!(source.entries.len(), 3);
        assert!(source.entries.contains_key(Path::new("metainfo.plist")));
        assert!(source.entries.contains_key(Path::new("layercontents.plist")));
        assert!(source.entries.contains_key(Path::new("glyphs/contents.plist")));
    }

    #[test]
    fn detect_root_strips_single_dir() {
        let meta = minimal_metainfo();
        let tmp = write_zip_to_tempfile(&[
            ("MyFont.ufo/metainfo.plist", &meta),
            ("MyFont.ufo/glyphs/contents.plist", &minimal_contents()),
        ]);

        let source = ZipSource::open(tmp.path()).unwrap();
        assert!(source.entries.contains_key(Path::new("metainfo.plist")));
        assert!(source.entries.contains_key(Path::new("glyphs/contents.plist")));
        assert!(!source.entries.keys().any(|k| k.starts_with("MyFont.ufo")));
    }

    #[test]
    fn detect_root_no_strip_when_multiple_top_dirs() {
        let tmp = write_zip_to_tempfile(&[("A/file1", b"a"), ("B/file2", b"b")]);
        let source = ZipSource::open(tmp.path()).unwrap();
        assert!(source.entries.contains_key(Path::new("A/file1")));
        assert!(source.entries.contains_key(Path::new("B/file2")));
    }

    #[test]
    fn detect_root_no_strip_when_root_files_exist() {
        let tmp =
            write_zip_to_tempfile(&[("readme.txt", b"hi"), ("MyFont.ufo/metainfo.plist", b"x")]);
        let source = ZipSource::open(tmp.path()).unwrap();
        assert!(source.entries.contains_key(Path::new("readme.txt")));
        assert!(source.entries.contains_key(Path::new("MyFont.ufo/metainfo.plist")));
    }

    #[test]
    fn try_read_missing() {
        let tmp = write_zip_to_tempfile(&[("metainfo.plist", b"x")]);
        let source = ZipSource::open(tmp.path()).unwrap();
        assert!(source.try_read(Path::new("nonexistent")).is_none());
    }

    #[test]
    fn list_dir_root() {
        let meta = minimal_metainfo();
        let lc = minimal_layercontents();
        let contents = minimal_contents();
        let tmp = write_zip_to_tempfile(&[
            ("metainfo.plist", &meta),
            ("layercontents.plist", &lc),
            ("glyphs/contents.plist", &contents),
            ("glyphs/A_.glif", b"<glyph/>"),
            ("data/foo.txt", b"hello"),
        ]);

        let source = ZipSource::open(tmp.path()).unwrap();
        let mut entries = source.list_dir(Path::new("")).unwrap();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let names: Vec<_> =
            entries.iter().map(|(p, is_dir)| (p.to_string_lossy().to_string(), *is_dir)).collect();
        assert!(names.contains(&("data".into(), true)));
        assert!(names.contains(&("glyphs".into(), true)));
        assert!(names.contains(&("metainfo.plist".into(), false)));
        assert!(names.contains(&("layercontents.plist".into(), false)));
    }

    #[test]
    fn list_dir_subdirectory() {
        let tmp = write_zip_to_tempfile(&[
            ("metainfo.plist", b"meta"),
            ("glyphs/contents.plist", b"x"),
            ("glyphs/A_.glif", b"y"),
        ]);

        let source = ZipSource::open(tmp.path()).unwrap();
        let mut entries = source.list_dir(Path::new("glyphs")).unwrap();
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0], (PathBuf::from("A_.glif"), false));
        assert_eq!(entries[1], (PathBuf::from("contents.plist"), false));
    }

    #[test]
    fn list_dir_empty() {
        let tmp = write_zip_to_tempfile(&[("metainfo.plist", b"x")]);
        let source = ZipSource::open(tmp.path()).unwrap();
        let entries = source.list_dir(Path::new("images")).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn open_invalid_zip() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"not a zip file at all").unwrap();
        let result = ZipSource::open(tmp.path());
        assert!(matches!(result, Err(FontLoadError::InvalidZipFile(_))));
    }

    #[test]
    fn open_nonexistent_file() {
        let result = ZipSource::open(Path::new("/tmp/norad_test_nonexistent_ufoz.zip"));
        assert!(matches!(result, Err(FontLoadError::AccessUfoDir(_))));
    }

    #[test]
    fn detect_root_direct() {
        // Test detect_zip_root directly with an in-memory zip.
        let mut buf = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            writer.start_file("Root.ufo/metainfo.plist", opts).unwrap();
            std::io::Write::write_all(&mut writer, b"data").unwrap();
            writer.start_file("Root.ufo/glyphs/A_.glif", opts).unwrap();
            std::io::Write::write_all(&mut writer, b"glyph").unwrap();
            writer.finish().unwrap();
        }
        buf.set_position(0);
        let mut archive = zip::ZipArchive::new(buf).unwrap();
        assert_eq!(detect_zip_root(&mut archive), Some(PathBuf::from("Root.ufo")));
    }

    #[test]
    fn dir_entries_skipped() {
        // Create a zip with explicit directory entries.
        let tmp = NamedTempFile::new().unwrap();
        {
            let mut writer = zip::ZipWriter::new(std::fs::File::create(tmp.path()).unwrap());
            let opts = zip::write::SimpleFileOptions::default();
            writer.add_directory("glyphs/", opts).unwrap();
            writer.start_file("glyphs/A_.glif", opts).unwrap();
            std::io::Write::write_all(&mut writer, b"glyph").unwrap();
            writer.start_file("metainfo.plist", opts).unwrap();
            std::io::Write::write_all(&mut writer, b"meta").unwrap();
            writer.finish().unwrap();
        }

        let source = ZipSource::open(tmp.path()).unwrap();
        // Should have only file entries, not directory entries.
        assert_eq!(source.entries.len(), 2);
        assert!(source.entries.contains_key(Path::new("glyphs/A_.glif")));
        assert!(source.entries.contains_key(Path::new("metainfo.plist")));
    }
}
