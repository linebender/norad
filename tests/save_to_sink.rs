//! Integration tests for saving fonts to a non-filesystem FontSink.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use norad::{Font, FontSink, QuoteChar, WriteOptions};

/// A simple in-memory sink that wraps a HashMap.
#[derive(Default)]
struct MemorySink(Mutex<HashMap<PathBuf, Vec<u8>>>);

impl FontSink for MemorySink {
    fn write(&self, path: &Path, data: &[u8]) -> Result<(), io::Error> {
        self.0.lock().unwrap().insert(path.to_owned(), data.to_vec());
        Ok(())
    }
}

impl MemorySink {
    fn into_inner(self) -> HashMap<PathBuf, Vec<u8>> {
        self.0.into_inner().unwrap()
    }
}

/// Collect the contents of a real UFO directory on disk into a map of
/// UFO-relative paths.
fn files_in_ufo_dir(ufo_path: &Path) -> HashMap<PathBuf, Vec<u8>> {
    let mut map = HashMap::new();
    walk_dir(ufo_path, ufo_path, &mut map);
    map
}

fn walk_dir(root: &Path, dir: &Path, map: &mut HashMap<PathBuf, Vec<u8>>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            walk_dir(root, &path, map);
        } else {
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            let contents = std::fs::read(&path).unwrap();
            map.insert(rel, contents);
        }
    }
}

/// The UFOs used for fs-vs-sink comparisons: one multi-layer, one with
/// data and images stores.
static TEST_UFOS: &[&str] = &["testdata/MutatorSansLightWide.ufo", "testdata/dataimagetest.ufo"];

fn assert_sink_matches_filesystem(ufo_path: &str, options: &WriteOptions) {
    let font = Font::load(ufo_path).unwrap();

    let dir = tempfile::TempDir::new().unwrap();
    let fs_path = dir.path().join("font.ufo");
    font.save_with_options(&fs_path, options).unwrap();
    let fs_files = files_in_ufo_dir(&fs_path);

    let sink = MemorySink::default();
    font.save_to_sink(&sink, options).unwrap();
    let sink_files = sink.into_inner();

    let mut fs_paths: Vec<_> = fs_files.keys().collect();
    let mut sink_paths: Vec<_> = sink_files.keys().collect();
    fs_paths.sort();
    sink_paths.sort();
    assert_eq!(fs_paths, sink_paths, "file set mismatch for {ufo_path}");

    for (path, contents) in &sink_files {
        assert_eq!(contents, &fs_files[path], "file '{}' mismatch for {ufo_path}", path.display());
    }
}

/// Saving to an in-memory sink should produce the same files, byte for byte,
/// as saving to disk.
#[test]
fn save_to_sink_matches_filesystem() {
    for ufo_path in TEST_UFOS {
        assert_sink_matches_filesystem(ufo_path, &WriteOptions::default());
    }
}

/// As above, but with custom serialization options, exercising the
/// quote-style rewriting for both destinations.
#[test]
fn save_to_sink_matches_filesystem_custom_options() {
    let options =
        WriteOptions::default().indent(WriteOptions::SPACE, 2).quote_char(QuoteChar::Single);
    for ufo_path in TEST_UFOS {
        assert_sink_matches_filesystem(ufo_path, &options);
    }

    // spot-check that the single-quote declaration actually made it through
    let font = Font::load("testdata/MutatorSansLightWide.ufo").unwrap();
    let sink = MemorySink::default();
    font.save_to_sink(&sink, &options).unwrap();
    let files = sink.into_inner();
    let metainfo = &files[Path::new("metainfo.plist")];
    assert!(metainfo.starts_with(b"<?xml version='1.0' encoding='UTF-8'?>"));
}

/// A closure can be used as an ad-hoc sink.
#[test]
fn save_to_sink_with_closure() {
    let font = Font::load("testdata/MutatorSansLightWide.ufo").unwrap();

    let files = Mutex::new(HashMap::<PathBuf, Vec<u8>>::new());
    let sink = |path: &Path, data: &[u8]| {
        files.lock().unwrap().insert(path.to_owned(), data.to_vec());
        Ok(())
    };
    font.save_to_sink(&sink, &WriteOptions::default()).unwrap();

    let files = files.into_inner().unwrap();
    assert!(files.contains_key(Path::new("metainfo.plist")));
    assert!(files.contains_key(Path::new("glyphs/contents.plist")));
    let total_glyphs = font.iter_layers().map(|l| l.len()).sum::<usize>();
    let glif_count = files.keys().filter(|p| p.extension().is_some_and(|e| e == "glif")).count();
    assert_eq!(glif_count, total_glyphs);
}

/// A font loaded from memory and saved to memory should survive a round trip.
#[test]
fn memory_round_trip() {
    let ufo_path = Path::new("testdata/MutatorSansLightWide.ufo");
    let source_files = files_in_ufo_dir(ufo_path);
    let source = move |path: &Path| source_files.get(path).cloned().map(Ok::<_, io::Error>);

    let font = Font::load_from_source(&norad::DataRequest::all(), &source).unwrap();

    let sink = MemorySink::default();
    font.save_to_sink(&sink, &WriteOptions::default()).unwrap();
    let saved_files = sink.into_inner();

    let reload_source = move |path: &Path| saved_files.get(path).cloned().map(Ok::<_, io::Error>);
    let reloaded = Font::load_from_source(&norad::DataRequest::all(), &reload_source).unwrap();

    assert_eq!(font.font_info, reloaded.font_info);
    assert_eq!(font.lib, reloaded.lib);
    assert_eq!(font.groups, reloaded.groups);
    assert_eq!(font.kerning, reloaded.kerning);
    assert_eq!(font.features, reloaded.features);
    assert_eq!(font.glyph_count(), reloaded.glyph_count());
    for layer in font.iter_layers() {
        assert_eq!(Some(layer), reloaded.layers.get(layer.name()), "layer mismatch");
    }
}

/// A failing sink write surfaces as an error.
#[test]
fn sink_errors_propagate() {
    let font = Font::load("testdata/MutatorSansLightWide.ufo").unwrap();
    let sink = |_: &Path, _: &[u8]| Err(io::Error::other("sink full"));
    assert!(font.save_to_sink(&sink, &WriteOptions::default()).is_err());
}
