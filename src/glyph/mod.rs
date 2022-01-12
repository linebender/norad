//! Data related to individual glyphs.

pub mod builder;
mod parse;
mod serialize;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

#[cfg(feature = "kurbo")]
use crate::error::ConvertContourError;

#[cfg(feature = "druid")]
use druid::{Data, Lens};

use crate::error::{ErrorKind, GlifLoadError, GlifWriteError, StoreError};
use crate::name::Name;
use crate::names::NameList;
use crate::shared_types::PUBLIC_OBJECT_LIBS_KEY;
use crate::{Color, Guideline, Identifier, Line, Plist, WriteOptions};

/// A glyph, loaded from a [`.glif` file][glif].
///
/// [glif]: http://unifiedfontobject.org/versions/ufo3/glyphs/glif/
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Lens))]
pub struct Glyph {
    /// The name of the glyph.
    pub name: Name,
    /// Glif file format version.
    pub format: GlifVersion,
    /// Glyph height.
    pub height: f64,
    /// Glyph width.
    pub width: f64,
    /// A collection of glyph Unicode code points.
    ///
    /// The first entry defines the primary Unicode value for this glyph.
    pub codepoints: Vec<char>,
    /// Arbitrary glyph note.
    pub note: Option<String>,
    /// A collection of glyph guidelines.
    pub guidelines: Vec<Guideline>,
    /// A collection of glyph anchors.
    pub anchors: Vec<Anchor>,
    /// A collection of glyph components.
    pub components: Vec<Component>,
    /// A collection of glyph contours.
    pub contours: Vec<Contour>,
    /// Glyph image data.
    pub image: Option<Image>,
    /// Glyph library data.
    pub lib: Plist,
}

impl Glyph {
    /// Attempt to parse a `Glyph` from a [`.glif`] at the provided path.
    ///
    /// [`.glif`]: http://unifiedfontobject.org/versions/ufo3/glyphs/glif/
    pub fn load(path: impl AsRef<Path>) -> Result<Self, GlifLoadError> {
        let path = path.as_ref();
        let names = NameList::default();
        Glyph::load_with_names(path, &names)
    }

    /// Attempt to load the glyph at `path`, reusing names from the `NameList`.
    ///
    /// This uses string interning to reuse allocations when a glyph name
    /// occurs multiple times (such as in components or in different layers).
    pub(crate) fn load_with_names(path: &Path, names: &NameList) -> Result<Self, GlifLoadError> {
        std::fs::read(path)
            .map_err(GlifLoadError::Io)
            .and_then(|data| parse::GlifParser::from_xml(&data, Some(names)))
    }

    #[doc(hidden)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), GlifWriteError> {
        let path = path.as_ref();
        let opts = WriteOptions::default();
        self.save_with_options(path, &opts)
    }

    pub(crate) fn save_with_options(
        &self,
        path: &Path,
        opts: &WriteOptions,
    ) -> Result<(), GlifWriteError> {
        if self.format != GlifVersion::V2 {
            return Err(GlifWriteError::Downgrade);
        }
        if self.lib.contains_key(PUBLIC_OBJECT_LIBS_KEY) {
            return Err(GlifWriteError::PreexistingPublicObjectLibsKey);
        }

        let data = self.encode_xml_with_options(opts)?;
        std::fs::write(path, &data).map_err(GlifWriteError::Io)?;

        Ok(())
    }

    /// Returns a new, "empty" [`Glyph`] with the given `name`.
    ///
    /// # Panics
    ///
    /// panics if `name` is empty or if it contains any [control characters].
    ///
    /// [control characters]: https://unifiedfontobject.org/versions/ufo3/conventions/#controls
    pub fn new_named(name: &str) -> Self {
        Glyph::new(Name::new_raw(name), GlifVersion::V2)
    }

    pub(crate) fn new(name: Name, format: GlifVersion) -> Self {
        Glyph {
            name,
            format,
            height: 0.0,
            width: 0.0,
            codepoints: Vec::new(),
            note: None,
            guidelines: Vec::new(),
            anchors: Vec::new(),
            components: Vec::new(),
            contours: Vec::new(),
            image: None,
            lib: Plist::new(),
        }
    }

    /// Returns true if [`Glyph`] contains one or more [`Component`]s.
    pub fn has_component(&self) -> bool {
        !self.components.is_empty()
    }

    /// Returns the number of [`Component`]s in the Glyph.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Returns true if the Glyph contains one or more [`Component`]s with base
    /// glyph name `basename`.
    pub fn has_component_with_base(&self, basename: &str) -> bool {
        self.components.iter().any(|x| *x.base == *basename)
    }

    /// Returns an iterator over immutable [`Component`] references filtered by base glyph name.
    pub fn get_components_with_base<'b, 'a: 'b>(
        &'a self,
        basename: &'b str,
    ) -> impl Iterator<Item = &'a Component> + 'b {
        self.components.iter().filter(move |x| *x.base == *basename)
    }

    /// Move libs from the lib's `public.objectLibs` into the actual objects.
    /// The key will be removed from the glyph lib.
    fn load_object_libs(&mut self) -> Result<(), GlifLoadError> {
        // Use a macro to reduce boilerplate, to avoid having to mess with the typing system.
        macro_rules! transfer_lib {
            ($object:expr, $object_libs:expr) => {
                if let Some(id) = $object.identifier().map(|v| v.as_str()) {
                    if let Some(lib) = $object_libs.remove(id) {
                        let lib = lib
                            .into_dictionary()
                            .ok_or(GlifLoadError::ObjectLibMustBeDictionary(id.into()))?;
                        $object.replace_lib(lib);
                    }
                }
            };
        }

        let mut object_libs = match self.lib.remove(PUBLIC_OBJECT_LIBS_KEY) {
            Some(lib) => {
                lib.into_dictionary().ok_or(GlifLoadError::PublicObjectLibsMustBeDictionary)?
            }
            None => return Ok(()),
        };

        for anchor in &mut self.anchors {
            transfer_lib!(anchor, object_libs);
        }
        for guideline in &mut self.guidelines {
            transfer_lib!(guideline, object_libs);
        }
        for contour in &mut self.contours {
            transfer_lib!(contour, object_libs);
            for point in &mut contour.points {
                transfer_lib!(point, object_libs);
            }
        }
        for component in &mut self.components {
            transfer_lib!(component, object_libs);
        }

        Ok(())
    }

    /// Dump guideline libs into a Plist.
    fn dump_object_libs(&self) -> Plist {
        let mut object_libs = Plist::default();

        let mut dump_lib = |id: Option<&Identifier>, lib: &Plist| {
            let id = id.map(|id| id.as_str().to_string());
            object_libs.insert(id.unwrap(), plist::Value::Dictionary(lib.clone()));
        };

        for anchor in &self.anchors {
            if let Some(lib) = anchor.lib() {
                dump_lib(anchor.identifier(), lib);
            }
        }

        for guideline in &self.guidelines {
            if let Some(lib) = guideline.lib() {
                dump_lib(guideline.identifier(), lib);
            }
        }

        for contour in &self.contours {
            if let Some(lib) = contour.lib() {
                dump_lib(contour.identifier(), lib);
            }
            for point in &contour.points {
                if let Some(lib) = point.lib() {
                    dump_lib(point.identifier(), lib);
                }
            }
        }
        for component in &self.components {
            if let Some(lib) = component.lib() {
                dump_lib(component.identifier(), lib);
            }
        }

        object_libs
    }
}

#[cfg(feature = "druid")]
impl Data for Glyph {
    fn same(&self, other: &Glyph) -> bool {
        self.name.same(&other.name)
            && self.format.same(&other.format)
            && self.height.same(&other.height)
            && self.width.same(&other.width)
            && self.codepoints == other.codepoints
            && self.note == other.note
            && self.guidelines == other.guidelines
            && self.anchors == other.anchors
            && self.components == other.components
            && self.contours == other.contours
            && self.image == other.image
            && self.lib == other.lib
    }
}

/// Version of a `.glif` file, per the [UFO spec].
///
/// [UFO spec]: https://unifiedfontobject.org/versions/ufo1/glyphs/glif/#specification
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub enum GlifVersion {
    /// Glif file format version 1. Saving this version is not supported.
    V1 = 1,
    /// Glif file format version 2.
    V2 = 2,
}

/// A reference position in a glyph, such as for attaching accents.
///
/// See the [Anchor section] of the UFO spec for more information.
///
/// [Anchor section]: https://unifiedfontobject.org/versions/ufo3/glyphs/glif/#anchor
#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    /// Anchor x coordinate value.
    pub x: f64,
    /// Anchor y coordinate value.
    pub y: f64,
    /// Optional arbitrary name for the anchor.
    pub name: Option<String>,
    /// Optional anchor color.
    pub color: Option<Color>,
    /// Optional unique identifier for the anchor within the glyph.
    ///
    /// This attribute is only required when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// Optional anchor lib for arbitary data.
    lib: Option<Plist>,
}

/// A reference to another glyph, to be included in this glyph's outline.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    /// The name of the base glyph used in the component.
    pub base: Name,
    /// Component affine transormation definition.
    pub transform: AffineTransform,
    /// Optional unique identifier for the component within the glyph.
    ///
    /// This attribute is only required when a lib is present and should otherwise only
    /// be added as needed.
    identifier: Option<Identifier>,
    ///  Optional lib for arbitary component data.
    lib: Option<Plist>,
}

/// A single open or closed bezier path segment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Contour {
    /// A collection of contour points.
    pub points: Vec<ContourPoint>,
    /// Unique identifier for the contour within the glyph.
    ///
    /// This attribute is only required when a lib is present and should otherwise only
    /// be added as needed.
    identifier: Option<Identifier>,
    /// Optional lib for arbitary contour data.
    lib: Option<Plist>,
}

impl Contour {
    /// Whether the contour is closed.
    pub fn is_closed(&self) -> bool {
        self.points.first().map_or(true, |v| v.typ != PointType::Move)
    }

    /// Converts the `Contour` to a [`kurbo::BezPath`].
    #[cfg(feature = "kurbo")]
    pub fn to_kurbo(&self) -> Result<kurbo::BezPath, ConvertContourError> {
        let mut path = kurbo::BezPath::new();
        let mut offs = std::collections::VecDeque::new();
        let mut points = if self.is_closed() {
            // Add end-of-contour offcurves to queue
            let rotate = self
                .points
                .iter()
                .rev()
                .position(|pt| pt.typ != PointType::OffCurve)
                .map(|idx| self.points.len() - 1 - idx);
            self.points.iter().cycle().skip(rotate.unwrap_or(0)).take(self.points.len() + 1)
        } else {
            self.points.iter().cycle().skip(0).take(self.points.len())
        };
        if let Some(start) = points.next() {
            path.move_to(start.to_kurbo());
        }
        for pt in points {
            let kurbo_point = pt.to_kurbo();
            match pt.typ {
                PointType::Move => path.move_to(kurbo_point),
                PointType::Line => path.line_to(kurbo_point),
                PointType::OffCurve => offs.push_back(kurbo_point),
                PointType::Curve => {
                    match offs.make_contiguous() {
                        [] => return Err(ConvertContourError::new(ErrorKind::BadPoint)),
                        [p1] => path.quad_to(*p1, kurbo_point),
                        [p1, p2] => path.curve_to(*p1, *p2, kurbo_point),
                        _ => return Err(ConvertContourError::new(ErrorKind::TooManyOffCurves)),
                    };
                    offs.clear();
                }
                PointType::QCurve => {
                    while let Some(pt) = offs.pop_front() {
                        if let Some(next) = offs.front() {
                            let implied_point = pt.midpoint(*next);
                            path.quad_to(pt, implied_point);
                        } else {
                            path.quad_to(pt, kurbo_point);
                        }
                    }
                    offs.clear();
                }
            }
        }
        Ok(path)
    }
}

/// A single point in a [`Contour`].
#[derive(Debug, Clone, PartialEq)]
pub struct ContourPoint {
    /// Contour point x coordinate value.
    pub x: f64,
    /// Contour point y coordinate value.
    pub y: f64,
    /// Contour point type.
    pub typ: PointType,
    /// Whether a smooth curvature should be maintained at this point. Must not be set for off-curve points.
    pub smooth: bool,
    /// Optional contour point name.
    pub name: Option<String>,
    /// Optional unique identifier for the point within the glyph.
    ///
    /// This attribute is only required when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// Optional lib for arbitary contour point data.
    lib: Option<Plist>,
}

/// Possible types of points that can exist in a [`Contour`].
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

/// FromStr trait implementation for [`PointType`].
impl std::str::FromStr for PointType {
    type Err = ErrorKind;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "move" => Ok(PointType::Move),
            "line" => Ok(PointType::Line),
            "offcurve" => Ok(PointType::OffCurve),
            "curve" => Ok(PointType::Curve),
            "qcurve" => Ok(PointType::QCurve),
            _other => Err(ErrorKind::UnknownPointType),
        }
    }
}

/// Display trait implementation for [`PointType`].
impl std::fmt::Display for PointType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PointType::Move => write!(f, "move"),
            PointType::Line => write!(f, "line"),
            PointType::OffCurve => write!(f, "offcurve"),
            PointType::Curve => write!(f, "curve"),
            PointType::QCurve => write!(f, "qcurve"),
        }
    }
}

/// A 2D affine transformation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct AffineTransform {
    /// x-scale value.
    pub x_scale: f64,
    /// xy-scale value.
    pub xy_scale: f64,
    /// yx-scale value.
    pub yx_scale: f64,
    /// y-scale value.
    pub y_scale: f64,
    /// x-offset value.
    pub x_offset: f64,
    /// y-offset value.
    pub y_offset: f64,
}

impl Anchor {
    /// Returns a new [`Anchor`] given `x` and `y` coordinate values.
    pub fn new(
        x: f64,
        y: f64,
        name: Option<String>,
        color: Option<Color>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        let mut this = Self { x, y, name, color, identifier: None, lib: None };
        if let Some(id) = identifier {
            this.replace_identifier(id);
        }
        if let Some(lib) = lib {
            this.replace_lib(lib);
        }
        this
    }

    /// Returns a reference to the anchor's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the anchor's lib.
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

    /// Takes the lib out of the anchor, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns a reference to the anchor's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}

impl Contour {
    /// Returns a new [`Contour`] given a vector of contour points.
    pub fn new(
        points: Vec<ContourPoint>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        let mut this = Self { points, identifier: None, lib: None };
        if let Some(id) = identifier {
            this.replace_identifier(id);
        }
        if let Some(lib) = lib {
            this.replace_lib(lib);
        }
        this
    }

    /// Returns a reference to the contour's lib.
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

    /// Returns a reference to the contour's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}

impl ContourPoint {
    /// Returns a new [`ContourPoint`] given an `x` coordinate value,
    /// `y` coordinate value, point type, and smooth definition.
    pub fn new(
        x: f64,
        y: f64,
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

    /// Returns a reference to the contour's lib.
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

    /// Returns a reference to the contour's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }

    /// Returns a [`kurbo::Point`] with this `ContourPoint`'s coordinates.
    #[cfg(feature = "kurbo")]
    pub fn to_kurbo(&self) -> kurbo::Point {
        kurbo::Point::new(self.x as f64, self.y as f64)
    }

    /// Applies a transformation matrix to the point's coordinates
    pub fn transform(&mut self, transform: AffineTransform) {
        let new_x = transform.x_scale * self.x + transform.yx_scale * self.y + transform.x_offset;
        let new_y = transform.xy_scale * self.x + transform.y_scale * self.y + transform.y_offset;
        self.x = new_x;
        self.y = new_y;
    }
}

impl Component {
    /// Returns a new [`Component`] given a base glyph name and affine transformation definition.
    ///
    /// The 'name' argument should be taken from an existing glyph in  the same layer.
    pub fn new(
        base: Name,
        transform: AffineTransform,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        let mut this = Self { base, transform, identifier: None, lib: None };
        if let Some(id) = identifier {
            this.replace_identifier(id);
        }
        if let Some(lib) = lib {
            this.replace_lib(lib);
        }
        this
    }

    /// Returns a reference to the component's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the component's lib.
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

    /// Takes the lib out of the component, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns a reference to the component's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
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

/// An image included in a glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// The name of the image file. Must be a base file name, no subdirectories involved.
    file_name: PathBuf,
    /// Optional image color.
    pub color: Option<Color>,
    /// Affine transformation.
    pub transform: AffineTransform,
}

impl Image {
    /// Create a new image.
    pub fn new(
        file_name: PathBuf,
        color: Option<Color>,
        transform: AffineTransform,
    ) -> Result<Self, StoreError> {
        // Note: Mostly mirrors [`Self::validate_entry`].
        if file_name.as_os_str().is_empty() {
            return Err(StoreError::EmptyPath);
        }
        if file_name.is_absolute() {
            return Err(StoreError::PathIsAbsolute);
        }
        if file_name.parent().map_or(false, |p| !p.as_os_str().is_empty()) {
            return Err(StoreError::Subdir);
        }
        Ok(Self { file_name, color, transform })
    }

    /// Returns the file name of the image.
    pub fn file_name(&self) -> &Path {
        self.file_name.as_path()
    }
}

#[cfg(feature = "kurbo")]
impl From<AffineTransform> for kurbo::Affine {
    fn from(src: AffineTransform) -> kurbo::Affine {
        kurbo::Affine::new([
            src.x_scale,
            src.xy_scale,
            src.yx_scale,
            src.y_scale,
            src.x_offset,
            src.y_offset,
        ])
    }
}

#[cfg(feature = "kurbo")]
impl From<kurbo::Affine> for AffineTransform {
    fn from(src: kurbo::Affine) -> AffineTransform {
        let coeffs = src.as_coeffs();
        AffineTransform {
            x_scale: coeffs[0],
            xy_scale: coeffs[1],
            yx_scale: coeffs[2],
            y_scale: coeffs[3],
            x_offset: coeffs[4],
            y_offset: coeffs[5],
        }
    }
}

#[cfg(feature = "druid")]
impl From<druid::piet::Color> for Color {
    fn from(src: druid::piet::Color) -> Color {
        let rgba = src.as_rgba_u32();
        let r = ((rgba >> 24) & 0xff) as f64 / 255.0;
        let g = ((rgba >> 16) & 0xff) as f64 / 255.0;
        let b = ((rgba >> 8) & 0xff) as f64 / 255.0;
        let a = (rgba & 0xff) as f64 / 255.0;
        assert!((0.0..=1.0).contains(&b), "b: {}, raw {}", b, (rgba & (0xff << 8)));

        Color::new(
            r.max(0.0).min(1.0),
            g.max(0.0).min(1.0),
            b.max(0.0).min(1.0),
            a.max(0.0).min(1.0),
        )
        .unwrap()
    }
}

#[cfg(feature = "druid")]
impl From<Color> for druid::piet::Color {
    fn from(src: Color) -> druid::piet::Color {
        let (red, green, blue, alpha) = src.channels();
        druid::piet::Color::rgba(red, green, blue, alpha)
    }
}
