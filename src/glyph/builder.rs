//! A builder for outlines.
//!
//! An [`OutlineBuilder`] is a point-oriented builder for a glyph's graphical outline,
//! not unlike a [fontTools point pen], but different, because it does not draw _into_ a
//! glyph due to ownership issues.
//!
//! To be used internally by [`super::parse::GlifParser`]. Does not keep track of identifier
//! uniqueness (`GlifParser` has to).
//!
//! [fontTools point pen]: https://fonttools.readthedocs.io/en/latest/pens/basePen.html

use crate::{
    error::ErrorKind, AffineTransform, Component, Contour, ContourPoint, Identifier, Name,
    PointType,
};

#[derive(Debug, Default)]
pub(crate) struct OutlineBuilder {
    components: Vec<Component>,
    contours: Vec<Contour>,
    scratch_state: OutlineBuilderState,
}

#[derive(Debug, Default)]
enum OutlineBuilderState {
    #[default]
    Idle,
    Drawing {
        scratch_contour: Contour,
        number_of_offcurves: u32,
    },
}

impl OutlineBuilder {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Begin a new path to be added to the glyph.
    ///
    /// It must be finished with [`Self::end_path`] before the outline can be
    /// [`Self::finish`]ed and retrieved.
    ///
    /// Errors when a path has been begun already but not ended yet.
    ///
    /// On error, it won't begin a new path and you can continue drawing the previously
    /// started path.
    pub(crate) fn begin_path(
        &mut self,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        match self.scratch_state {
            OutlineBuilderState::Idle => {
                self.scratch_state = OutlineBuilderState::Drawing {
                    scratch_contour: Contour::new(Vec::new(), identifier),
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
    /// 2. the point is an off-curve with the smooth attribute set.
    /// 3. the point sequence is forbidden by the specification.
    ///
    /// On error, it won't add any part of the point, but you can try again with a new
    /// and improved point.
    pub(crate) fn add_point(
        &mut self,
        (x, y): (f64, f64),
        segment_type: PointType,
        smooth: bool,
        name: Option<Name>,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, ErrorKind> {
        match &mut self.scratch_state {
            OutlineBuilderState::Idle => Err(ErrorKind::PenPathNotStarted),
            OutlineBuilderState::Drawing { scratch_contour, number_of_offcurves } => {
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
                        *number_of_offcurves = number_of_offcurves.saturating_add(1);
                    }
                    PointType::QCurve => *number_of_offcurves = 0,
                    PointType::Curve => {
                        if *number_of_offcurves > 2 {
                            return Err(ErrorKind::TooManyOffCurves);
                        }
                        *number_of_offcurves = 0;
                    }
                }
                scratch_contour.points.push(ContourPoint::new(
                    x,
                    y,
                    segment_type,
                    smooth,
                    name,
                    identifier,
                ));
                Ok(self)
            }
        }
    }

    /// Ends the path begun by [`Self::begin_path`] and adds the contour to the glyph's
    /// outline, unless it's empty.
    ///
    /// Errors when:
    /// 1. [`Self::begin_path`] wasn't called first.
    /// 2. the point sequence is forbidden by the specification.
    ///
    /// On error, it drops the path you were trying to end and you can
    /// [`Self::begin_path`] again. It doesn't change the previously added paths.
    pub(crate) fn end_path(&mut self) -> Result<&mut Self, ErrorKind> {
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
                                    number_of_offcurves = number_of_offcurves.saturating_add(1);
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
                    self.contours.push(scratch_contour);
                }
                Ok(self)
            }
        }
    }

    /// Add a component to the glyph.
    pub(crate) fn add_component(
        &mut self,
        base: Name,
        transform: AffineTransform,
        identifier: Option<Identifier>,
    ) -> &mut Self {
        self.components.push(Component::new(base, transform, identifier));
        self
    }

    /// Consume the builder and return the final [`Contour`]s and [`Component`]s.
    ///
    /// Errors when a path has been begun but not ended.
    ///
    /// On error, it won't finish the outline and return it to you, but you can
    /// [`Self::end_path`] before trying to finish again.
    pub(crate) fn finish(self) -> Result<(Vec<Contour>, Vec<Component>), ErrorKind> {
        match self.scratch_state {
            OutlineBuilderState::Idle => Ok((self.contours, self.components)),
            OutlineBuilderState::Drawing { .. } => Err(ErrorKind::UnfinishedDrawing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_basic() -> Result<(), ErrorKind> {
        let mut outline_builder = OutlineBuilder::new();
        outline_builder
            .begin_path(Some(Identifier::new_raw("abc")))?
            .add_point((173.0, 536.0), PointType::Line, false, None, None)?
            .add_point((85.0, 536.0), PointType::Line, false, None, None)?
            .add_point((85.0, 0.0), PointType::Line, false, None, None)?
            .add_point(
                (173.0, 0.0),
                PointType::Line,
                false,
                None,
                Some(Identifier::new_raw("def")),
            )?
            .end_path()?
            .add_component(
                Name::new_raw("hallo"),
                AffineTransform::default(),
                Some(Identifier::new_raw("xyz")),
            );
        let (contours, components) = outline_builder.finish()?;

        assert_eq!(
            contours,
            vec![Contour::new(
                vec![
                    ContourPoint::new(173.0, 536.0, PointType::Line, false, None, None),
                    ContourPoint::new(85.0, 536.0, PointType::Line, false, None, None),
                    ContourPoint::new(85.0, 0.0, PointType::Line, false, None, None),
                    ContourPoint::new(
                        173.0,
                        0.0,
                        PointType::Line,
                        false,
                        None,
                        Some(Identifier::new_raw("def")),
                    ),
                ],
                Some(Identifier::new_raw("abc")),
            )]
        );

        assert_eq!(
            components,
            vec![Component::new(
                Name::new_raw("hallo"),
                AffineTransform {
                    x_scale: 1.0,
                    xy_scale: 0.0,
                    yx_scale: 0.0,
                    y_scale: 1.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                },
                Some(Identifier::new_raw("xyz")),
            )]
        );

        Ok(())
    }

    #[test]
    #[should_panic(expected = "UnfinishedDrawing")]
    fn outline_builder_unfinished_drawing() {
        let mut outline_builder = OutlineBuilder::new();
        outline_builder.begin_path(Some(Identifier::new_raw("abc"))).unwrap();
        outline_builder.finish().unwrap();
    }

    #[test]
    #[should_panic(expected = "UnfinishedDrawing")]
    fn outline_builder_unfinished_drawing2() {
        OutlineBuilder::new()
            .begin_path(Some(Identifier::new_raw("abc")))
            .unwrap()
            .begin_path(None)
            .unwrap();
    }
}
