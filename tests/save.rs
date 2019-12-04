//! Testing saving files.

use norad::{Glyph, Layer, MetaInfo, Ufo};

#[test]
fn save_new_file() {
    let mut my_ufo = Ufo::new(MetaInfo::default());
    let mut my_glyph = Glyph::new_named("A");
    my_glyph.codepoints = Some(vec!['A']);
    my_glyph.note = Some("I did a glyph!".into());
    my_ufo.get_default_layer_mut().unwrap().insert_glyph(my_glyph);

    let dir = tempdir::TempDir::new("Test.ufo").unwrap();
    my_ufo.save(&dir).unwrap();

    assert!(dir.path().join("glyphs").exists());
    assert!(dir.path().join("glyphs/contents.plist").exists());
    assert!(dir.path().join("glyphs/A_.glif").exists());

    let loaded = Ufo::load(dir).unwrap();
    assert!(loaded.get_default_layer().unwrap().get_glyph("A").is_some());
    let glyph = loaded.get_default_layer().unwrap().get_glyph("A").unwrap();
    assert_eq!(glyph.codepoints.as_ref(), Some(&vec!['A']));
}

#[test]
fn save_fancy() {
    let mut my_ufo = Ufo::new(MetaInfo::default());
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
