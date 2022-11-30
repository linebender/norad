//! Testing saving files.

use indexmap::indexset;
use norad::{Font, FormatVersion, Glyph, Identifier, Plist};
use plist::Value;

#[test]
fn save_default() {
    let my_ufo = Font::new();

    let dir = tempdir::TempDir::new("Test.ufo").unwrap();
    my_ufo.save(&dir).unwrap();

    assert!(dir.path().join("metainfo.plist").exists());
    assert!(dir.path().join("glyphs").exists());
    assert!(dir.path().join("glyphs/contents.plist").exists());

    let loaded = Font::load(dir).unwrap();
    assert!(loaded.meta.format_version == FormatVersion::V3);
    assert!(loaded.meta.creator == Some("org.linebender.norad".into()));
    assert_eq!(loaded.layers.len(), 1);
}

#[test]
fn save_new_file() {
    let mut my_ufo = Font::new();
    let mut my_glyph = Glyph::new("A");
    my_glyph.codepoints = indexset!['A'];
    my_glyph.note = Some("I did a glyph!".into());
    let mut plist = Plist::new();
    plist.insert("my-cool-key".into(), plist::Value::Integer(420_u32.into()));
    my_glyph.lib = plist;
    my_ufo.default_layer_mut().insert_glyph(my_glyph);

    let dir = tempdir::TempDir::new("Test.ufo").unwrap();
    my_ufo.save(&dir).unwrap();

    assert!(dir.path().join("metainfo.plist").exists());
    assert!(dir.path().join("glyphs").exists());
    assert!(dir.path().join("glyphs/contents.plist").exists());
    assert!(dir.path().join("glyphs/A_.glif").exists());

    let loaded = Font::load(dir).unwrap();
    assert!(loaded.default_layer().get_glyph("A").is_some());
    let glyph = loaded.default_layer().get_glyph("A").unwrap();
    assert_eq!(glyph.codepoints, indexset!['A']);
    let lib_val = glyph.lib.get("my-cool-key").and_then(|val| val.as_unsigned_integer());
    assert_eq!(lib_val, Some(420));
}

#[test]
fn save_fancy() {
    let mut my_ufo = Font::new();
    let other_ufo = Font::load("testdata/MutatorSansLightWide.ufo").unwrap();
    *my_ufo.default_layer_mut() = other_ufo.layers.get("foreground").unwrap().clone();

    let dir = tempdir::TempDir::new("Fancy.ufo").unwrap();
    my_ufo.save(&dir).unwrap();

    let loaded = Font::load(dir).unwrap();
    let pre_layer = my_ufo.default_layer();
    let post_layer = loaded.default_layer();
    assert_eq!(pre_layer.iter().count(), post_layer.iter().count());

    for glyph in pre_layer.iter() {
        let other = post_layer.get_glyph(glyph.name());
        assert!(other.is_some(), "missing {}", glyph.name());
        assert_eq!(glyph, other.unwrap());
    }
}

#[test]
fn roundtrip_object_libs() {
    let ufo = Font::load("testdata/identifiers.ufo").unwrap();
    assert!(!ufo.lib.contains_key("public.objectLibs"));

    let glyph = ufo.get_glyph("test").unwrap();
    assert!(!glyph.lib.contains_key("public.objectLibs"));

    let dir = tempdir::TempDir::new("identifiers.ufo").unwrap();
    ufo.save(&dir).unwrap();
    assert!(!glyph.lib.contains_key("public.objectLibs"));

    let ufo2 = Font::load(&dir).unwrap();
    assert!(!ufo2.lib.contains_key("public.objectLibs"));

    let font_guideline_second = &ufo2.font_info.guidelines.as_ref().unwrap()[1];
    assert_eq!(
        font_guideline_second.identifier(),
        Some(&Identifier::new("3f0f37d1-52d6-429c-aff4-3f81aed4abf0").unwrap())
    );
    assert_eq!(
        font_guideline_second
            .lib()
            .as_ref()
            .unwrap()
            .get("com.test.foo")
            .unwrap()
            .as_unsigned_integer()
            .unwrap(),
        1234
    );

    let glyph2 = ufo2.get_glyph("test").unwrap();
    assert!(!glyph2.lib.contains_key("public.objectLibs"));

    let anchor_second = &glyph2.anchors[1];
    assert_eq!(
        anchor_second.identifier(),
        Some(&Identifier::new("90b7eb80-e21a-4a79-a8c0-7634c25ddc18").unwrap())
    );
    assert_eq!(
        anchor_second
            .lib()
            .as_ref()
            .and_then(|l| l.get("com.test.anchorTool").and_then(Value::as_boolean)),
        Some(true)
    );

    assert_eq!(
        glyph2.guidelines[1].identifier(),
        Some(&Identifier::new("c76955c2-e9f2-4adf-8b51-1ae03da11dca").unwrap())
    );
    assert_eq!(
        glyph2.guidelines[1]
            .lib()
            .as_ref()
            .unwrap()
            .get("com.test.foo")
            .unwrap()
            .as_unsigned_integer()
            .unwrap(),
        4321
    );

    assert_eq!(
        glyph.contours[0].identifier(),
        Some(&Identifier::new("9bf0591d-6281-4c76-8c13-9ff3d93eec4f").unwrap())
    );
    assert_eq!(
        glyph.contours[0].lib().as_ref().unwrap().get("com.test.foo").unwrap().as_string().unwrap(),
        "a"
    );

    assert_eq!(
        glyph.contours[1].points[0].identifier(),
        Some(&Identifier::new("f32ac0e8-4ec8-45f6-88b1-0e49390b8f5b").unwrap())
    );
    assert_eq!(
        glyph.contours[1].points[0]
            .lib()
            .as_ref()
            .unwrap()
            .get("com.test.foo")
            .unwrap()
            .as_string()
            .unwrap(),
        "c"
    );
    assert_eq!(
        glyph.contours[1].points[2].identifier(),
        Some(&Identifier::new("spare-id").unwrap())
    );
    assert!(glyph.contours[1].points[2].lib().is_none());

    assert_eq!(
        glyph.components[0].identifier(),
        Some(&Identifier::new("a50e8ccd-2ba4-4279-a011-4c82a8075dd9").unwrap())
    );
    assert_eq!(
        glyph.components[0]
            .lib()
            .as_ref()
            .unwrap()
            .get("com.test.foo")
            .unwrap()
            .as_string()
            .unwrap(),
        "b"
    );
}

#[test]
fn object_libs_reject_existing_key() {
    let dir = tempdir::TempDir::new("test.ufo").unwrap();
    let mut ufo = Font::new();

    let mut test_lib = plist::Dictionary::new();
    test_lib.insert("public.objectLibs".into(), plist::Value::Dictionary(plist::Dictionary::new()));

    ufo.lib = test_lib.clone();
    assert!(ufo.save(&dir).is_err());
    ufo.lib.remove("public.objectLibs");

    let mut glyph = Glyph::new("test");
    glyph.lib = test_lib;

    ufo.default_layer_mut().insert_glyph(glyph);
    assert!(ufo.save(&dir).is_err());
}
