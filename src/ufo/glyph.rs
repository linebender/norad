
#[cfg(test)]
#[path = "glyph_tests.rs"]
mod tests;

use std::path::PathBuf;
use std::collections::HashMap;

use quick_xml::{events::{attributes::Attribute, Event}, Reader};

//use crate::error::Error;

type Plist = ();

//type Contents = HashMap<String, PathBuf>;

////Placeholder; documentation is vague
//struct LayerInfo {
    //color: String,
    //lib: String,
//}

pub enum Error {
    ParseError(quick_xml::Error),
    BadXmlDeclaration(String),
    UnsupportedGlifVersion(String),

}

struct Glyph {
    name: String,
    format: GlifVersion,
    width: Option<f64>,
    height: Option<f64>,
    codepoints: Option<Vec<char>>,
    note: Option<String>,
    guidelines: Option<Vec<Guideline>>,
    anchors: Option<Vec<Anchor>>,
    outline: Option<Outline>,
    image: Option<Image>,
    lib: Option<Plist>,
}

enum ParseState {
    Ready,
    Parsing()

}

//impl Glyph {
    //pub fn from_xml(xml: &str) -> Result<Glyph, Error> {
        //let mut reader = Reader::from_str(xml);
        //let mut buf = Vec::new();
        //reader.trim_text(true);

        //let mut name = String::new();
        //let mut format: Option<GlifVersion> = None;
        //let mut width: Option<f64> = None;
        //let mut height: Option<f64> = None;
        //let mut codepoints: Option<Vec<char>> = None;
        //let mut note: Option<String> = None;
        //let mut guidelines: Option<Vec<Guideline>> = None;
        //let mut anchors: Option<Vec<Anchor>> = None;
        //let mut outline: Option<Outline> = None;
        //let mut image: Option<Image> = None;
        //let mut lib: Option<Plist> = None;

        //fn start(r: &mut Reader, b: &mut [u8]) -> Result<(String, GlifVersion), Error> {
            //loop {
                //match reader.read_event(&mut buf) {
                    //Ok(Event::Decl(_)) => (),
                    //Ok(Event::Start(ref tag)) if tag.name() == "glyphs".as_bytes() => {
                        //let mut name: Option<String> = None;
                        //let mut format: Option<GlifVersion> = None;
                        //for attr in tag.attributes() {
                            //let Attribute { key, value } = attr.map_err(|e| Error::ParseError(e))?;
                            //match key {
                                //b"name" => {
                                    //name = value.unescape_and_decode(&reader).ok();
                                //}
                                //b"format" if value == b"2" => {
                                    //format = Some(GlifVersion::V2);
                                //}
                                //b"format" => return Err(Error::UnsupportedGlifVersion(String::from_utf8_lossy(value).to_owned())),
                                //_other => (), // ignore unknown attrs for now?
                            //}
                        //}


                    //}
                //}
            //}
        //}
    //}
//}

enum GlifVersion {
    V1 = 1,
    V2 = 2,
}

/// Identifiers are optional attributes of several objects in the UFO.
/// These identifiers are required to be unique within certain contexts
/// as defined on a per object basis throughout this specification.
/// Identifiers are specified as a string between one and 100 characters long.
/// All characters must be in the printable ASCII range, 0x20 to 0x7E.
struct Identifier(String);

/// A guideline associated with a glyph.
pub struct Guideline {
    /// The line itself.
    line: Line,
    /// An arbitrary name for the guideline.
    name: Option<String>,
    /// The color of the line.
    color: Option<Color>,
    /// Unique identifier for the guideline. This attribute is not required
    /// and should only be added to guidelines as needed.
    identifier: Option<Identifier>,
}

pub enum Line {
    /// A vertical line, passing through a given `x` coordinate.
    Vertical(f32),
    /// A horizontal line, passing through a given `y` coordinate.
    Horizontal(f32),
    /// An angled line passing through `(x, y)` at `degrees` degrees counteer-clockwise
    /// to the horizontal.
    Angle { x: f32, y: f32, degrees: f32 },
}

struct Anchor {
    x: f32,
    y: f32,
    /// An arbitrary name for the anchor.
    name: Option<String>,
    color: Option<Color>,
    identifier: Option<Identifier>,
}

struct Outline {
    component: Vec<Component>,
    contour: Vec<Contour>,
}

/// Another glyph inserted as part of the outline.
pub struct Component {
    /// The name of the base glyph.
    base: Option<String>,
    transform: AffineTransform,
    identifier: Option<Identifier>,
}

struct Contour {
    identifier: Option<Identifier>,

}

struct ContourPoint {
    name: Option<String>,
    x: f32,
    y: f32,
    typ: PointType,
    smooth: bool,
    identifier: Option<Identifier>,
}


enum PointType {
    /// A point of this type must be the first in a contour. The reverse is not true:
    /// a contour does not necessarily start with a move point. When a contour
    /// does start with a move point, it signifies the beginning of an open contour.
    /// A closed contour does not start with a move and is defined as a cyclic
    /// list of points, with no predominant start point. There is always a next
    /// point and a previous point. For this purpose the list of points can be
    /// seen as endless in both directions. The actual list of points can be
    /// rotated arbitrarily (by removing the first N points and appending
    /// them at the end) while still describing the same outline.
    Move,
    /// Draw a straight line from the previous point to this point.
    /// The previous point must be a move, a line, a curve or a qcurve.
    /// It must not be an offcurve.
    Line,
    /// This point is part of a curve segment that goes up to the next point
    /// that is either a curve or a qcurve.
    OffCurve,
    /// Draw a cubic bezier curve from the last non-offcurve point to this point.
    /// The number of offcurve points can be zero, one or two.
    /// If the number of offcurve points is zero, a straight line is drawn.
    /// If it is one, a quadratic curve is drawn.
    /// If it is two, a regular cubic bezier is drawn.
    Curve,
    /// Similar to curve, but uses quadratic curves, using the TrueType
    /// “implied on-curve points” principle.
    QCurve,

}

/// Taken together in order, these fields represent an affine transformation matrix.
struct AffineTransform {
    x_scale: f32,
    xy_scale: f32,
    yx_scale: f32,
    y_scale: f32,
    x_offset: f32,
    y_offset: f32,
}

impl AffineTransform {
    ///  [1 0 0 1 0 0]; the identity transformation.
    fn identity() -> Self {
        AffineTransform {
            x_scale: 1.0,
            xy_scale: 0.,
            yx_scale: 0.,
            y_scale: 1.0,
            x_offset: 0.,
            y_offset: 0.,
        }
    }
}


impl std::default::Default for AffineTransform {
    fn default() -> Self {
        Self::identity()
    }
}

struct Color {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
}

struct Image {
    /// Not an absolute / relative path, but the name of the image file.
    file_name: PathBuf,
    color: Option<Color>,
}

