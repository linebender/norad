use std::collections::HashSet;

use crate::error::ErrorKind;
use crate::glyph::{
    Advance, AffineTransform, Anchor, Component, Contour, ContourPoint, GlifVersion, Glyph,
    GlyphName, Guideline, Image, Outline, Plist, PointType,
};
use crate::shared_types::Identifier;

/// A GlyphBuilder is a consuming builder for [`Glyph`](../struct.Glyph.html)s.
///
/// It is different from fontTools' Pen concept, in that it is used to build the entire `Glyph`,
/// not just the outlines and components, with built-in checking for conformity to the glif
/// specification. It also upgrades all Glyphs to the highest supported version.
///
/// Specifically, the specification defines the following constraints:
/// 1. If a "move" occurs, it must be the first point of a contour. ufoLib may allow straying from
///    this for format 1, we don't.
/// 2. The "smooth" attribute must not be set on off-curve points.
/// 3. An off-curve point must be followed by a curve or qcurve.
/// 4. A maximum of two offcurves can precede a curve.
/// 5. All identifiers used in a `Glyph` must be unique within it.
///
/// Since GlyphBuilder is also used by the Glif parser, additional constraints are baked into GlyphBuilder to enforce
/// constraints about how often a Glyph field or "element" can appear in a `.glif` file. For
/// example, calling `outline()` twice results in an error.
///
/// # Example
///
/// ```
/// use std::str::FromStr;
///
/// use norad::error::ErrorKind;
/// use norad::{AffineTransform, Anchor, GlifVersion, Guideline, Identifier, Line, GlyphBuilder, PointType};
///
/// fn main() -> Result<(), ErrorKind> {
///     let mut builder = GlyphBuilder::new("test", GlifVersion::V2);
///     builder.width(10.0)?
///         .unicode('ä')
///         .guideline(Guideline {
///             line: Line::Horizontal(10.0),
///             name: None,
///             color: None,
///             identifier: Some(Identifier::from_str("test1")?),
///         })?
///         .anchor(Anchor {
///             x: 1.0,
///             y: 2.0,
///             name: Some("anchor1".into()),
///             color: None,
///             identifier: Some(Identifier::from_str("test3")?),
///         })?
///         .outline()?
///         .begin_path(Some(Identifier::from_str("abc")?))?
///         .add_point((173.0, 536.0), PointType::Line, false, None, None)?
///         .add_point((85.0, 536.0), PointType::Line, false, None, None)?
///         .add_point((85.0, 0.0), PointType::Line, false, None, None)?
///         .add_point((173.0, 0.0), PointType::Line, false, None, Some(Identifier::from_str("def")?))?
///         .end_path()?
///         .add_component(
///             "hallo".into(),
///             AffineTransform::default(),
///             Some(Identifier::from_str("xyz")?),
///         )?;
///     let glyph = builder.finish()?;
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct GlyphBuilder {
    glyph: Glyph,
    height: Option<f32>,
    width: Option<f32>,
    identifiers: HashSet<u64>, // All identifiers within a glyph must be unique.
    scratch_contour: Option<Contour>,
    number_of_offcurves: u32,
}

impl GlyphBuilder {
    /// Create a new GlyphBuilder for a `Glyph` named `name`, using the format version `format` to interpret
    /// commands.
    pub fn new(name: impl Into<GlyphName>, format: GlifVersion) -> Self {
        Self {
            glyph: Glyph::new(name.into(), format),
            height: None,
            width: None,
            identifiers: HashSet::new(),
            scratch_contour: None,
            number_of_offcurves: 0,
        }
    }

    /// Return the format version currently set.
    pub fn get_format(&self) -> &GlifVersion {
        &self.glyph.format
    }

    /// Set the glyph width.
    ///
    /// Errors when the function is called more than once.
    pub fn width(&mut self, width: f32) -> Result<&mut Self, ErrorKind> {
        if self.width.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        self.width.replace(width);
        Ok(self)
    }

    /// Set the glyph height.
    ///
    /// Errors when the function is called more than once.
    pub fn height(&mut self, height: f32) -> Result<&mut Self, ErrorKind> {
        if self.height.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        self.height.replace(height);
        Ok(self)
    }

    /// Add the Unicode value `char` to the `Glyph`'s Unicode values.
    pub fn unicode(&mut self, unicode: char) -> &mut Self {
        self.glyph.codepoints.get_or_insert(Vec::new()).push(unicode);
        self
    }

    /// Add a note to the `Glyph`.
    ///
    /// Errors when the function is called more than once.
    pub fn note(&mut self, note: String) -> Result<&mut Self, ErrorKind> {
        if self.glyph.note.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        self.glyph.note.replace(note);
        Ok(self)
    }

    /// Add a guideline to the `Glyph`. The optional identifier must be unique within the `Glyph`.
    ///
    /// Errors when format version 1 is set.
    pub fn guideline(&mut self, guideline: Guideline) -> Result<&mut Self, ErrorKind> {
        if &self.glyph.format == &GlifVersion::V1 {
            return Err(ErrorKind::UnexpectedTag);
        }
        if let Some(identifier) = &guideline.identifier {
            let identifier_hash = identifier.hash();
            if !self.identifiers.insert(identifier_hash) {
                return Err(ErrorKind::DuplicateIdentifier);
            }
        }
        self.glyph.guidelines.get_or_insert(Vec::new()).push(guideline);
        Ok(self)
    }

    /// Add an anchor to the `Glyph`.
    ///
    /// Errors when format version 1 is set or the optional identifier is not unique within the glyph.
    pub fn anchor(&mut self, anchor: Anchor) -> Result<&mut Self, ErrorKind> {
        if &self.glyph.format == &GlifVersion::V1 {
            return Err(ErrorKind::UnexpectedTag);
        }
        if let Some(identifier) = &anchor.identifier {
            let identifier_hash = identifier.hash();
            if !self.identifiers.insert(identifier_hash) {
                return Err(ErrorKind::DuplicateIdentifier);
            }
        }
        self.glyph.anchors.get_or_insert(Vec::new()).push(anchor);
        Ok(self)
    }

    /// Add an outline to the `Glyph`.
    ///
    /// Errors when the function is called more than once.
    pub fn outline(&mut self) -> Result<&mut Self, ErrorKind> {
        if self.glyph.outline.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        self.glyph.outline.replace(Outline::default());
        Ok(self)
    }

    /// Add an image to the `Glyph`.
    ///
    /// Errors when format version 1 is set or the function is called more than once.
    pub fn image(&mut self, image: Image) -> Result<&mut Self, ErrorKind> {
        if &self.glyph.format == &GlifVersion::V1 {
            return Err(ErrorKind::UnexpectedTag);
        }
        if self.glyph.image.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        self.glyph.image.replace(image);
        Ok(self)
    }

    /// Add a lib to the `Glyph`.
    ///
    /// Errors when the function is called more than once.
    pub fn lib(&mut self, lib: Plist) -> Result<&mut Self, ErrorKind> {
        if self.glyph.lib.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        self.glyph.lib.replace(lib);
        Ok(self)
    }

    /// Consume the builder and return the final `Glyph`.
    ///
    /// Errors when a path has been begun but not ended.
    pub fn finish(mut self) -> Result<Glyph, ErrorKind> {
        if self.scratch_contour.is_some() {
            return Err(ErrorKind::UnfinishedDrawing);
        }
        if self.height.is_some() || self.width.is_some() {
            self.glyph.advance = Some(Advance {
                width: self.width.unwrap_or(0.0),
                height: self.height.unwrap_or(0.0),
            })
        }

        self.glyph.format = GlifVersion::V2;
        Ok(self.glyph)
    }

    /// Start a new path to be added to the glyph with `end_path()`.
    ///
    /// Errors when:
    /// 1. `outline()` wasn't called first.
    /// 2. a path has been begun already but not ended yet.
    /// 3. format version 1 is set but an identifier has been given.
    /// 4. the identifier is not unique within the glyph.
    pub fn begin_path(&mut self, identifier: Option<Identifier>) -> Result<&mut Self, ErrorKind> {
        if self.glyph.outline.is_none() {
            return Err(ErrorKind::UnexpectedDrawing);
        }
        if self.scratch_contour.is_some() {
            return Err(ErrorKind::UnfinishedDrawing);
        }
        if &self.glyph.format == &GlifVersion::V1 && identifier.is_some() {
            return Err(ErrorKind::UnexpectedAttribute);
        }
        if let Some(identifier) = &identifier {
            let identifier_hash = identifier.hash();
            if !self.identifiers.insert(identifier_hash) {
                return Err(ErrorKind::DuplicateIdentifier);
            }
        }
        self.scratch_contour.replace(Contour { identifier, points: Vec::new() });
        Ok(self)
    }

    /// Add a point to the path begun by `begin_path()`.
    ///
    /// Errors when:
    /// 1. `outline()` wasn't called first.
    /// 2. `begin_path()` wasn't called first.
    /// 3. format version 1 is set but an identifier has been given.
    /// 4. the identifier is not unique within the glyph.
    /// 5. the point is an off-curve with the smooth attribute set.
    /// 6. the point sequence is forbidden by the specification.
    pub fn add_point(
        &mut self,
        point: (f32, f32),
        segment_type: PointType,
        smooth: bool,
        name: Option<String>,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        if self.glyph.outline.is_none() {
            return Err(ErrorKind::UnexpectedDrawing);
        }
        if &self.glyph.format == &GlifVersion::V1 && identifier.is_some() {
            return Err(ErrorKind::UnexpectedAttribute);
        }
        if smooth && segment_type == PointType::OffCurve {
            return Err(ErrorKind::UnexpectedSmooth);
        }

        let point =
            ContourPoint { name, x: point.0, y: point.1, typ: segment_type, smooth, identifier };
        match &mut self.scratch_contour {
            Some(c) => {
                match point.typ {
                    PointType::Move => {
                        if !c.points.is_empty() {
                            return Err(ErrorKind::UnexpectedMove);
                        }
                    }
                    PointType::Line => {
                        if self.number_of_offcurves > 0 {
                            return Err(ErrorKind::UnexpectedPointAfterOffCurve);
                        }
                    }
                    PointType::OffCurve => self.number_of_offcurves += 1,
                    PointType::QCurve => self.number_of_offcurves = 0,
                    PointType::Curve => {
                        if self.number_of_offcurves > 2 {
                            return Err(ErrorKind::TooManyOffCurves);
                        }
                        self.number_of_offcurves = 0;
                    }
                }
                if let Some(identifier) = &point.identifier {
                    // TODO: test membership at fn start and insert() before push()?
                    let identifier_hash = identifier.hash();
                    if !self.identifiers.insert(identifier_hash) {
                        return Err(ErrorKind::DuplicateIdentifier);
                    }
                }
                c.points.push(point);
                Ok(self)
            }
            None => Err(ErrorKind::PenPathNotStarted),
        }
    }

    /// Ends the path begun by `begin_path()` and adds the contour it to the glyph's outline.
    ///
    /// Errors when:
    /// 1. `outline()` wasn't called first.
    /// 2. `begin_path()` wasn't called first.
    /// 3. the point sequence is forbidden by the specification.
    ///
    /// Discards path in case of error.
    pub fn end_path(&mut self) -> Result<&mut Self, ErrorKind> {
        if self.glyph.outline.is_none() {
            return Err(ErrorKind::UnexpectedDrawing);
        }

        let contour = self.scratch_contour.take();
        match contour {
            Some(mut c) => {
                // If using format version 1, check if we are actually looking at an implicit
                // anchor and convert if so (leaving an empty outline if there is nothing else).
                if Self::contour_is_v1_anchor(self.get_format(), &c.points) {
                    let anchor_point = c.points.remove(0);
                    let anchor = Anchor {
                        name: anchor_point.name,
                        x: anchor_point.x,
                        y: anchor_point.y,
                        identifier: None,
                        color: None,
                    };
                    self.glyph.anchors.get_or_insert(Vec::new()).push(anchor);
                    Ok(self)
                // If ending a closed contour with off-curve points, wrap around and check
                // from the beginning that we have a curve or qcurve following eventually.
                } else {
                    if self.number_of_offcurves > 0 {
                        if c.is_closed() {
                            for point in &c.points {
                                match point.typ {
                                    PointType::OffCurve => self.number_of_offcurves += 1,
                                    PointType::QCurve => break,
                                    PointType::Curve => {
                                        if self.number_of_offcurves > 2 {
                                            return Err(ErrorKind::TooManyOffCurves);
                                        }
                                        break;
                                    }
                                    PointType::Line => {
                                        return Err(ErrorKind::UnexpectedPointAfterOffCurve);
                                    }
                                    PointType::Move => unreachable!(),
                                }
                            }
                        } else {
                            return Err(ErrorKind::TrailingOffCurves);
                        }
                    }
                    self.number_of_offcurves = 0;
                    if c.points.len() > 0 {
                        self.glyph.outline.get_or_insert(Outline::default()).contours.push(c);
                    }
                    Ok(self)
                }
            }
            None => Err(ErrorKind::PenPathNotStarted),
        }
    }

    /// Add a component to the glyph.
    ///
    /// Errors when:
    /// 1. `outline()` wasn't called first.
    /// 2. format version 1 is set but an identifier has been given.
    /// 3. the identifier is not unique within the glyph.
    pub fn add_component(
        &mut self,
        base: GlyphName,
        transform: AffineTransform,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        if self.glyph.outline.is_none() {
            return Err(ErrorKind::UnexpectedDrawing);
        }
        if &self.glyph.format == &GlifVersion::V1 && identifier.is_some() {
            return Err(ErrorKind::UnexpectedAttribute);
        }
        if let Some(identifier) = &identifier {
            let identifier_hash = identifier.hash();
            if !self.identifiers.insert(identifier_hash) {
                return Err(ErrorKind::DuplicateIdentifier);
            }
        }
        let component = Component { base, transform, identifier };
        self.glyph.outline.get_or_insert(Outline::default()).components.push(component);
        Ok(self)
    }

    /// Check if a contour is really an informal anchor according to the Glif v2 specification.
    fn contour_is_v1_anchor(format: &GlifVersion, points: &[ContourPoint]) -> bool {
        *format == GlifVersion::V1
            && points.len() == 1
            && points[0].typ == PointType::Move
            && points[0].name.is_some()
    }
}

// #[derive(Debug)]
// pub struct OutlineBuilder {
//     identifiers: HashSet<u64>, // All identifiers within a glyph must be unique.
//     scratch_contour: Option<Contour>,
//     number_of_offcurves: u32,
// }

// impl OutlineBuilder {

// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::Line;

    #[test]
    fn pen_one_line() -> Result<(), ErrorKind> {
        let mut builder = GlyphBuilder::new("test", GlifVersion::V2);
        builder
            .width(10.0)?
            .height(20.0)?
            .unicode('\u{2020}')
            .unicode('\u{2021}')
            .note("hello".into())?
            .guideline(Guideline {
                line: Line::Horizontal(10.0),
                name: None,
                color: None,
                identifier: Some(Identifier::new("test1".into()).unwrap()),
            })?
            .guideline(Guideline {
                line: Line::Vertical(20.0),
                name: None,
                color: None,
                identifier: Some(Identifier::new("test2".into()).unwrap()),
            })?
            .anchor(Anchor {
                x: 1.0,
                y: 2.0,
                name: Some("anchor1".into()),
                color: None,
                identifier: Some(Identifier::new("test3".into()).unwrap()),
            })?
            .anchor(Anchor {
                x: 3.0,
                y: 4.0,
                name: Some("anchor2".into()),
                color: None,
                identifier: Some(Identifier::new("test4".into()).unwrap()),
            })?
            .outline()?
            .begin_path(Some(Identifier::new("abc".into()).unwrap()))?
            .add_point((173.0, 536.0), PointType::Line, false, None, None)?
            .add_point((85.0, 536.0), PointType::Line, false, None, None)?
            .add_point((85.0, 0.0), PointType::Line, false, None, None)?
            .add_point(
                (173.0, 0.0),
                PointType::Line,
                false,
                None,
                Some(Identifier::new("def".into()).unwrap()),
            )?
            .end_path()?
            .add_component(
                "hallo".into(),
                AffineTransform::default(),
                Some(Identifier::new("xyz".into()).unwrap()),
            )?;
        let glyph = builder.finish()?;

        assert_eq!(
            glyph,
            Glyph {
                name: "test".into(),
                format: GlifVersion::V2,
                advance: Some(Advance { height: 20.0, width: 10.0 }),
                codepoints: Some(vec!['†', '‡']),
                note: Some("hello".into()),
                guidelines: Some(vec![
                    Guideline {
                        line: Line::Horizontal(10.0),
                        name: None,
                        color: None,
                        identifier: Some(Identifier::new("test1".into()).unwrap()),
                    },
                    Guideline {
                        line: Line::Vertical(20.0),
                        name: None,
                        color: None,
                        identifier: Some(Identifier::new("test2".into()).unwrap()),
                    },
                ]),
                anchors: Some(vec![
                    Anchor {
                        x: 1.0,
                        y: 2.0,
                        name: Some("anchor1".into()),
                        color: None,
                        identifier: Some(Identifier::new("test3".into()).unwrap()),
                    },
                    Anchor {
                        x: 3.0,
                        y: 4.0,
                        name: Some("anchor2".into()),
                        color: None,
                        identifier: Some(Identifier::new("test4".into()).unwrap()),
                    },
                ]),
                outline: Some(Outline {
                    contours: vec![Contour {
                        identifier: Some(Identifier::new("abc".into()).unwrap()),
                        points: vec![
                            ContourPoint {
                                name: None,
                                x: 173.0,
                                y: 536.0,
                                typ: PointType::Line,
                                smooth: false,
                                identifier: None,
                            },
                            ContourPoint {
                                name: None,
                                x: 85.0,
                                y: 536.0,
                                typ: PointType::Line,
                                smooth: false,
                                identifier: None,
                            },
                            ContourPoint {
                                name: None,
                                x: 85.0,
                                y: 0.0,
                                typ: PointType::Line,
                                smooth: false,
                                identifier: None,
                            },
                            ContourPoint {
                                name: None,
                                x: 173.0,
                                y: 0.0,
                                typ: PointType::Line,
                                smooth: false,
                                identifier: Some(Identifier::new("def".into()).unwrap()),
                            },
                        ],
                    },],
                    components: vec![Component {
                        base: "hallo".into(),
                        transform: AffineTransform {
                            x_scale: 1.0,
                            xy_scale: 0.0,
                            yx_scale: 0.0,
                            y_scale: 1.0,
                            x_offset: 0.0,
                            y_offset: 0.0,
                        },
                        identifier: Some(Identifier::new("xyz".into()).unwrap()),
                    }]
                }),
                image: None,
                lib: None,
            }
        );

        Ok(())
    }

    #[test]
    fn pen_upgrade_v1_anchor() -> Result<(), ErrorKind> {
        let mut builder = GlyphBuilder::new("test", GlifVersion::V1);
        builder
            .outline()?
            .begin_path(None)?
            .add_point((173.0, 536.0), PointType::Move, false, Some("top".into()), None)?
            .end_path()?;
        let glyph = builder.finish()?;

        assert_eq!(
            glyph,
            Glyph {
                name: "test".into(),
                format: GlifVersion::V2,
                advance: None,
                codepoints: None,
                note: None,
                guidelines: None,
                anchors: Some(vec![Anchor {
                    x: 173.0,
                    y: 536.0,
                    name: Some("top".into()),
                    color: None,
                    identifier: None,
                }]),
                outline: Some(Outline::default()),
                image: None,
                lib: None,
            }
        );

        Ok(())
    }

    #[test]
    #[should_panic(expected = "DuplicateIdentifier")]
    fn pen_add_guidelines_duplicate_id() {
        GlyphBuilder::new("test", GlifVersion::V2)
            .guideline(Guideline {
                line: Line::Horizontal(10.0),
                name: None,
                color: None,
                identifier: Some(Identifier::new("test1".into()).unwrap()),
            })
            .unwrap()
            .guideline(Guideline {
                line: Line::Vertical(20.0),
                name: None,
                color: None,
                identifier: Some(Identifier::new("test1".into()).unwrap()),
            })
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "DuplicateIdentifier")]
    fn pen_add_duplicate_id() {
        GlyphBuilder::new("test", GlifVersion::V2)
            .guideline(Guideline {
                line: Line::Horizontal(10.0),
                name: None,
                color: None,
                identifier: Some(Identifier::new("test1".into()).unwrap()),
            })
            .unwrap()
            .anchor(Anchor {
                x: 1.0,
                y: 2.0,
                name: None,
                color: None,
                identifier: Some(Identifier::new("test1".into()).unwrap()),
            })
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "UnfinishedDrawing")]
    fn pen_unfinished_drawing() {
        let mut builder = GlyphBuilder::new("test", GlifVersion::V2);
        builder
            .outline()
            .unwrap()
            .begin_path(Some(Identifier::new("abc".into()).unwrap()))
            .unwrap();
        let _glyph = builder.finish().unwrap();
    }

    #[test]
    #[should_panic(expected = "UnfinishedDrawing")]
    fn pen_unfinished_drawing2() {
        let mut builder = GlyphBuilder::new("test", GlifVersion::V2);
        builder
            .outline()
            .unwrap()
            .begin_path(Some(Identifier::new("abc".into()).unwrap()))
            .unwrap()
            .begin_path(None)
            .unwrap();
    }
}
