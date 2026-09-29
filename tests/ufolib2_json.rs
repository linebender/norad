#![cfg(feature = "unstable-json")]
//! Testing that ufoLib2's JSON format loads to the same `Font` as the matching UFO.

use std::path::{Path, PathBuf};

use norad::error::JsonLoadError;
use norad::{Color, Font, Glyph};
use plist::Value;
use pretty_assertions::assert_eq;

const JSON_DIR: &str = "testdata/ufolib2_json";

/// Load `ufo` and `json_name`, and check that the two fonts are equivalent.
fn check_fixture(ufo: &str, json_name: &str) {
    let from_ufo = Font::load(Path::new("testdata").join(ufo)).unwrap();
    let from_json = load_json(json_name);
    assert_equivalent(&from_ufo, &from_json);
}

fn load_json(name: &str) -> Font {
    Font::load_ufolib2_json(Path::new(JSON_DIR).join(format!("{name}.json"))).unwrap()
}

fn sorted_keys<T: norad::datastore::DataType>(store: &norad::datastore::Store<T>) -> Vec<PathBuf> {
    let mut keys: Vec<_> = store.keys().cloned().collect();
    keys.sort();
    keys
}

fn assert_stores_equal<T: norad::datastore::DataType>(
    what: &str,
    a: &norad::datastore::Store<T>,
    b: &norad::datastore::Store<T>,
) {
    assert_eq!(sorted_keys(a), sorted_keys(b), "{what}: paths differ");
    for key in a.keys() {
        let a_bytes = a.get(key).unwrap().unwrap();
        let b_bytes = b.get(key).unwrap().unwrap();
        assert!(a_bytes == b_bytes, "{what}: contents of {key:?} differ");
    }
}

/// Compare everything the JSON format can carry.
///
/// Not compared: `meta` (the JSON has no metainfo), and `Layer` paths and
/// glyph file names (the JSON has no directory layout, so these are made up by
/// the loader). This is also why `Layer` and `Font` aren't compared with `==`.
fn assert_equivalent(from_ufo: &Font, from_json: &Font) {
    assert_eq!(from_ufo.font_info, from_json.font_info, "font_info differs");
    assert_eq!(from_ufo.lib, from_json.lib, "font lib differs");
    assert_eq!(from_ufo.groups, from_json.groups, "groups differ");
    assert_eq!(from_ufo.kerning, from_json.kerning, "kerning differs");
    assert_eq!(from_ufo.features, from_json.features, "features differ");
    assert_stores_equal("data", &from_ufo.data, &from_json.data);
    assert_stores_equal("images", &from_ufo.images, &from_json.images);

    let ufo_names: Vec<_> = from_ufo.layers.iter().map(|l| l.name().to_string()).collect();
    let json_names: Vec<_> = from_json.layers.iter().map(|l| l.name().to_string()).collect();
    assert_eq!(ufo_names, json_names, "layer names (or their order) differ");
    assert_eq!(
        from_ufo.default_layer().name(),
        from_json.default_layer().name(),
        "default layer differs"
    );
    assert!(from_json.default_layer().is_default());

    for (ufo_layer, json_layer) in from_ufo.layers.iter().zip(from_json.layers.iter()) {
        let layer = ufo_layer.name();
        assert_eq!(ufo_layer.color, json_layer.color, "layer {layer:?}: color differs");
        assert_eq!(ufo_layer.lib, json_layer.lib, "layer {layer:?}: lib differs");

        let ufo_glyphs: Vec<_> = ufo_layer.iter().map(|g| g.name().to_string()).collect();
        let json_glyphs: Vec<_> = json_layer.iter().map(|g| g.name().to_string()).collect();
        assert_eq!(ufo_glyphs, json_glyphs, "layer {layer:?}: glyph sets differ");

        for glyph in ufo_layer.iter() {
            let other = json_layer.get_glyph(glyph.name()).unwrap();
            assert_eq!(glyph, other, "layer {layer:?}: glyph {:?} differs", glyph.name());
        }
    }
}

#[test]
fn mutatorsans_light_wide() {
    check_fixture("MutatorSansLightWide.ufo", "MutatorSansLightWide");
}

#[test]
fn dataimagetest() {
    check_fixture("dataimagetest.ufo", "dataimagetest");
}

#[test]
fn fontinfotest() {
    check_fixture("fontinfotest.ufo", "fontinfotest");
}

/// UFO v1 keeps PostScript hinting data and OpenType features in the lib, under
/// `org.robofab.*` keys. Norad moves them into `font_info` and `features` (and
/// removes the lib keys) when it upconverts; ufoLib2 doesn't upconvert on read,
/// so the JSON still has the lib keys, and no such `info` fields or features.
/// Everything else must still match, so those differences are checked
/// explicitly and the rest is compared with the upconverted data copied over.
#[test]
fn fontinfotest_v1() {
    const ROBOFAB_KEYS: [&str; 4] = [
        "org.robofab.postScriptHintData",
        "org.robofab.opentype.classes",
        "org.robofab.opentype.featureorder",
        "org.robofab.opentype.features",
    ];
    let from_ufo = Font::load("testdata/fontinfotest_v1.ufo").unwrap();
    let mut from_json = load_json("fontinfotest_v1");

    for key in ROBOFAB_KEYS {
        assert!(!from_ufo.lib.contains_key(key), "{key} left in lib by norad");
        assert!(from_json.lib.contains_key(key), "{key} missing from JSON lib");
        from_json.lib.remove(key);
    }
    assert!(from_json.features.is_empty());
    assert!(!from_ufo.features.is_empty());
    assert_eq!(from_ufo.font_info.postscript_blue_values.as_ref().map(Vec::len), Some(8));
    assert_eq!(from_json.font_info.postscript_blue_values, None);

    from_json.features = from_ufo.features.clone();
    let ufo_info = &from_ufo.font_info;
    let json_info = &mut from_json.font_info;
    macro_rules! copy_hint_fields {
        ($($field:ident),*) => { $(
            assert!(json_info.$field.is_none(), "JSON has {}", stringify!($field));
            json_info.$field = ufo_info.$field.clone();
        )* };
    }
    copy_hint_fields!(
        postscript_blue_fuzz,
        postscript_blue_scale,
        postscript_blue_shift,
        postscript_blue_values,
        postscript_family_blues,
        postscript_family_other_blues,
        postscript_force_bold,
        postscript_other_blues,
        postscript_stem_snap_h,
        postscript_stem_snap_v
    );
    assert_equivalent(&from_ufo, &from_json);
}

#[test]
fn fontinfotest_v2() {
    check_fixture("fontinfotest_v2.ufo", "fontinfotest_v2");
}

#[test]
fn fontinfotest_v3() {
    check_fixture("fontinfotest_v3.ufo", "fontinfotest_v3");
}

#[test]
fn identifiers() {
    check_fixture("identifiers.ufo", "identifiers");
}

/// ufoLib2 reads `features.fea` in text mode, so its `\r\n` become `\n`; norad
/// keeps the file's bytes. The rest must match.
#[test]
fn line_endings() {
    let mut from_ufo = Font::load("testdata/lineendings/Tester-LineEndings.ufo").unwrap();
    let from_json = load_json("Tester-LineEndings");
    assert!(from_ufo.features.contains("\r\n"));
    assert!(!from_json.features.contains('\r'));
    from_ufo.features = from_ufo.features.replace("\r\n", "\n");
    assert_equivalent(&from_ufo, &from_json);
}

#[test]
fn upconversion_kerning_v1() {
    check_fixture(
        "upconversion_kerning/glyphname_groupname_UFOv1.ufo",
        "glyphname_groupname_UFOv1",
    );
}

#[test]
fn upconversion_kerning_v2() {
    check_fixture(
        "upconversion_kerning/glyphname_groupname_UFOv2.ufo",
        "glyphname_groupname_UFOv2",
    );
}

#[test]
fn kitchen_sink() {
    let from_ufo = Font::load(Path::new(JSON_DIR).join("kitchen_sink.ufo")).unwrap();
    assert_equivalent(&from_ufo, &load_json("kitchen_sink"));
}

/// Loading from bytes must give the same font as the UFO, not just the same as
/// loading from a path.
#[test]
fn from_bytes_matches_ufo() {
    let bytes = std::fs::read(Path::new(JSON_DIR).join("kitchen_sink.json")).unwrap();
    let from_bytes = Font::from_ufolib2_json(&bytes).unwrap();
    let from_ufo = Font::load(Path::new(JSON_DIR).join("kitchen_sink.ufo")).unwrap();
    assert_equivalent(&from_ufo, &from_bytes);
}

#[test]
fn invalid_json_is_an_error() {
    assert!(matches!(Font::from_ufolib2_json(b"not json"), Err(JsonLoadError::Json(_))));
    assert!(matches!(Font::from_ufolib2_json(b"[]"), Err(JsonLoadError::Json(_))));
    let missing = Path::new(JSON_DIR).join("does_not_exist.json");
    assert!(matches!(Font::load_ufolib2_json(missing), Err(JsonLoadError::Io(_))));
}

// Targeted checks on decoded values, so a pass doesn't rely on both loaders
// sharing a bug.

fn data(bytes: &[u8]) -> Value {
    Value::Data(bytes.to_vec())
}

#[test]
fn sink_font_lib_values() {
    let font = load_json("kitchen_sink");
    let lib = &font.lib;
    assert_eq!(lib.get("com.example.data"), Some(&data(&[0, 1, 2, 253, 254, 255])));
    assert_eq!(lib.get("com.example.int"), Some(&Value::from(42)));
    assert_eq!(lib.get("com.example.float"), Some(&Value::from(1.5)));
    assert_eq!(lib.get("com.example.integral-float"), Some(&Value::from(2.0)));
    assert_eq!(lib.get("com.example.bool"), Some(&Value::from(true)));
    assert_eq!(lib.get("com.example.string"), Some(&Value::from("h\u{e9}llo")));
    // data nested in a list in a dict
    let items = lib.get("com.example.datalist").unwrap().as_dictionary().unwrap();
    let items = items.get("items").unwrap().as_array().unwrap();
    assert_eq!(items[0], Value::from(1));
    assert_eq!(items[1], data(b"\x00\xffabc"));
    assert_eq!(items[2].as_dictionary().unwrap().get("inner"), Some(&data(b"xyz")));
    let nested = lib.get("com.example.dict").unwrap().as_dictionary().unwrap();
    let b = nested.get("nested").unwrap().as_dictionary().unwrap().get("b").unwrap();
    assert_eq!(b.as_array().unwrap()[0], Value::from(true));
}

#[test]
fn sink_layers() {
    let font = load_json("kitchen_sink");
    let names: Vec<_> = font.layers.iter().map(|l| l.name().to_string()).collect();
    assert_eq!(names, ["foreground", "background"]);
    assert_eq!(font.default_layer().name().as_str(), "foreground");
    assert!(font.default_layer().color.is_none());

    let bg = font.layers.get("background").unwrap();
    assert_eq!(bg.color, Some(Color::new(0.1, 0.2, 0.3, 0.4).unwrap()));
    let lib = bg.lib.get("com.example.background").unwrap().as_dictionary().unwrap();
    assert_eq!(lib.get("answer"), Some(&Value::from(42)));
    assert_eq!(lib.get("pi"), Some(&Value::from(3.25)));
    assert_eq!(bg.len(), 2);
    assert!(bg.contains_glyph("onlybg"));
}

fn glyph_a(font: &Font) -> &Glyph {
    font.default_layer().get_glyph("A").unwrap()
}

#[test]
fn sink_glyph_basics() {
    let font = load_json("kitchen_sink");
    let a = glyph_a(&font);
    assert_eq!((a.width, a.height), (600.0, 800.0));
    let cps: Vec<_> = a.codepoints.iter().collect();
    assert_eq!(cps, ['A', '\u{c0}', '\u{391}']);
    assert_eq!(a.note.as_deref(), Some("A glyph with everything."));
    assert_eq!(a.lib.get("com.example.data"), Some(&data(&[0, 1, 2, 253, 254, 255])));
}

#[test]
fn sink_points() {
    use norad::PointType;
    let font = load_json("kitchen_sink");
    let a = glyph_a(&font);
    assert_eq!(a.contours.len(), 2);

    let closed = &a.contours[0];
    assert!(closed.is_closed());
    assert_eq!(closed.identifier().map(|i| i.as_str()), Some("c-closed"));
    let types: Vec<_> = closed.points.iter().map(|p| p.typ).collect();
    use PointType::*;
    assert_eq!(types, [Line, Line, OffCurve, OffCurve, Curve, OffCurve, QCurve, Line]);
    assert!(closed.points[1].smooth && closed.points[4].smooth);
    assert!(!closed.points[0].smooth);
    assert_eq!(closed.points[1].name.as_deref(), Some("top-left"));
    assert_eq!(closed.points[4].identifier().map(|i| i.as_str()), Some("p-curve"));

    let open = &a.contours[1];
    assert!(!open.is_closed());
    assert_eq!(open.points[0].typ, Move);
    assert_eq!(open.points[2].typ, QCurve);
    assert_eq!(open.points[2].name.as_deref(), Some("qc"));
    assert_eq!(open.points[0].identifier().map(|i| i.as_str()), Some("p-move"));
    assert_eq!((open.points[2].x, open.points[2].y), (150.0, 60.0));
}

#[test]
fn sink_offcurve_only_contour() {
    use norad::PointType::*;
    let font = load_json("kitchen_sink");
    let glyph = font.default_layer().get_glyph("offcurves").unwrap();
    assert_eq!(glyph.contours.len(), 1);
    let types: Vec<_> = glyph.contours[0].points.iter().map(|p| p.typ).collect();
    assert_eq!(types, [OffCurve, OffCurve, OffCurve]);
}

#[test]
fn sink_object_libs() {
    let font = load_json("kitchen_sink");
    let a = glyph_a(&font);

    let point_lib = a.contours[0].points[0].lib().unwrap();
    assert_eq!(point_lib.get("com.example.point"), Some(&Value::from("on a point")));
    assert!(a.contours[0].points[1].lib().is_none());
    let contour_lib = a.contours[0].lib().unwrap();
    assert_eq!(contour_lib.get("com.example.contour"), Some(&Value::from("on a contour")));

    let anchor_lib = a.anchors[0].lib().unwrap();
    assert_eq!(anchor_lib.get("com.example.anchor").unwrap().as_array().unwrap().len(), 3);
    assert!(a.anchors[1].lib().is_none());

    assert_eq!(a.components[1].lib().unwrap().get("com.example.component"), Some(&Value::from(7)));
    assert!(a.components[0].lib().is_none());

    let g = a.guidelines[2].lib().unwrap();
    let g = g.get("com.example.guideline").unwrap().as_dictionary().unwrap();
    assert_eq!(g.get("k"), Some(&Value::from(true)));
    assert!(a.guidelines[0].lib().is_none());

    // The font-level object lib ends up on the matching font guideline.
    let fg = &font.guidelines()[0];
    assert_eq!(fg.identifier().map(|i| i.as_str()), Some("fg-x"));
    assert_eq!(fg.lib().unwrap().get("com.example.fontguide"), Some(&Value::from("font-level")));
    assert!(font.guidelines()[1].lib().is_none());
}

#[test]
fn sink_components_anchors_guidelines_image() {
    use norad::Line;
    let font = load_json("kitchen_sink");
    let a = glyph_a(&font);

    assert_eq!(a.components.len(), 2);
    assert_eq!(a.components[0].base.as_str(), "B");
    let identity = norad::AffineTransform::default();
    assert_eq!(a.components[0].transform, identity);
    let t = a.components[1].transform;
    assert_eq!(
        [t.x_scale, t.xy_scale, t.yx_scale, t.y_scale, t.x_offset, t.y_offset],
        [0.5, 0.1, -0.1, 0.75, 20.0, 30.0]
    );
    assert_eq!(a.components[1].identifier().map(|i| i.as_str()), Some("comp-xf"));

    assert_eq!(a.anchors.len(), 3);
    assert_eq!(a.anchors[0].name.as_deref(), Some("top"));
    assert_eq!(a.anchors[0].color, Some(Color::new(1.0, 0.0, 0.0, 0.5).unwrap()));
    assert_eq!(a.anchors[0].identifier().map(|i| i.as_str()), Some("a-top"));
    assert_eq!((a.anchors[2].x, a.anchors[2].y), (1.5, 2.5));
    assert!(a.anchors[2].name.is_none() && a.anchors[2].color.is_none());

    let lines: Vec<_> = a.guidelines.iter().map(|g| g.line).collect();
    assert_eq!(
        lines,
        [
            Line::Vertical(250.0),
            Line::Horizontal(-20.0),
            Line::Angle { x: 10.0, y: 20.0, degrees: 45.5 }
        ]
    );
    assert_eq!(a.guidelines[0].name.as_deref(), Some("vertical"));

    let image = a.image.as_ref().unwrap();
    assert_eq!(image.file_name(), Path::new("image1.png"));
    assert_eq!(image.color, Some(Color::new(0.5, 0.5, 0.5, 1.0).unwrap()));
    assert_eq!(image.transform.x_scale, 0.5);
    assert_eq!((image.transform.x_offset, image.transform.y_offset), (10.0, 20.0));
}

#[test]
fn sink_font_level_data() {
    let font = load_json("kitchen_sink");
    assert_eq!(font.features, "feature liga {\n    sub A B by space;\n} liga;\n");
    let bar = font.data.get(Path::new("com.example.foo/bar.txt")).unwrap().unwrap();
    assert_eq!(&*bar, b"hello\nworld\n");
    let top = font.data.get(Path::new("top.bin")).unwrap().unwrap();
    assert_eq!(&*top, (0..=255u8).collect::<Vec<_>>().as_slice());
    let png = font.images.get(Path::new("image1.png")).unwrap().unwrap();
    assert_eq!(&*png, std::fs::read("testdata/dataimagetest.ufo/images/image1.png").unwrap());

    assert_eq!(font.kerning["public.kern1.A"]["public.kern2.B"], -30.0);
    assert_eq!(font.kerning["A"]["B"], 10.5);
    assert_eq!(font.groups["public.kern2.round"], ["space"]);
}

#[test]
fn sink_font_info() {
    let info = load_json("kitchen_sink").font_info;
    assert_eq!(info.units_per_em.map(|v| v.as_f64()), Some(1000.0));
    assert_eq!(info.ascender, Some(750.5));
    assert_eq!(info.open_type_head_created.as_deref(), Some("2024/01/02 03:04:05"));
    assert_eq!(info.open_type_gasp_range_records.as_ref().unwrap().len(), 2);
    assert_eq!(info.open_type_name_records.as_ref().unwrap().len(), 2);
    let ext = info.woff_metadata_extensions.as_ref().unwrap();
    assert_eq!(ext.len(), 2);
    assert_eq!(ext[0].id, None);
    assert_eq!(ext[1].id.as_deref(), Some("ext2"));
    assert_eq!(ext[0].items[0].id.as_deref(), Some("item1"));
    assert_eq!(ext[0].items[1].id, None);
}
