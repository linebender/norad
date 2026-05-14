#![cfg(feature = "ufoz")]

use std::path::{Path, PathBuf};

use norad::error::FontLoadError;
use norad::{DataRequest, Font};
use tempfile::NamedTempFile;

/// Walk a `.ufo` directory on disk and write all files into a zip archive.
/// If `wrap_in_dir` is `Some("Foo.ufo")`, every entry is prefixed with that directory.
fn ufo_dir_to_zip(ufo_path: &Path, wrap_in_dir: Option<&str>) -> NamedTempFile {
    let tmp = NamedTempFile::new().unwrap();
    let mut writer = zip::ZipWriter::new(std::fs::File::create(tmp.path()).unwrap());
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for entry in walkdir(ufo_path) {
        let rel = entry.strip_prefix(ufo_path).unwrap();
        let zip_name = match wrap_in_dir {
            Some(dir) => format!("{}/{}", dir, rel.to_string_lossy()),
            None => rel.to_string_lossy().to_string(),
        };
        writer.start_file(&zip_name, opts).unwrap();
        let data = std::fs::read(&entry).unwrap();
        std::io::Write::write_all(&mut writer, &data).unwrap();
    }
    writer.finish().unwrap();
    tmp
}

/// Recursively collect all file paths under `dir`.
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_recursive(dir, &mut files);
    files.sort();
    files
}

fn walk_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            walk_recursive(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// Create a zip with extra __MACOSX entries injected.
fn ufo_dir_to_zip_with_macosx(ufo_path: &Path, wrap_dir: &str) -> NamedTempFile {
    let tmp = NamedTempFile::new().unwrap();
    let mut writer = zip::ZipWriter::new(std::fs::File::create(tmp.path()).unwrap());
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    // Inject __MACOSX junk entries first.
    writer.start_file("__MACOSX/._DS_Store", opts).unwrap();
    std::io::Write::write_all(&mut writer, b"junk").unwrap();
    writer.start_file(format!("__MACOSX/{}/._fontinfo.plist", wrap_dir), opts).unwrap();
    std::io::Write::write_all(&mut writer, b"more junk").unwrap();

    for entry in walkdir(ufo_path) {
        let rel = entry.strip_prefix(ufo_path).unwrap();
        let zip_name = format!("{}/{}", wrap_dir, rel.to_string_lossy());
        writer.start_file(&zip_name, opts).unwrap();
        let data = std::fs::read(&entry).unwrap();
        std::io::Write::write_all(&mut writer, &data).unwrap();
    }
    writer.finish().unwrap();
    tmp
}

fn minimal_zip_entries(entries: &[(&str, &[u8])]) -> NamedTempFile {
    let tmp = NamedTempFile::new().unwrap();
    let mut writer = zip::ZipWriter::new(std::fs::File::create(tmp.path()).unwrap());
    let opts =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, data) in entries {
        writer.start_file(*name, opts).unwrap();
        std::io::Write::write_all(&mut writer, data).unwrap();
    }
    writer.finish().unwrap();
    tmp
}

// --- Tests ---

#[test]
fn load_ufoz_matches_directory() {
    let ufo_path = Path::new("testdata/MutatorSansLightWide.ufo");
    let dir_font = Font::load(ufo_path).unwrap();

    let zip_file = ufo_dir_to_zip(ufo_path, None);
    let zip_font = Font::load(zip_file.path()).unwrap();

    assert_eq!(dir_font, zip_font);

    // Spot-check fields that Store::PartialEq doesn't deeply compare.
    assert_eq!(zip_font.glyph_count(), dir_font.glyph_count());
    assert_eq!(zip_font.iter_layers().count(), dir_font.iter_layers().count());
    assert_eq!(zip_font.kerning, dir_font.kerning);
    assert_eq!(zip_font.groups, dir_font.groups);
    assert_eq!(zip_font.features, dir_font.features);
    assert_eq!(zip_font.lib, dir_font.lib);
    assert_eq!(zip_font.font_info, dir_font.font_info);
}

#[test]
fn load_ufoz_with_wrapper_dir() {
    let ufo_path = Path::new("testdata/MutatorSansLightWide.ufo");
    let dir_font = Font::load(ufo_path).unwrap();

    let zip_file = ufo_dir_to_zip(ufo_path, Some("MutatorSansLightWide.ufo"));
    let zip_font = Font::load(zip_file.path()).unwrap();

    assert_eq!(dir_font, zip_font);
    assert_eq!(zip_font.glyph_count(), dir_font.glyph_count());
    assert_eq!(zip_font.features, dir_font.features);
}

#[test]
fn load_ufoz_with_macosx_entries() {
    let ufo_path = Path::new("testdata/MutatorSansLightWide.ufo");
    let dir_font = Font::load(ufo_path).unwrap();

    let zip_file = ufo_dir_to_zip_with_macosx(ufo_path, "MutatorSansLightWide.ufo");
    let zip_font = Font::load(zip_file.path()).unwrap();

    assert_eq!(dir_font, zip_font);
}

#[test]
fn load_ufoz_data_request_none() {
    let ufo_path = Path::new("testdata/MutatorSansLightWide.ufo");
    let zip_file = ufo_dir_to_zip(ufo_path, None);

    let font = Font::load_requested_data(zip_file.path(), DataRequest::none()).unwrap();

    assert_eq!(font.iter_layers().count(), 1);
    assert!(font.default_layer().is_empty());
    assert!(font.lib.is_empty());
    assert!(font.groups.is_empty());
    assert!(font.kerning.is_empty());
    assert!(font.features.is_empty());
}

#[test]
fn load_ufoz_data_and_images() {
    let ufo_path = Path::new("testdata/dataimagetest.ufo");
    let dir_font = Font::load(ufo_path).unwrap();

    let zip_file = ufo_dir_to_zip(ufo_path, None);
    let zip_font = Font::load(zip_file.path()).unwrap();

    assert_eq!(dir_font, zip_font);

    // Verify data store contents.
    assert_eq!(zip_font.data.len(), dir_font.data.len());
    assert_eq!(zip_font.images.len(), dir_font.images.len());

    // Spot-check actual data content.
    let zip_data = zip_font.data.get(Path::new("a.txt")).unwrap().unwrap();
    let dir_data = dir_font.data.get(Path::new("a.txt")).unwrap().unwrap();
    assert_eq!(&*zip_data, &*dir_data);
}

#[test]
fn load_zip_missing_metainfo() {
    let lc = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><array>
<array><string>public.default</string><string>glyphs</string></array>
</array></plist>"#;
    let contents = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict></dict></plist>"#;

    let tmp =
        minimal_zip_entries(&[("layercontents.plist", lc), ("glyphs/contents.plist", contents)]);

    let result = Font::load(tmp.path());
    assert!(matches!(result, Err(FontLoadError::MissingMetaInfoFile)));
}
