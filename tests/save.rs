//! Testing saving files.

use norad::{FormatVersion, Glyph, Identifier, Layer, Plist, Ufo};

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

#[test]
fn roundtrip_object_libs() {
    let ufo = Ufo::load("testdata/identifiers.ufo").unwrap();
    assert_eq!(ufo.lib.as_ref().unwrap().contains_key("public.objectLibs"), false);

    let glyph = ufo.get_glyph("test").unwrap();
    assert_eq!(glyph.lib.as_ref().unwrap().contains_key("public.objectLibs"), false);

    let dir = tempdir::TempDir::new("identifiers.ufo").unwrap();
    ufo.save(&dir).unwrap();
    assert_eq!(glyph.lib.as_ref().unwrap().contains_key("public.objectLibs"), false);

    let ufo2 = Ufo::load(&dir).unwrap();
    assert_eq!(ufo2.lib.as_ref().unwrap().contains_key("public.objectLibs"), false);

    let font_guideline_second = &ufo2.font_info.as_ref().unwrap().guidelines.as_ref().unwrap()[1];
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
    assert_eq!(glyph2.lib.as_ref().unwrap().contains_key("public.objectLibs"), false);

    let anchor_second = &glyph2.anchors.as_ref().unwrap()[1];
    assert_eq!(
        anchor_second.identifier(),
        Some(&Identifier::new("90b7eb80-e21a-4a79-a8c0-7634c25ddc18").unwrap())
    );
    assert_eq!(
        anchor_second
            .lib()
            .as_ref()
            .unwrap()
            .get("com.test.anchorTool")
            .unwrap()
            .as_boolean()
            .unwrap(),
        true
    );

    let guideline_second = &glyph2.guidelines.as_ref().unwrap()[1];
    assert_eq!(
        guideline_second.identifier(),
        Some(&Identifier::new("c76955c2-e9f2-4adf-8b51-1ae03da11dca").unwrap())
    );
    assert_eq!(
        guideline_second
            .lib()
            .as_ref()
            .unwrap()
            .get("com.test.foo")
            .unwrap()
            .as_unsigned_integer()
            .unwrap(),
        4321
    );

    let outline2 = glyph2.outline.as_ref().unwrap();
    assert_eq!(
        outline2.contours[0].identifier(),
        Some(&Identifier::new("9bf0591d-6281-4c76-8c13-9ff3d93eec4f").unwrap())
    );
    assert_eq!(
        outline2.contours[0]
            .lib()
            .as_ref()
            .unwrap()
            .get("com.test.foo")
            .unwrap()
            .as_string()
            .unwrap(),
        "a"
    );

    assert_eq!(
        outline2.contours[1].points[0].identifier(),
        Some(&Identifier::new("f32ac0e8-4ec8-45f6-88b1-0e49390b8f5b").unwrap())
    );
    assert_eq!(
        outline2.contours[1].points[0]
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
        outline2.contours[1].points[2].identifier(),
        Some(&Identifier::new("spare-id").unwrap())
    );
    assert!(outline2.contours[1].points[2].lib().is_none());

    assert_eq!(
        outline2.components[0].identifier(),
        Some(&Identifier::new("a50e8ccd-2ba4-4279-a011-4c82a8075dd9").unwrap())
    );
    assert_eq!(
        outline2.components[0]
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
    let mut ufo = Ufo::new();

    let mut test_lib = plist::Dictionary::new();
    test_lib.insert("public.objectLibs".into(), plist::Value::Dictionary(plist::Dictionary::new()));

    ufo.lib.replace(test_lib.clone());
    assert!(ufo.save(&dir).is_err());
    ufo.lib.as_mut().unwrap().remove("public.objectLibs".into());

    let glyph = Glyph {
        name: "test".into(),
        format: norad::GlifVersion::V2,
        advance: None,
        anchors: None,
        codepoints: None,
        guidelines: None,
        image: None,
        lib: Some(test_lib),
        note: None,
        outline: None,
    };
    ufo.get_default_layer_mut().unwrap().insert_glyph(glyph);
    assert!(ufo.save(&dir).is_err());
}
