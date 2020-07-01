use super::parse::parse_glyph;
use super::*;
use std::path::PathBuf;

#[test]
fn transform() {
    let transform = AffineTransform::default();
    assert_eq!(transform.x_scale, 1.0);
}

#[test]
fn parse() {
    let bytes = include_bytes!("../../testdata/sample_period.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(&*glyph.name, "period");
    assert_eq!(
        glyph.image.as_ref().map(|img| img.file_name.clone()),
        Some(PathBuf::from("period sketch.png"))
    );
}

#[test]
fn parse_v1_upgrade_anchors() {
    let bytes = include_bytes!("../../testdata/glifv1.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(
        glyph.anchors,
        Some(vec![
            Anchor { name: Some("top".into()), x: 10.0, y: 10.0, color: None, identifier: None },
            Anchor { name: Some("bottom".into()), x: 10.0, y: 20.0, color: None, identifier: None },
            Anchor { name: Some("left".into()), x: 30.0, y: 20.0, color: None, identifier: None },
            Anchor { name: Some("right".into()), x: 40.0, y: 20.0, color: None, identifier: None }
        ])
    );
    assert_eq!(glyph.format, GlifVersion::V2);
    assert_eq!(glyph.guidelines, None);
    assert_eq!(glyph.image, None);
    assert_eq!(glyph.lib, None);
    assert_eq!(glyph.note, None);
}

#[test]
fn curve_types() {
    let bytes = include_bytes!("../../testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs/D_.glif");
    let glyph = parse_glyph(bytes).unwrap();
    let outline = glyph.outline.as_ref().unwrap();
    assert_eq!(outline.contours.len(), 2);
    assert_eq!(outline.contours[1].points[0].typ, PointType::Line);
    assert_eq!(outline.contours[1].points[0].smooth, false);
    assert_eq!(outline.contours[1].points[1].smooth, true);
    assert_eq!(outline.contours[1].points[2].typ, PointType::OffCurve);
    assert_eq!(outline.contours[1].points[4].typ, PointType::Curve);
}

#[test]
fn guidelines() {
    let bytes = include_bytes!("../../testdata/Blinker_one.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(glyph.guidelines.as_ref().map(Vec::len), Some(8));
    assert_eq!(glyph.outline.as_ref().map(|o| o.contours.len()), Some(2));
    assert_eq!(glyph.advance, Some(Advance { width: 364., height: 0. }));
}

#[test]
#[should_panic(expected = "MissingClose")]
fn missing_close() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
  <advance width="268"/>
  <unicode hex="002E"/>
  <outline>
    <contour>
      <point x="237" y="152"/>
      <point x="193" y="187"/>
    </contour>
  </outline>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
fn parse_note() {
    let bytes = include_bytes!("../../testdata/note.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(glyph.note, Some(".notdef".to_string()));
}

#[test]
fn save() {
    let bytes = include_bytes!("../../testdata/sample_period.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    let buf = glyph.encode_xml().expect("encode failed");

    println!("{}", String::from_utf8_lossy(&buf));
    //panic!("ahh");

    let glyph2 = parse_glyph(buf.as_slice()).expect("re-parse failed");
    assert_eq!(glyph.name, glyph2.name);
    assert_eq!(glyph.format, glyph2.format);
    assert_eq!(glyph.advance, glyph2.advance);
    assert_eq!(glyph.codepoints, glyph2.codepoints);
    assert_eq!(glyph.note, glyph2.note);
    assert_eq!(glyph.outline, glyph2.outline);
    assert_eq!(glyph.image, glyph2.image);
    assert_eq!(glyph.anchors, glyph2.anchors);
    assert_eq!(glyph.guidelines, glyph2.guidelines);
}

#[test]
fn notdef_failure() {
    let bytes = include_bytes!("../../testdata/noto-cjk-notdef.glif");
    let _ = parse_glyph(bytes).unwrap();
}

#[test]
fn path_for_name() {
    fn trimmed_name(name: &str) -> String {
        default_file_name_for_glyph_name(name).trim_end_matches(".glif").into()
    }

    assert_eq!(trimmed_name("newGlyph.1"), "newG_lyph.1".to_string());
    assert_eq!(trimmed_name("a"), "a".to_string());
    assert_eq!(trimmed_name("A"), "A_".to_string());
    assert_eq!(trimmed_name("AE"), "A_E_".to_string());
    assert_eq!(trimmed_name("Ae"), "A_e".to_string());
    assert_eq!(trimmed_name("ae"), "ae".to_string());
    assert_eq!(trimmed_name("aE"), "aE_".to_string());
    assert_eq!(trimmed_name("a.alt"), "a.alt".to_string());
    assert_eq!(trimmed_name("A.alt"), "A_.alt".to_string());
    assert_eq!(trimmed_name("A.Alt"), "A_.A_lt".to_string());
    assert_eq!(trimmed_name("A.aLt"), "A_.aL_t".to_string());
    assert_eq!(trimmed_name("A.alT"), "A_.alT_".to_string());
    assert_eq!(trimmed_name("T_H"), "T__H_".to_string());
    assert_eq!(trimmed_name("T_h"), "T__h".to_string());
    assert_eq!(trimmed_name("t_h"), "t_h".to_string());
    assert_eq!(trimmed_name("F_F_I"), "F__F__I_".to_string());
    assert_eq!(trimmed_name("f_f_i"), "f_f_i".to_string());
    assert_eq!(trimmed_name("Aacute_V.swash"), "A_acute_V_.swash".to_string());
    assert_eq!(trimmed_name(".notdef"), "_notdef".to_string());

    //FIXME: we're ignoring 'reserved filenames' for now
    //assert_eq!(trimmed_name("con"), "_con".to_string());
    //assert_eq!(trimmed_name("CON"), "C_O_N_".to_string());
    //assert_eq!(trimmed_name("con.alt"), "_con.alt".to_string());
    //assert_eq!(trimmed_name("alt.con"), "alt._con".to_string());
}

#[cfg(feature = "druid")]
#[test]
fn druid_from_color() {
    let color = druid::piet::Color::rgba(1.0, 0.11, 0.5, 0.23);
    let color2: druid::piet::Color = Color { red: 1.0, green: 0.11, blue: 0.5, alpha: 0.23 }.into();
    assert_eq!(color2.as_rgba_u32(), color.as_rgba_u32());
}

//#[test]
//fn parse_utf16() {
//let bytes = include_bytes!("../../testdata/utf16-glyph.xml");
//let glyph = parse_glyph(bytes).unwrap();
//assert_eq!(glyph.width, Some(268.));
//}
