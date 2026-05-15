//! Integration tests for loading fonts from a non-filesystem source.

#![cfg(feature = "no-fs")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use norad::{DataRequest, Font, FontSource};

/// A simple in-memory source that wraps a HashMap.
struct MemorySource(HashMap<PathBuf, String>);

impl FontSource for MemorySource {
    type Error = std::convert::Infallible;
    fn read_contents(&self, path: &Path) -> Result<Option<String>, Self::Error> {
        Ok(self.0.get(path).cloned())
    }
}

/// Build a MemorySource by walking a real UFO directory on disk.
fn source_from_ufo_dir(ufo_path: &str) -> MemorySource {
    let root = Path::new(ufo_path);
    let mut map = HashMap::new();
    walk_dir(root, root, &mut map);
    MemorySource(map)
}

fn walk_dir(root: &Path, dir: &Path, map: &mut HashMap<PathBuf, String>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            walk_dir(root, &path, map);
        } else {
            let rel = path.strip_prefix(root).unwrap().to_path_buf();
            let contents = std::fs::read_to_string(&path).unwrap();
            map.insert(rel, contents);
        }
    }
}

#[test]
fn load_from_source_matches_filesystem() {
    let ufo_path = "testdata/MutatorSansLightWide.ufo";
    let source = source_from_ufo_dir(ufo_path);

    let font_fs = Font::load(ufo_path).unwrap();
    let font_src = Font::load_from_source(DataRequest::all(), &source).unwrap();

    // Core data should match
    assert_eq!(font_fs.meta, font_src.meta);
    assert_eq!(font_fs.font_info, font_src.font_info);
    assert_eq!(font_fs.lib, font_src.lib);
    assert_eq!(font_fs.groups, font_src.groups);
    assert_eq!(font_fs.kerning, font_src.kerning);
    assert_eq!(font_fs.features, font_src.features);

    // Layers and glyphs
    assert_eq!(font_fs.iter_layers().count(), font_src.iter_layers().count());
    assert_eq!(font_fs.glyph_count(), font_src.glyph_count());
    for layer_fs in font_fs.iter_layers() {
        let layer_src = font_src.layers.get(layer_fs.name()).unwrap();
        assert_eq!(layer_fs, layer_src, "layer '{}' mismatch", layer_fs.name());
    }
}

#[test]
fn load_from_source_with_closure() {
    let ufo_path = "testdata/MutatorSansLightWide.ufo";
    let source = source_from_ufo_dir(ufo_path);

    let reader = |path: &Path| -> Result<Option<String>, std::convert::Infallible> {
        Ok(source.0.get(path).cloned())
    };
    let font = Font::load_from_source(DataRequest::all(), &reader).unwrap();

    assert_eq!(font.glyph_count(), 48);
}

#[test]
fn load_from_source_data_request_none() {
    let ufo_path = "testdata/MutatorSansLightWide.ufo";
    let source = source_from_ufo_dir(ufo_path);

    let font = Font::load_from_source(DataRequest::none(), &source).unwrap();

    assert!(font.groups.is_empty());
    assert!(font.kerning.is_empty());
    assert!(font.features.is_empty());
    assert!(font.default_layer().is_empty());
}

#[test]
fn load_from_source_missing_metainfo() {
    let source = MemorySource(HashMap::new());
    let result = Font::load_from_source(DataRequest::all(), &source);
    assert!(result.is_err());
}
