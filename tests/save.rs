//! Testing saving files.

use norad::{FormatVersion, Glyph, Layer, Plist, Ufo};

#[test]
fn save_default() {
    let my_ufo = Ufo::new();

    let dir = tempdir::TempDir::new("Test.ufo").unwrap();
    my_ufo.save(&dir).unwrap();

    assert!(dir.path().join("metainfo.plist").exists());
    assert!(dir.path().join("glyphs").exists());
    assert!(dir.path().join("glyphs/contents.plist").exists());

    let loaded = Ufo::load(dir).unwrap();
    assert!(loaded.meta.format_version == FormatVersion::V3);
    assert!(loaded.meta.creator == "org.linebender.norad");
    assert!(loaded.get_default_layer().is_some());
}

#[test]
fn save_new_file() {
    let mut my_ufo = Ufo::new();
    let mut my_glyph = Glyph::new_named("A");
    my_glyph.codepoints = Some(vec!['A']);
    my_glyph.note = Some("I did a glyph!".into());
    let mut plist = Plist::new();
    plist.insert("my-cool-key".into(), plist::Value::Integer(420_u32.into()));
    my_glyph.lib = Some(plist);
    my_ufo.get_default_layer_mut().unwrap().insert_glyph(my_glyph);

    let dir = tempdir::TempDir::new("Test.ufo").unwrap();
    my_ufo.save(&dir).unwrap();

    assert!(dir.path().join("metainfo.plist").exists());
    assert!(dir.path().join("glyphs").exists());
    assert!(dir.path().join("glyphs/contents.plist").exists());
    assert!(dir.path().join("glyphs/A_.glif").exists());

    let loaded = Ufo::load(dir).unwrap();
    assert!(loaded.get_default_layer().unwrap().get_glyph("A").is_some());
    let glyph = loaded.get_default_layer().unwrap().get_glyph("A").unwrap();
    assert_eq!(glyph.codepoints.as_ref(), Some(&vec!['A']));
    let lib_val = glyph
        .lib
        .as_ref()
        .and_then(|lib| lib.get("my-cool-key").and_then(|val| val.as_unsigned_integer()));
    assert_eq!(lib_val, Some(420));
}

#[test]
fn save_fancy() {
    let mut my_ufo = Ufo::new();
    let layer_path = "testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs";
    let layer = Layer::load(layer_path).unwrap();
    *my_ufo.get_default_layer_mut().unwrap() = layer;

    let dir = tempdir::TempDir::new("Fancy.ufo").unwrap();
    my_ufo.save(&dir).unwrap();

    let loaded = Ufo::load(dir).unwrap();
    let pre_layer = my_ufo.get_default_layer().unwrap();
    let post_layer = loaded.get_default_layer().unwrap();
    assert_eq!(pre_layer.iter_contents().count(), post_layer.iter_contents().count());

    for glyph in pre_layer.iter_contents() {
        let other = post_layer.get_glyph(&glyph.name);
        assert!(other.is_some(), "missing {}", &glyph.name);
        assert_eq!(&glyph, other.unwrap());
    }
}
