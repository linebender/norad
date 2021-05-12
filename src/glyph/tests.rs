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
        vec![
            Anchor::new(10.0, 10.0, Some("top".into()), None, None, None),
            Anchor::new(10.0, 20.0, Some("bottom".into()), None, None, None),
            Anchor::new(30.0, 20.0, Some("left".into()), None, None, None),
            Anchor::new(40.0, 20.0, Some("right".into()), None, None, None),
        ]
    );
    assert_eq!(glyph.format, GlifVersion::V2);
    assert_eq!(glyph.guidelines, vec![]);
    assert_eq!(glyph.image, None);
    assert_eq!(glyph.lib, Plist::new());
    assert_eq!(glyph.note, None);
}

#[test]
fn curve_types() {
    let bytes = include_bytes!("../../testdata/mutatorSans/MutatorSansBoldWide.ufo/glyphs/D_.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(glyph.contours.len(), 2);
    assert_eq!(glyph.contours[1].points[0].typ, PointType::Line);
    assert_eq!(glyph.contours[1].points[0].smooth, false);
    assert_eq!(glyph.contours[1].points[1].smooth, true);
    assert_eq!(glyph.contours[1].points[2].typ, PointType::OffCurve);
    assert_eq!(glyph.contours[1].points[4].typ, PointType::Curve);
}

#[test]
fn guidelines() {
    let bytes = include_bytes!("../../testdata/Blinker_one.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(glyph.guidelines.len(), 8);
    assert_eq!(glyph.contours.len(), 2);
    assert_eq!(glyph.width, 364.);
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
    assert_eq!(glyph.height, glyph2.height);
    assert_eq!(glyph.width, glyph2.width);
    assert_eq!(glyph.codepoints, glyph2.codepoints);
    assert_eq!(glyph.note, glyph2.note);
    assert_eq!(glyph.components, glyph2.components);
    assert_eq!(glyph.contours, glyph2.contours);
    assert_eq!(glyph.image, glyph2.image);
    assert_eq!(glyph.anchors, glyph2.anchors);
    assert_eq!(glyph.guidelines, glyph2.guidelines);
}

// https://github.com/linebender/norad/issues/105
#[test]
fn skip_zero_advance() {
    let glyph = Glyph::new_named("A");
    let encoded = glyph.encode_xml().unwrap();
    let as_str = String::from_utf8(encoded).expect("xml is valid utf-8");
    assert!(!as_str.contains("advance"));

    let mut glyph = Glyph::new_named("B");
    glyph.width = 500.0;

    let encoded = glyph.encode_xml().unwrap();
    let as_str = String::from_utf8(encoded).expect("xml is valid utf-8");
    assert!(as_str.contains("advance"));
}

// https://github.com/linebender/norad/issues/105
#[test]
fn skip_empty_outline() {
    let glyph = Glyph::new_named("A");
    let encoded = glyph.encode_xml().unwrap();
    let as_str = String::from_utf8(encoded).expect("xml is valid utf-8");
    assert!(!as_str.contains("outline"));

    let mut glyph = Glyph::new_named("B");
    glyph.components = vec![Component::new("A".into(), AffineTransform::default(), None, None)];

    let encoded = glyph.encode_xml().unwrap();
    let as_str = String::from_utf8(encoded).expect("xml is valid utf-8");
    assert!(as_str.contains("outline"));
}

#[test]
fn notdef_failure() {
    let bytes = include_bytes!("../../testdata/noto-cjk-notdef.glif");
    let _ = parse_glyph(bytes).unwrap();
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

#[test]
#[should_panic(expected = "UnexpectedMove")]
fn unexpected_move() {
    let data = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="period" format="2">
            <advance width="268"/>
            <unicode hex="002E"/>
            <outline>
                <contour>
                    <point x="237" y="152"/>
                    <point x="193" y="187" type="move"/>
                </contour>
            </outline>
        </glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedSmooth")]
fn unexpected_smooth() {
    let data = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="period" format="2">
            <advance width="268"/>
            <unicode hex="002E"/>
            <outline>
                    <contour>
                        <point x="193" y="187" smooth="yes"/>
                    </contour>
            </outline>
        </glyph>
  "#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
fn zero_to_two_offcurves_before_curve() {
    let data1 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0" type="line"/>
                    <point x="100" y="100" type="curve"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let data2 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0" type="line"/>
                    <point x="50" y="50"/>
                    <point x="100" y="100" type="curve"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let data3 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0" type="line"/>
                    <point x="33" y="33"/>
                    <point x="66" y="66"/>
                    <point x="100" y="100" type="curve"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let data4 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="100" y="100" type="curve"/>
                    <point x="0" y="0" type="line"/>
                    <point x="33" y="33"/>
                    <point x="66" y="66"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let _ = parse_glyph(data1.as_bytes()).unwrap();
    let _ = parse_glyph(data2.as_bytes()).unwrap();
    let _ = parse_glyph(data3.as_bytes()).unwrap();
    let _ = parse_glyph(data4.as_bytes()).unwrap();
}

#[test]
fn valid_offcurves() {
    let data1 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let data2 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0"/>
                    <point x="100" y="100"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let data3 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0"/>
                    <point x="50" y="25"/>
                    <point x="100" y="100"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let data4 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0"/>
                    <point x="50" y="25" type="curve"/>
                    <point x="100" y="100"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let _ = parse_glyph(data1.as_bytes()).unwrap();
    let _ = parse_glyph(data2.as_bytes()).unwrap();
    let _ = parse_glyph(data3.as_bytes()).unwrap();
    let _ = parse_glyph(data4.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "TrailingOffCurves")]
fn trailing_off_curves() {
    let data = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0" type="move"/>
                    <point x="50" y="25"/>
                    <point x="100" y="100"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "TooManyOffCurves")]
fn too_many_off_curves() {
    let data = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="0" y="0" type="line"/>
                    <point x="33" y="33"/>
                    <point x="66" y="66"/>
                    <point x="77" y="77"/>
                    <point x="100" y="100" type="curve"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedPointAfterOffCurve")]
fn unexpected_line_after_offcurve1() {
    let data = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="1" y="1" type="line"/>
                    <point x="33" y="33"/>
                    <point x="0" y="0" type="line"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedPointAfterOffCurve")]
fn unexpected_line_after_offcurve2() {
    let data = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="301" y="714" type="line" smooth="yes"/>
                    <point x="572" y="537" type="curve" smooth="yes"/>
                    <point x="572" y="667"/>
                    <point x="479" y="714"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "UnexpectedPointAfterOffCurve")]
fn unexpected_line_after_offcurve3() {
    let data = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                    <point x="479" y="714"/>
                    <point x="301" y="714" type="line" smooth="yes"/>
                    <point x="572" y="537" type="curve" smooth="yes"/>
                    <point x="572" y="667"/>
                </contour>
            </outline>
        </glyph>
    "#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
fn empty_outlines() {
    let data1 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
            </outline>
        </glyph>
        "#;
    let data2 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline/>
        </glyph>
        "#;
    let test1 = parse_glyph(data1.as_bytes()).unwrap();
    assert_eq!(test1.components, vec![]);
    assert_eq!(test1.contours, vec![]);
    let test2 = parse_glyph(data2.as_bytes()).unwrap();
    assert_eq!(test2.components, vec![]);
    assert_eq!(test2.contours, vec![]);
}

#[test]
fn empty_contours() {
    let data1 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour>
                </contour>
                <contour identifier="aaa">
                </contour>
            </outline>
        </glyph>
        "#;
    let data2 = r#"
        <?xml version="1.0" encoding="UTF-8"?>
        <glyph name="test" format="2">
            <outline>
                <contour/>
                <contour identifier="bbb"/>
            </outline>
        </glyph>
        "#;
    let test1 = parse_glyph(data1.as_bytes()).unwrap();
    assert_eq!(test1.components, vec![]);
    assert_eq!(test1.contours, vec![]);
    let test2 = parse_glyph(data2.as_bytes()).unwrap();
    assert_eq!(test2.components, vec![]);
    assert_eq!(test2.contours, vec![]);
}
