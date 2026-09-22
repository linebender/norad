//! Tests for on-demand loading with `FontReader`.

use std::path::Path;

use norad::error::FontLoadError;
use norad::{DataRequest, Font, FontReader, Name};

const V3_UFO: &str = "testdata/MutatorSansLightWide.ufo";
const V2_UFO: &str = "testdata/upconversion_kerning/glyphname_groupname_UFOv2.ufo";
const V1_UFO: &str = "testdata/upconversion_kerning/glyphname_groupname_UFOv1.ufo";

const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<FontReader>();
};

#[test]
fn index_matches_font() {
    let font = Font::load(V3_UFO).unwrap();
    let reader = FontReader::open(V3_UFO).unwrap();

    assert_eq!(reader.path(), Some(Path::new(V3_UFO)));
    assert_eq!(reader.layers().count(), font.layers.len());
    for (layer, font_layer) in reader.layers().zip(font.iter_layers()) {
        assert_eq!(layer.name(), font_layer.name());
        assert_eq!(layer.path(), font_layer.path());
        assert_eq!(layer.is_default(), font_layer.is_default());
        assert_eq!(layer.len(), font_layer.len());
        assert!(layer.glyph_names().eq(font_layer.iter().map(|g| g.name())));
    }

    let default = reader.default_layer().unwrap();
    assert!(default.is_default());
    assert_eq!(default.name(), font.default_layer().name());
    assert_eq!(reader.layer("background").unwrap().name().as_str(), "background");
    assert!(reader.layer("nope").is_none());
}

#[test]
fn load_glyph_matches_font() {
    let font = Font::load(V3_UFO).unwrap();
    let reader = FontReader::open(V3_UFO).unwrap();

    for layer in reader.layers() {
        let font_layer = font.layers.get(layer.name()).unwrap();
        for name in ["A", "B", "S"].into_iter().filter(|name| layer.contains_glyph(name)) {
            let glyph = layer.load_glyph(name).unwrap().unwrap();
            assert_eq!(&glyph, font_layer.get_glyph(name).unwrap());
        }
    }

    let default = reader.default_layer().unwrap();
    assert_eq!(default.glyph_path("A"), Some(Path::new("A_.glif")));
    assert!(default.glyph_path("nope").is_none());
    assert!(!default.contains_glyph("nope"));
    assert!(default.load_glyph("nope").is_none());
}

#[test]
fn load_layer_matches_font() {
    let font = Font::load(V3_UFO).unwrap();
    let reader = FontReader::open(V3_UFO).unwrap();
    for layer in reader.layers() {
        assert_eq!(&layer.load().unwrap(), font.layers.get(layer.name()).unwrap());
    }
}

#[test]
fn load_matches_font_load() {
    for path in
        [V3_UFO, V2_UFO, V1_UFO, "testdata/fontinfotest_v2.ufo", "testdata/dataimagetest.ufo"]
    {
        let reader = FontReader::open(path).unwrap();
        assert_eq!(reader.load(&DataRequest::all()).unwrap(), Font::load(path).unwrap(), "{path}");
    }
}

#[test]
fn v2_kerning_upconversion() {
    let reader = FontReader::open(V2_UFO).unwrap();
    assert_eq!(reader.meta().format_version, norad::FormatVersion::V2);
    let font = reader.load(&DataRequest::all()).unwrap();
    assert_eq!(font.meta.format_version, norad::FormatVersion::V3);
    assert!(font.groups.keys().any(|k| k.starts_with("public.kern1.")));
}

/// A v2 UFO's `layercontents.plist` is ignored, as in ufoLib.
///
/// AFDKO leaves one behind in some v2 UFOs that lists only its own processed
/// glyphs layer, not the default layer.
#[test]
fn v2_ignores_layercontents() {
    let tmp = tempfile::TempDir::new().unwrap();
    let ufo = tmp.path().join("font.ufo");
    copy_dir(Path::new(V2_UFO), &ufo);
    let processed = "glyphs.com.adobe.type.processedGlyphs";
    std::fs::create_dir(ufo.join(processed)).unwrap();
    plist::to_file_xml(ufo.join(processed).join("contents.plist"), &plist::Dictionary::new())
        .unwrap();
    let layercontents = vec![vec![processed.to_string(), processed.to_string()]];
    plist::to_file_xml(ufo.join("layercontents.plist"), &layercontents).unwrap();

    let reader = FontReader::open(&ufo).unwrap();
    assert_eq!(reader.layers().count(), 1);
    let default = reader.default_layer().unwrap();
    assert!(default.contains_glyph("X"));

    assert!(Font::load(&ufo).unwrap().default_layer().get_glyph("X").is_some());

    let request = DataRequest::none().filter_layers(|_, path| path == Path::new("glyphs"));
    let font = Font::load_requested_data(&ufo, request).unwrap();
    assert!(font.default_layer().get_glyph("X").is_some());
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).unwrap();
        }
    }
}

#[test]
fn load_requested_matches_font_load_requested() {
    let request = || DataRequest::none().kerning(true).groups(true);
    for path in [V3_UFO, V2_UFO, V1_UFO] {
        let reader = FontReader::open(path).unwrap();
        assert_eq!(
            reader.load(&request()).unwrap(),
            Font::load_requested_data(path, request()).unwrap(),
            "{path}"
        );
    }
}

#[test]
fn opened_with_none() {
    let reader = FontReader::open_requested(V3_UFO, &DataRequest::none()).unwrap();
    assert!(reader.default_layer().is_none());
    assert_eq!(reader.layers().count(), 0);

    let request = || DataRequest::none().lib(true);
    assert_eq!(
        reader.load(&request()).unwrap(),
        Font::load_requested_data(V3_UFO, request()).unwrap()
    );
    assert!(matches!(reader.load(&DataRequest::all()), Err(FontLoadError::MissingDefaultLayer)));
}

#[test]
fn opened_with_layer_filter() {
    let request = || DataRequest::none().filter_layers(|name, _| name == "background");
    let reader = FontReader::open_requested(V3_UFO, &request()).unwrap();
    assert!(reader.default_layer().is_none());
    assert_eq!(
        reader.layers().map(|l| l.name().clone()).collect::<Vec<_>>(),
        [Name::new("background").unwrap()]
    );
    assert_eq!(
        reader.load(&request()).unwrap(),
        Font::load_requested_data(V3_UFO, request()).unwrap()
    );
}

#[test]
fn debug_impl() {
    let reader = FontReader::open(V3_UFO).unwrap();
    let debug = format!("{reader:?}");
    assert!(debug.contains("foreground"), "{debug}");
    assert!(debug.contains("background"), "{debug}");
    assert!(debug.contains("V3"), "{debug}");
}

#[cfg(not(feature = "ufoz"))]
#[test]
fn open_file_is_not_a_dir() {
    let path = "testdata/MutatorSansLightWide.ufo/metainfo.plist";
    assert!(matches!(FontReader::open(path), Err(FontLoadError::UfoNotADir)));
    assert!(matches!(Font::load(path), Err(FontLoadError::UfoNotADir)));
}
