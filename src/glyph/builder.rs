use std::collections::HashSet;

use crate::error::ErrorKind;
use crate::glyph::{
    Advance, AffineTransform, Anchor, Component, Contour, ContourPoint, GlifVersion, Glyph,
    GlyphName, Guideline, Image, Outline, PointType,
};
use crate::shared_types::{Identifier, Plist};

/// A GlyphBuilder is a consuming builder for [`crate::glyph::Glyph`].
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
///         .guideline(Guideline::new(
///             Line::Horizontal(10.0),
///             None,
///             None,
///             Some(Identifier::new("test1")?),
///             None,
///         ))?
///         .anchor(Anchor::new(
///             1.0,
///             2.0,
///             Some("anchor1".into()),
///             None,
///             Some(Identifier::new("test3")?),
///             None,
///         ))?;
///     let mut outline_builder = OutlineBuilder::new();
///     outline_builder
///         .begin_path(Some(Identifier::new("abc")?))?
///         .add_point((173.0, 536.0), PointType::Line, false, None, None)?
///         .add_point((85.0, 536.0), PointType::Line, false, None, None)?
///         .add_point((85.0, 0.0), PointType::Line, false, None, None)?
///         .add_point(
///             (173.0, 0.0),
///             PointType::Line,
///             false,
///             None,
///             Some(Identifier::new("def")?),
///         )?
///         .end_path()?
///         .add_component(
///             "hallo".into(),
///             AffineTransform::default(),
///             Some(Identifier::new("xyz")?),
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
    identifiers: HashSet<Identifier>, // All identifiers within a glyph must be unique.
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
        if self.glyph.format == GlifVersion::V1 {
            return Err(ErrorKind::UnexpectedTag);
        }
        insert_identifier(&mut self.identifiers, guideline.identifier().cloned())?;
        self.glyph.guidelines.get_or_insert(Vec::new()).push(guideline);
        Ok(self)
    }

    /// Add an anchor to the `Glyph`.
    ///
    /// Errors when format version 1 is set or the optional identifier is not unique within the glyph.
    pub fn anchor(&mut self, anchor: Anchor) -> Result<&mut Self, ErrorKind> {
        if self.glyph.format == GlifVersion::V1 {
            return Err(ErrorKind::UnexpectedTag);
        }
        insert_identifier(&mut self.identifiers, anchor.identifier.clone())?;
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
        identifiers: HashSet<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        if self.glyph.outline.is_some() {
            return Err(ErrorKind::UnexpectedDuplicate);
        }
        if self.glyph.format == GlifVersion::V1 && !identifiers.is_empty() {
            return Err(ErrorKind::UnexpectedAttribute);
        }
        if !self.identifiers.is_disjoint(&identifiers) {
            return Err(ErrorKind::DuplicateIdentifier);
        }
        self.identifiers.extend(identifiers);

        if self.glyph.format == GlifVersion::V1 {
            for c in &mut outline.contours {
                if c.points.len() == 1
                    && c.points[0].typ == PointType::Move
                    && c.points[0].name.is_some()
                {
                    let anchor_point = c.points.remove(0);
                    let anchor = Anchor::new(
                        anchor_point.x,
                        anchor_point.y,
                        anchor_point.name,
                        None,
                        None,
                        None,
                    );
                    self.glyph.anchors.get_or_insert(Vec::new()).push(anchor);
                }
            }

            // Clean up now empty contours.
            outline.contours.retain(|c| !c.points.is_empty());
        }

        self.glyph.outline.replace(outline);
        Ok(self)
    }

    /// Add an image to the `Glyph`.
    ///
    /// Errors when format version 1 is set or the function is called more than once.
    pub fn image(&mut self, image: Image) -> Result<&mut Self, ErrorKind> {
        if self.glyph.format == GlifVersion::V1 {
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

/// An OutlineBuilder is a consuming builder for [`crate::glyph::Outline`], not unlike a [fontTools point pen].
///
/// Primarily to be used in conjunction with [`GlyphBuilder`].
///
/// [fontTools point pen]: https://fonttools.readthedocs.io/en/latest/pens/basePen.html
#[derive(Debug, Default)]
pub struct OutlineBuilder {
    identifiers: HashSet<Identifier>,
    outline: Outline,
    scratch_state: OutlineBuilderState,
}

#[derive(Debug)]
enum OutlineBuilderState {
    Idle,
    Drawing { scratch_contour: Contour, number_of_offcurves: u32 },
}

impl Default for OutlineBuilderState {
    fn default() -> Self {
        OutlineBuilderState::Idle
    }
}

impl OutlineBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    /// Begin a new path to be added to the glyph.
    ///
    /// It must be finished with [`Self::end_path`] before the outline can be [`Self::finish`]ed
    /// and retrieved.
    ///
    /// Errors when:
    /// 1. a path has been begun already but not ended yet.
    /// 2. the identifier is not unique within the glyph.
    ///
    /// On error, it won't begin a new path and you can continue drawing the previously started
    /// path.
    pub fn begin_path(&mut self, identifier: Option<Identifier>) -> Result<&mut Self, ErrorKind> {
        match self.scratch_state {
            OutlineBuilderState::Idle => {
                insert_identifier(&mut self.identifiers, identifier.clone())?;
                self.scratch_state = OutlineBuilderState::Drawing {
                    scratch_contour: Contour::new(Vec::new(), identifier, None),
                    number_of_offcurves: 0,
                };
                Ok(self)
            }
            OutlineBuilderState::Drawing { .. } => Err(ErrorKind::UnfinishedDrawing),
        }
    }

    /// Add a point to the path begun by `begin_path()`.
    ///
    /// Errors when:
    /// 1. [`Self::begin_path`] wasn't called first.
    /// 2. the identifier is not unique within the outline.
    /// 3. the point is an off-curve with the smooth attribute set.
    /// 4. the point sequence is forbidden by the specification.
    ///
    /// On error, it won't add any part of the point, but you can try again with a new and improved
    /// point.
    pub fn add_point(
        &mut self,
        (x, y): (f32, f32),
        segment_type: PointType,
        smooth: bool,
        name: Option<String>,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        match &mut self.scratch_state {
            OutlineBuilderState::Idle => Err(ErrorKind::PenPathNotStarted),
            OutlineBuilderState::Drawing { scratch_contour, number_of_offcurves } => {
                // NOTE: Check identifier collision early, a duplicate identifier may otherwise
                // leave behind a changed number_of_offcurves.
                if let Some(identifier) = &identifier {
                    if self.identifiers.contains(identifier) {
                        return Err(ErrorKind::DuplicateIdentifier);
                    }
                }

                match segment_type {
                    PointType::Move => {
                        if !scratch_contour.points.is_empty() {
                            return Err(ErrorKind::UnexpectedMove);
                        }
                    }
                    PointType::Line => {
                        if *number_of_offcurves > 0 {
                            return Err(ErrorKind::UnexpectedPointAfterOffCurve);
                        }
                    }
                    PointType::OffCurve => {
                        if smooth {
                            return Err(ErrorKind::UnexpectedSmooth);
                        }
                        *number_of_offcurves = number_of_offcurves.saturating_add(1)
                    }
                    PointType::QCurve => *number_of_offcurves = 0,
                    PointType::Curve => {
                        if *number_of_offcurves > 2 {
                            return Err(ErrorKind::TooManyOffCurves);
                        }
                        *number_of_offcurves = 0;
                    }
                }
                insert_identifier(&mut self.identifiers, identifier.clone()).unwrap();
                scratch_contour.points.push(ContourPoint::new(
                    x,
                    y,
                    segment_type,
                    smooth,
                    name,
                    identifier,
                    None,
                ));
                Ok(self)
            }
        }
    }

    /// Ends the path begun by [`Self::begin_path`] and adds the contour to the glyph's outline, unless
    /// it's empty.
    ///
    /// Errors when:
    /// 1. [`Self::begin_path`] wasn't called first.
    /// 2. the point sequence is forbidden by the specification.
    ///
    /// On error, it drops the path you were trying to end and you can [`Self::begin_path`] again. It
    /// doesn't change the previously added paths.
    pub fn end_path(&mut self) -> Result<&mut Self, ErrorKind> {
        match std::mem::replace(&mut self.scratch_state, OutlineBuilderState::Idle) {
            OutlineBuilderState::Idle => Err(ErrorKind::PenPathNotStarted),
            OutlineBuilderState::Drawing { scratch_contour, mut number_of_offcurves } => {
                // If ending a closed contour with off-curve points, wrap around and check
                // from the beginning that we have a curve or qcurve following eventually.
                if number_of_offcurves > 0 {
                    if scratch_contour.is_closed() {
                        for point in &scratch_contour.points {
                            match point.typ {
                                PointType::OffCurve => {
                                    number_of_offcurves = number_of_offcurves.saturating_add(1)
                                }
                                PointType::QCurve => break,
                                PointType::Curve => {
                                    if number_of_offcurves > 2 {
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
                // Empty contours are allowed by the specification but make no sense, skip them.
                if !scratch_contour.points.is_empty() {
                    self.outline.contours.push(scratch_contour);
                }
                Ok(self)
            }
        }
    }

    /// Add a component to the glyph.
    ///
    /// Errors when the identifier is not unique within the glyph.
    ///
    /// On error, it won't add the component, but you can try again with a new and improved
    /// component.
    pub fn add_component(
        &mut self,
        base: GlyphName,
        transform: AffineTransform,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        insert_identifier(&mut self.identifiers, identifier.clone())?;
        self.outline.components.push(Component::new(base, transform, identifier, None));
        Ok(self)
    }

    /// Consume the builder and return the final [`Outline`] with its set of hashed indetifiers.
    ///
    /// Errors when a path has been begun but not ended.
    ///
    /// On error, it won't finish the outline and return it to you, but you can [`Self::end_path`]
    /// before trying to finish again.
    pub fn finish(self) -> Result<(Outline, HashSet<Identifier>), ErrorKind> {
        match self.scratch_state {
            OutlineBuilderState::Idle => Ok((self.outline, self.identifiers)),
            OutlineBuilderState::Drawing { .. } => Err(ErrorKind::UnfinishedDrawing),
        }
    }
}

/// Helper, inserts an identifier into a builder's set.
fn insert_identifier(
    set: &mut HashSet<Identifier>,
    identifier: Option<Identifier>,
) -> Result<(), ErrorKind> {
    if let Some(identifier) = identifier {
        if !set.insert(identifier) {
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
            .guideline(Guideline::new(
                Line::Horizontal(10.0),
                None,
                None,
                Some(Identifier::new("test1").unwrap()),
                None,
            ))?
            .guideline(Guideline::new(
                Line::Vertical(20.0),
                None,
                None,
                Some(Identifier::new("test2").unwrap()),
                None,
            ))?
            .anchor(Anchor::new(
                1.0,
                2.0,
                Some("anchor1".into()),
                None,
                Some(Identifier::new("test3").unwrap()),
                None,
            ))?
            .anchor(Anchor::new(
                3.0,
                4.0,
                Some("anchor2".into()),
                None,
                Some(Identifier::new("test4").unwrap()),
                None,
            ))?;

        let mut outline_builder = OutlineBuilder::new();
        outline_builder
            .begin_path(Some(Identifier::new("abc").unwrap()))?
            .add_point((173.0, 536.0), PointType::Line, false, None, None)?
            .add_point((85.0, 536.0), PointType::Line, false, None, None)?
            .add_point((85.0, 0.0), PointType::Line, false, None, None)?
            .add_point(
                (173.0, 0.0),
                PointType::Line,
                false,
                None,
                Some(Identifier::new("def").unwrap()),
            )?
            .end_path()?
            .add_component(
                "hallo".into(),
                AffineTransform::default(),
                Some(Identifier::new("xyz").unwrap()),
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
                    Guideline::new(
                        Line::Horizontal(10.0),
                        None,
                        None,
                        Some(Identifier::new("test1").unwrap()),
                        None,
                    ),
                    Guideline::new(
                        Line::Vertical(20.0),
                        None,
                        None,
                        Some(Identifier::new("test2").unwrap()),
                        None,
                    ),
                ]),
                anchors: Some(vec![
                    Anchor::new(
                        1.0,
                        2.0,
                        Some("anchor1".into()),
                        None,
                        Some(Identifier::new("test3").unwrap()),
                        None
                    ),
                    Anchor::new(
                        3.0,
                        4.0,
                        Some("anchor2".into()),
                        None,
                        Some(Identifier::new("test4").unwrap()),
                        None
                    ),
                ]),
                outline: Some(Outline {
                    contours: vec![Contour::new(
                        vec![
                            ContourPoint::new(
                                173.0,
                                536.0,
                                PointType::Line,
                                false,
                                None,
                                None,
                                None,
                            ),
                            ContourPoint::new(
                                85.0,
                                536.0,
                                PointType::Line,
                                false,
                                None,
                                None,
                                None,
                            ),
                            ContourPoint::new(85.0, 0.0, PointType::Line, false, None, None, None),
                            ContourPoint::new(
                                173.0,
                                0.0,
                                PointType::Line,
                                false,
                                None,
                                Some(Identifier::new("def").unwrap()),
                                None,
                            ),
                        ],
                        Some(Identifier::new("abc").unwrap()),
                        None,
                    )],
                    components: vec![Component::new(
                        "hallo".into(),
                        AffineTransform {
                            x_scale: 1.0,
                            xy_scale: 0.0,
                            yx_scale: 0.0,
                            y_scale: 1.0,
                            x_offset: 0.0,
                            y_offset: 0.0,
                        },
                        Some(Identifier::new("xyz").unwrap()),
                        None,
                    )]
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
                anchors: Some(vec![Anchor::new(
                    173.0,
                    536.0,
                    Some("top".into()),
                    None,
                    None,
                    None
                )]),
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
            .guideline(Guideline::new(
                Line::Horizontal(10.0),
                None,
                None,
                Some(Identifier::new("test1").unwrap()),
                None,
            ))
            .unwrap()
            .guideline(Guideline::new(
                Line::Vertical(20.0),
                None,
                None,
                Some(Identifier::new("test1").unwrap()),
                None,
            ))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "DuplicateIdentifier")]
    fn glyph_builder_add_duplicate_id() {
        GlyphBuilder::new("test", GlifVersion::V2)
            .guideline(Guideline::new(
                Line::Horizontal(10.0),
                None,
                None,
                Some(Identifier::new("test1").unwrap()),
                None,
            ))
            .unwrap()
            .anchor(Anchor::new(
                1.0,
                2.0,
                None,
                None,
                Some(Identifier::new("test1").unwrap()),
                None,
            ))
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "UnfinishedDrawing")]
    fn outline_builder_unfinished_drawing() {
        let mut outline_builder = OutlineBuilder::new();
        outline_builder.begin_path(Some(Identifier::new("abc").unwrap())).unwrap();
        outline_builder.finish().unwrap();
    }

    #[test]
    #[should_panic(expected = "UnfinishedDrawing")]
    fn outline_builder_unfinished_drawing2() {
        OutlineBuilder::new()
            .begin_path(Some(Identifier::new("abc").unwrap()))
            .unwrap()
            .begin_path(None)
            .unwrap();
    }
}
