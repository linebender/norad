use super::parse::parse_glyph;
use super::*;
use crate::write::QuoteChar;
use std::path::PathBuf;
use std::str::FromStr;

#[test]
#[allow(clippy::float_cmp)]
fn transform() {
    let transform = AffineTransform::default();
    assert_eq!(transform.x_scale, 1.0);
}

#[test]
fn serialize_empty_glyph() {
    let glyph = Glyph::new("a");
    let glif = glyph.encode_xml().unwrap();
    let glif = std::str::from_utf8(&glif).unwrap();
    assert_eq!(
        glif,
        r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="a" format="2">
</glyph>
"#
        .trim_start()
    );
}

#[test]
fn parse_format_minor() {
    let data = r#"
 <?xml version="1.0" encoding="UTF-8"?>
 <glyph name="a" format="2" formatMinor="0">
 </glyph>
     "#
    .trim();
    // if this doesn't panic life is okay
    parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "UnsupportedGlifVersion")]
fn parse_format_unsupported_major() {
    let data = r#"
 <?xml version="1.0" encoding="UTF-8"?>
 <glyph name="a" format="3">
 </glyph>
     "#
    .trim();
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
fn serialize_empty_glyph_explicit_line_ending_check() {
    let glyph = Glyph::new("a");
    let glif = glyph.encode_xml().unwrap();
    let glif = std::str::from_utf8(&glif).unwrap();
    assert_eq!(
        glif,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<glyph name=\"a\" format=\"2\">\n</glyph>\n"
    );
}

#[test]
fn serialize_full_glyph() {
    let source = include_str!("../../testdata/sample_period_normalized.glif");
    let glyph = parse_glyph(source.as_bytes()).unwrap();
    let glif = glyph.encode_xml().unwrap();
    let glif = String::from_utf8(glif).expect("xml is always valid UTF-8");
    pretty_assertions::assert_eq!(glif, source);
}

#[test]
fn serialize_with_default_formatting() {
    let data = include_str!("../../testdata/small_lib.glif");
    let glyph = parse_glyph(data.as_bytes()).unwrap();
    let one_tab = glyph.encode_xml().unwrap();
    let one_tab = std::str::from_utf8(&one_tab).unwrap();
    pretty_assertions::assert_eq!(
        one_tab,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<glyph name="hello" format="2">
	<advance width="1200"/>
	<outline>
		<contour>
			<point x="2" y="30" type="line"/>
			<point x="44" y="10" type="line"/>
		</contour>
	</outline>
	<lib>
		<dict>
			<key>test.key</key>
			<string>I am a creative professional :)</string>
		</dict>
	</lib>
	<note>durp</note>
</glyph>
"#
    );
}

#[test]
fn serialize_with_custom_whitespace() {
    let data = include_str!("../../testdata/small_lib.glif");
    let glyph = parse_glyph(data.as_bytes()).unwrap();
    let options = WriteOptions::default().indent(WriteOptions::SPACE, 2);
    let two_spaces = glyph.encode_xml_with_options(&options).unwrap();
    let two_spaces = std::str::from_utf8(&two_spaces).unwrap();

    pretty_assertions::assert_eq!(
        two_spaces,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<glyph name="hello" format="2">
  <advance width="1200"/>
  <outline>
    <contour>
      <point x="2" y="30" type="line"/>
      <point x="44" y="10" type="line"/>
    </contour>
  </outline>
  <lib>
    <dict>
      <key>test.key</key>
      <string>I am a creative professional :)</string>
    </dict>
  </lib>
  <note>durp</note>
</glyph>
"#
    );
}

#[test]
fn serialize_with_single_quote_style() {
    let data = include_str!("../../testdata/small_lib.glif");
    let glyph = parse_glyph(data.as_bytes()).unwrap();
    let options = WriteOptions::default().quote_char(QuoteChar::Single);
    let one_tab = glyph.encode_xml_with_options(&options).unwrap();
    let one_tab = std::str::from_utf8(&one_tab).unwrap();
    pretty_assertions::assert_eq!(
        one_tab,
        r#"<?xml version='1.0' encoding='UTF-8'?>
<glyph name="hello" format="2">
	<advance width="1200"/>
	<outline>
		<contour>
			<point x="2" y="30" type="line"/>
			<point x="44" y="10" type="line"/>
		</contour>
	</outline>
	<lib>
		<dict>
			<key>test.key</key>
			<string>I am a creative professional :)</string>
		</dict>
	</lib>
	<note>durp</note>
</glyph>
"#
    );
}

#[test]
fn serialize_with_custom_whitespace_and_single_quote_style() {
    let data = include_str!("../../testdata/small_lib.glif");
    let glyph = parse_glyph(data.as_bytes()).unwrap();
    let options =
        WriteOptions::default().indent(WriteOptions::SPACE, 2).quote_char(QuoteChar::Single);
    let two_spaces = glyph.encode_xml_with_options(&options).unwrap();
    let two_spaces = std::str::from_utf8(&two_spaces).unwrap();

    pretty_assertions::assert_eq!(
        two_spaces,
        r#"<?xml version='1.0' encoding='UTF-8'?>
<glyph name="hello" format="2">
  <advance width="1200"/>
  <outline>
    <contour>
      <point x="2" y="30" type="line"/>
      <point x="44" y="10" type="line"/>
    </contour>
  </outline>
  <lib>
    <dict>
      <key>test.key</key>
      <string>I am a creative professional :)</string>
    </dict>
  </lib>
  <note>durp</note>
</glyph>
"#
    );
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
            Anchor::new(10.0, 10.0, Some("top".into()), None, None,),
            Anchor::new(10.0, 20.0, Some("bottom".into()), None, None,),
            Anchor::new(30.0, 20.0, Some("left".into()), None, None,),
            Anchor::new(40.0, 20.0, Some("right".into()), None, None,),
        ]
    );
    assert_eq!(glyph.guidelines, vec![]);
    assert_eq!(glyph.image, None);
    assert_eq!(glyph.lib, Plist::new());
    assert_eq!(glyph.note, None);
}

#[test]
fn curve_types() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/D_.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(glyph.contours.len(), 2);
    assert_eq!(glyph.contours[1].points[0].typ, PointType::Line);
    assert!(!glyph.contours[1].points[0].smooth);
    assert!(glyph.contours[1].points[1].smooth);
    assert_eq!(glyph.contours[1].points[2].typ, PointType::OffCurve);
    assert_eq!(glyph.contours[1].points[4].typ, PointType::Curve);
}

#[test]
#[allow(clippy::float_cmp)]
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
#[should_panic(expected = "DuplicateElement")]
fn duplicate_outline() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
  <outline>
  </outline>
  <outline/>
</glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "ComponentMissingBase")]
fn component_missing_base() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
  <outline>
    <component/>
  </outline>
</glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "ComponentEmptyBase")]
fn component_empty_base() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
  <outline>
    <component base=""/>
  </outline>
</glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "BadAngle")]
fn bad_angle() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
  <guideline x="1" y="2" angle="-10"/>
</glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "LibMustBeDictionary")]
fn lib_must_be_dict() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
    <lib>
        <string>I am a creative professional :)</string>
    </lib>
</glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "PublicObjectLibsMustBeDictionary")]
fn public_object_libs_must_be_dict() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
    <lib>
        <dict>
            <key>public.objectLibs</key>
            <string>0,1,0,0.5</string>
        </dict>
    </lib>
</glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
#[should_panic(expected = "ObjectLibMustBeDictionary")]
fn object_lib_must_be_dict() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
	<anchor name="top" x="74" y="197" identifier="KN3WZjorob"/>
    <lib>
        <dict>
            <key>public.objectLibs</key>
            <dict>
                <key>KN3WZjorob</key>
                <string>0,1,0,0.5</string>
            </dict>
        </dict>
    </lib>
</glyph>
"#;
    let _ = parse_glyph(data.as_bytes()).unwrap();
}

#[test]
fn if_no_one_uses_your_lib_is_it_broken() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
    <lib>
        <dict>
            <key>public.objectLibs</key>
            <dict>
                <key>KN3WZjorob</key>
                <string>0,1,0,0.5</string>
            </dict>
        </dict>
    </lib>
</glyph>
"#;
    let glyph = parse_glyph(data.as_bytes()).unwrap();
    assert!(glyph.lib.get("public.objectLibs").is_none());
}

#[test]
fn parse_note() {
    let bytes = include_bytes!("../../testdata/note.glif");
    let glyph = parse_glyph(bytes).unwrap();
    assert_eq!(glyph.note, Some(".notdef".to_string()));
}

#[test]
#[allow(clippy::float_cmp)]
fn save() {
    let bytes = include_bytes!("../../testdata/sample_period.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    let buf = glyph.encode_xml().expect("encode failed");

    println!("{}", String::from_utf8_lossy(&buf));
    //panic!("ahh");

    let glyph2 = parse_glyph(buf.as_slice()).expect("re-parse failed");
    assert_eq!(glyph.name, glyph2.name);
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

#[test]
fn notdef_failure() {
    let bytes = include_bytes!("../../testdata/noto-cjk-notdef.glif");
    let _ = parse_glyph(bytes).unwrap();
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
#[should_panic(expected = "InvalidName")]
fn invalid_name() {
    let data = "
        <?xml version=\"1.0\" encoding=\"UTF-8\"?>
        <glyph name=\"\x01\" format=\"2\">
            <outline>
                <contour>
                    <point x=\"572\" y=\"667\"/>
                    <point x=\"479\" y=\"714\"/>
                </contour>
            </outline>
        </glyph>
    ";
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

#[test]
fn pointtype_display_trait() {
    assert_eq!(format!("{}", PointType::Move), "move");
    assert_eq!(format!("{}", PointType::Line), "line");
    assert_eq!(format!("{}", PointType::OffCurve), "offcurve");
    assert_eq!(format!("{}", PointType::Curve), "curve");
    assert_eq!(format!("{}", PointType::QCurve), "qcurve");
}

#[test]
fn pointtype_from_str_trait() {
    assert!(PointType::from_str("move").unwrap() == PointType::Move);
    assert!(PointType::from_str("line").unwrap() == PointType::Line);
    assert!(PointType::from_str("offcurve").unwrap() == PointType::OffCurve);
    assert!(PointType::from_str("curve").unwrap() == PointType::Curve);
    assert!(PointType::from_str("qcurve").unwrap() == PointType::QCurve);
}

#[test]
#[should_panic(expected = "UnknownPointType")]
fn pointtype_from_str_unknown_type() {
    PointType::from_str("bogus").unwrap();
}

#[test]
fn components_load() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_dieresis.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    // component order
    assert_eq!(glyph.components[0].base, "A");
    assert_eq!(glyph.components[1].base, "dieresis");
    let error_margin = f64::EPSILON;
    // component affine transforms
    assert!(glyph.components[0].transform.x_scale - 1.0 < error_margin);
    assert!(glyph.components[0].transform.y_scale - 1.0 < error_margin);
    assert!(glyph.components[0].transform.xy_scale - 0.0 < error_margin);
    assert!(glyph.components[0].transform.yx_scale - 0.0 < error_margin);
    assert!(glyph.components[0].transform.x_offset - 0.0 < error_margin);
    assert!(glyph.components[0].transform.y_offset - 0.0 < error_margin);

    assert!(glyph.components[1].transform.x_scale - 1.0 < error_margin);
    assert!(glyph.components[1].transform.y_scale - 1.0 < error_margin);
    assert!(glyph.components[1].transform.xy_scale - 0.0 < error_margin);
    assert!(glyph.components[1].transform.yx_scale - 0.0 < error_margin);
    assert!(glyph.components[1].transform.x_offset - 421.0 < error_margin);
    assert!(glyph.components[1].transform.y_offset - 20.0 < error_margin);
}

#[test]
fn has_component() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_dieresis.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    assert!(glyph.has_component());

    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    assert!(!glyph.has_component());
}

#[test]
fn component_count() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_dieresis.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    assert_eq!(glyph.component_count(), 2);

    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    assert_eq!(glyph.component_count(), 0);
}

#[test]
fn get_components_with_base() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_dieresis.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");

    assert_eq!(glyph.components[0].base, "A");
    assert_eq!(glyph.components[1].base, "dieresis");

    let component_a_vec = glyph.get_components_with_base("A").collect::<Vec<&Component>>();
    assert!(component_a_vec.len() == 1);
    assert_eq!(glyph.components[0], *component_a_vec[0]);

    let component_dieresis_vec =
        glyph.get_components_with_base("dieresis").collect::<Vec<&Component>>();
    assert!(component_dieresis_vec.len() == 1);
    assert_eq!(glyph.components[1], *component_dieresis_vec[0]);
}

#[test]
fn get_components_with_base_multiple_same_base_components() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/quotedblbase.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    let error_margin = f64::EPSILON;
    assert_eq!(glyph.components[0].base, "comma");
    assert!(glyph.components[0].transform.x_offset - 0.0 < error_margin);
    assert_eq!(glyph.components[1].base, "comma");
    assert!(glyph.components[1].transform.x_offset - 130.0 < error_margin);

    let component_comma_vec = glyph.get_components_with_base("comma").collect::<Vec<&Component>>();
    assert!(component_comma_vec.len() == 2);
    assert_eq!(glyph.components[0], *component_comma_vec[0]);
    assert_eq!(glyph.components[1], *component_comma_vec[1]);
}

#[test]
fn get_components_with_base_missing() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_dieresis.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    assert!(glyph.get_components_with_base("Z").next().is_none());
}

#[test]
fn has_component_with_base() {
    let bytes = include_bytes!("../../testdata/MutatorSansLightWide.ufo/glyphs/A_dieresis.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    assert!(glyph.has_component_with_base("A"));
    assert!(glyph.has_component_with_base("dieresis"));
    assert!(!glyph.has_component_with_base("Z"));
}

#[test]
fn deduplicate_unicodes2() {
    let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
  <unicode hex="0065"/>
  <unicode hex="0066"/>
  <unicode hex="0065"/>
  <unicode hex="0067"/>
</glyph>
"#;
    let mut glyph = parse_glyph(data.as_bytes()).unwrap();
    assert_eq!(glyph.codepoints, Codepoints::new(['e', 'f', 'g']));

    glyph.codepoints = Codepoints::new(['e', 'f', 'e', 'g']);
    let data2 = glyph.encode_xml().unwrap();
    let data2 = std::str::from_utf8(&data2).unwrap();
    let data2_expected = r#"<?xml version="1.0" encoding="UTF-8"?>
<glyph name="period" format="2">
	<unicode hex="0065"/>
	<unicode hex="0066"/>
	<unicode hex="0067"/>
</glyph>
"#;
    assert_eq!(data2, data2_expected);
}

#[test]
fn bom_glif() {
    let bytes = include_bytes!("../../testdata/bom_glif.glif");
    let glyph = parse_glyph(bytes).expect("initial load failed");
    assert_eq!(glyph.lib.get("hi").unwrap().as_string(), Some("hello"));
}
