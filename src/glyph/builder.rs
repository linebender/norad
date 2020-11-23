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
/// use norad::{
///     AffineTransform, Anchor, GlifVersion, GlyphBuilder, Guideline, Identifier, Line,
///     OutlineBuilder, PointType,
/// };
///
/// fn main() -> Result<(), ErrorKind> {
///     let mut builder = GlyphBuilder::new("test", GlifVersion::V2);
///     builder
///         .width(10.0)?
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
///         })?;
///     let mut outline_builder = OutlineBuilder::new();
///     outline_builder
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
///     let (outline, identifiers) = outline_builder.finish()?;
///     builder.outline(outline, identifiers)?;
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
        insert_identifier(&mut self.identifiers, &guideline.identifier)?;
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
        insert_identifier(&mut self.identifiers, &anchor.identifier)?;
        self.glyph.anchors.get_or_insert(Vec::new()).push(anchor);
        Ok(self)
    }

    /// Add an outline to the `Glyph`, along with its identifiers.
    ///
    /// Errors when:
    /// 1. It has been called more than once.
    /// 2. Format version 1 is set but identifiers are passed.
    /// 3. Duplicate identifiers are found.
    pub fn outline(
        &mut self,
        mut outline: Outline,
        identifiers: HashSet<u64>,
    ) -> Result<&mut Self, ErrorKind> {
        if self.glyph.outline.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        if &self.glyph.format == &GlifVersion::V1 && !identifiers.is_empty() {
            return Err(ErrorKind::UnexpectedAttribute);
        }
        if !self.identifiers.is_disjoint(&identifiers) {
            return Err(ErrorKind::DuplicateIdentifier);
        }
        self.identifiers.extend(&identifiers);

        if &self.glyph.format == &GlifVersion::V1 {
            for c in &mut outline.contours {
                if c.points.len() == 1
                    && c.points[0].typ == PointType::Move
                    && c.points[0].name.is_some()
                {
                    let anchor_point = c.points.remove(0);
                    let anchor = Anchor {
                        name: anchor_point.name,
                        x: anchor_point.x,
                        y: anchor_point.y,
                        identifier: None,
                        color: None,
                    };
                    self.glyph.anchors.get_or_insert(Vec::new()).push(anchor);
                }
            }

            // Clean up now empty contours.
            let mut i = 0;
            while i != outline.contours.len() {
                if outline.contours[i].points.len() == 0 {
                    outline.contours.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        self.glyph.outline.replace(outline);
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
        if self.height.is_some() || self.width.is_some() {
            self.glyph.advance = Some(Advance {
                width: self.width.unwrap_or(0.0),
                height: self.height.unwrap_or(0.0),
            })
        }

        self.glyph.format = GlifVersion::V2;
        Ok(self.glyph)
    }
}

#[derive(Debug)]
pub struct OutlineBuilder {
    identifiers: HashSet<u64>,
    outline: Outline,
    scratch_contour: Option<Contour>,
    number_of_offcurves: u32,
}

impl OutlineBuilder {
    pub fn new() -> Self {
        Self {
            identifiers: HashSet::new(),
            outline: Outline::default(),
            scratch_contour: None,
            number_of_offcurves: 0,
        }
    }

    /// Start a new path to be added to the glyph with `end_path()`.
    ///
    /// Errors when:
    /// 1. `outline()` wasn't called first.
    /// 2. a path has been begun already but not ended yet.
    /// 3. format version 1 is set but an identifier has been given.
    /// 4. the identifier is not unique within the glyph.
    pub fn begin_path(&mut self, identifier: Option<Identifier>) -> Result<&mut Self, ErrorKind> {
        if self.scratch_contour.is_some() {
            return Err(ErrorKind::UnfinishedDrawing);
        }
        insert_identifier(&mut self.identifiers, &identifier)?;
        self.scratch_contour.replace(Contour { identifier, points: Vec::new() });
        Ok(self)
    }

    /// Add a point to the path begun by `begin_path()`.
    ///
    /// Errors when:
    /// 1. `begin_path()` wasn't called first.
    /// 2. the identifier is not unique within the outline.
    /// 3. the point is an off-curve with the smooth attribute set.
    /// 4. the point sequence is forbidden by the specification.
    pub fn add_point(
        &mut self,
        point: (f32, f32),
        segment_type: PointType,
        smooth: bool,
        name: Option<String>,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
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
                    PointType::OffCurve => {
                        self.number_of_offcurves = self.number_of_offcurves.saturating_add(1)
                    }
                    PointType::QCurve => self.number_of_offcurves = 0,
                    PointType::Curve => {
                        if self.number_of_offcurves > 2 {
                            return Err(ErrorKind::TooManyOffCurves);
                        }
                        self.number_of_offcurves = 0;
                    }
                }
                insert_identifier(&mut self.identifiers, &point.identifier)?;
                c.points.push(point);
                Ok(self)
            }
            None => Err(ErrorKind::PenPathNotStarted),
        }
    }

    /// Ends the path begun by `begin_path()` and adds the contour it to the glyph's outline.
    ///
    /// Errors when:
    /// 1. `begin_path()` wasn't called first.
    /// 2. the point sequence is forbidden by the specification.
    ///
    /// Discards path in case of error.
    pub fn end_path(&mut self) -> Result<&mut Self, ErrorKind> {
        let contour = self.scratch_contour.take();
        match contour {
            Some(c) => {
                // If ending a closed contour with off-curve points, wrap around and check
                // from the beginning that we have a curve or qcurve following eventually.
                if self.number_of_offcurves > 0 {
                    if c.is_closed() {
                        for point in &c.points {
                            match point.typ {
                                PointType::OffCurve => {
                                    self.number_of_offcurves =
                                        self.number_of_offcurves.saturating_add(1)
                                }
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
                // Empty contours are allowed by the specification but make no sense, skip them.
                if c.points.len() > 0 {
                    self.outline.contours.push(c);
                }
                Ok(self)
            }
            None => Err(ErrorKind::PenPathNotStarted),
        }
    }

    /// Add a component to the glyph.
    ///
    /// Errors when the identifier is not unique within the glyph.
    pub fn add_component(
        &mut self,
        base: GlyphName,
        transform: AffineTransform,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        insert_identifier(&mut self.identifiers, &identifier)?;
        self.outline.components.push(Component { base, transform, identifier });
        Ok(self)
    }

    /// Consume the builder and return the final `Outline` with the set of hashed indetifiers.
    ///
    /// Errors when a path has been begun but not ended.
    pub fn finish(self) -> Result<(Outline, HashSet<u64>), ErrorKind> {
        if self.scratch_contour.is_some() {
            return Err(ErrorKind::UnfinishedDrawing);
        }

        Ok((self.outline, self.identifiers))
    }
}

/// Helper, inserts the hash of an identifier into a builder's set.
fn insert_identifier(
    set: &mut HashSet<u64>,
    identifier: &Option<Identifier>,
) -> Result<(), ErrorKind> {
    if let Some(identifier) = &identifier {
        let identifier_hash = identifier.hash();
        if !set.insert(identifier_hash) {
            return Err(ErrorKind::DuplicateIdentifier);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glyph::Line;

    #[test]
    fn glyph_builder_basic() -> Result<(), ErrorKind> {
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
            })?;

        let mut outline_builder = OutlineBuilder::new();
        outline_builder
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
        let (outline, identifiers) = outline_builder.finish()?;
        builder.outline(outline, identifiers)?;
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
    fn glyph_builder_upgrade_v1_anchor() -> Result<(), ErrorKind> {
        let mut builder = GlyphBuilder::new("test", GlifVersion::V1);

        let mut outline_builder = OutlineBuilder::new();
        outline_builder
            .begin_path(None)?
            .add_point((173.0, 536.0), PointType::Move, false, Some("top".into()), None)?
            .end_path()?;
        let (outline, identifiers) = outline_builder.finish()?;
        builder.outline(outline, identifiers)?;
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
    fn glyph_builder_add_guidelines_duplicate_id() {
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
    fn glyph_builder_add_duplicate_id() {
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
    fn outline_builder_unfinished_drawing() {
        let mut outline_builder = OutlineBuilder::new();
        outline_builder.begin_path(Some(Identifier::new("abc".into()).unwrap())).unwrap();
        outline_builder.finish().unwrap();
    }

    #[test]
    #[should_panic(expected = "UnfinishedDrawing")]
    fn outline_builder_unfinished_drawing2() {
        OutlineBuilder::new()
            .begin_path(Some(Identifier::new("abc".into()).unwrap()))
            .unwrap()
            .begin_path(None)
            .unwrap();
    }
}
