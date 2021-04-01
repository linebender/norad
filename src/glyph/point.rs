use crate::{Identifier, Plist};

#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub typ: PointType,
    pub smooth: bool,
    pub name: Option<String>,
    /// Unique identifier for the point within the glyph. This attribute is only required
    /// when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// The point's lib for arbitary data.
    lib: Option<Plist>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointType {
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

impl Point {
    pub fn new(
        x: f32,
        y: f32,
        typ: PointType,
        smooth: bool,
        name: Option<String>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        let mut this = Self { x, y, typ, smooth, name, identifier: None, lib: None };
        if let Some(id) = identifier {
            this.replace_identifier(id);
        }
        if let Some(lib) = lib {
            this.replace_lib(lib);
        }
        this
    }

    /// Returns an immutable reference to the contour's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the contour's lib.
    pub fn lib_mut(&mut self) -> Option<&mut Plist> {
        self.lib.as_mut()
    }

    /// Replaces the actual lib by the lib given in parameter, returning the old
    /// lib if present. Sets a new UUID v4 identifier if none is set already.
    pub fn replace_lib(&mut self, lib: Plist) -> Option<Plist> {
        if self.identifier.is_none() {
            self.identifier.replace(Identifier::from_uuidv4());
        }
        self.lib.replace(lib)
    }

    /// Takes the lib out of the contour, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns an immutable reference to the contour's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}
