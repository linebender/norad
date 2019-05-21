use super::*;
use crate::parse::parse_glyph;
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
    assert_eq!(&glyph.name, "period");
    assert_eq!(
        glyph.image.as_ref().map(|img| img.file_name.clone()),
        Some(PathBuf::from("period sketch.png"))
    );
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
    assert_eq!(glyph.advance, Some(Advance::Width(364.)));
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

//#[test]
//fn parse_utf16() {
//let bytes = include_bytes!("../../testdata/utf16-glyph.xml");
//let glyph = parse_glyph(bytes).unwrap();
//assert_eq!(glyph.width, Some(268.));
//}
