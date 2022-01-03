//! A builder for outlines.
//!
//! An [`OutlineBuilder`] is a point-oriented builder for a glyph's graphical outline,
//! not unlike a [fontTools point pen], but different, because it does not draw _into_ a
//! glyph due to ownership issues.
//!
//! To be used internally by [`GlifParser`]. Does not keep track of identifier
//! uniqueness (`GlifParser` has to).
//!
//! [fontTools point pen]: https://fonttools.readthedocs.io/en/latest/pens/basePen.html

use crate::{AffineTransform, Component, Contour, ContourPoint, GlyphName, Identifier, PointType};

#[derive(Debug, Default)]
pub(crate) struct OutlineBuilder {
    components: Vec<Component>,
    contours: Vec<Contour>,
    scratch_state: OutlineBuilderState,
}

#[derive(Debug)]
enum OutlineBuilderState {
    Idle,
    Drawing { scratch_contour: Contour, number_of_offcurves: u32 },
}

/// An error that occurs while attempting to draw a glyph.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OutlineBuilderError {
    /// The contour pen path was not started.
    #[error("must call begin_path() before calling add_point() or end_path()")]
    PenPathNotStarted,
    /// Has too many off curve points in sequence.
    #[error("at most two off-curve points can precede a curve")]
    TooManyOffCurves,
    /// Has trailing off curve points defined.
    #[error("open contours must not have trailing off-curves")]
    TrailingOffCurves,
    /// Has an unexpected move definition.
    #[error("unexpected move point, can only occur at start of contour")]
    UnexpectedMove,
    /// Has an unexpected point following an off curve point definition.
    #[error("an off-curve point must be followed by a curve or qcurve")]
    UnexpectedPointAfterOffCurve,
    /// Has an unexpected smooth definition.
    #[error("unexpected smooth attribute on an off-curve point")]
    UnexpectedSmooth,
    /// Has incomplete drawing data.
    #[error("unfinished drawing, you must call end_path()")]
    UnfinishedDrawing,
}

impl Default for OutlineBuilderState {
    fn default() -> Self {
        OutlineBuilderState::Idle
    }
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
    ) -> Result<&mut Self, OutlineBuilderError> {
        match self.scratch_state {
            OutlineBuilderState::Idle => {
                self.scratch_state = OutlineBuilderState::Drawing {
                    scratch_contour: Contour::new(Vec::new(), identifier, None),
                    number_of_offcurves: 0,
                };
                Ok(self)
            }
            OutlineBuilderState::Drawing { .. } => Err(OutlineBuilderError::UnfinishedDrawing),
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
        name: Option<String>,
        identifier: Option<Identifier>,
    ) -> Result<&mut Self, OutlineBuilderError> {
        match &mut self.scratch_state {
            OutlineBuilderState::Idle => Err(OutlineBuilderError::PenPathNotStarted),
            OutlineBuilderState::Drawing { scratch_contour, number_of_offcurves } => {
                match segment_type {
                    PointType::Move => {
                        if !scratch_contour.points.is_empty() {
                            return Err(OutlineBuilderError::UnexpectedMove);
                        }
                    }
                    PointType::Line => {
                        if *number_of_offcurves > 0 {
                            return Err(OutlineBuilderError::UnexpectedPointAfterOffCurve);
                        }
                    }
                    PointType::OffCurve => {
                        if smooth {
                            return Err(OutlineBuilderError::UnexpectedSmooth);
                        }
                        *number_of_offcurves = number_of_offcurves.saturating_add(1)
                    }
                    PointType::QCurve => *number_of_offcurves = 0,
                    PointType::Curve => {
                        if *number_of_offcurves > 2 {
                            return Err(OutlineBuilderError::TooManyOffCurves);
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
                    None,
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
    pub(crate) fn end_path(&mut self) -> Result<&mut Self, OutlineBuilderError> {
        match std::mem::replace(&mut self.scratch_state, OutlineBuilderState::Idle) {
            OutlineBuilderState::Idle => Err(OutlineBuilderError::PenPathNotStarted),
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
                                        return Err(OutlineBuilderError::TooManyOffCurves);
                                    }
                                    break;
                                }
                                PointType::Line => {
                                    return Err(OutlineBuilderError::UnexpectedPointAfterOffCurve);
                                }
                                PointType::Move => unreachable!(),
                            }
                        }
                    } else {
                        return Err(OutlineBuilderError::TrailingOffCurves);
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
        base: GlyphName,
        transform: AffineTransform,
        identifier: Option<Identifier>,
    ) -> &mut Self {
        self.components.push(Component::new(base, transform, identifier, None));
        self
    }

    /// Consume the builder and return the final [`Contour`]s and [`Component`]s.
    ///
    /// Errors when a path has been begun but not ended.
    ///
    /// On error, it won't finish the outline and return it to you, but you can
    /// [`Self::end_path`] before trying to finish again.
    pub(crate) fn finish(self) -> Result<(Vec<Contour>, Vec<Component>), OutlineBuilderError> {
        match self.scratch_state {
            OutlineBuilderState::Idle => Ok((self.contours, self.components)),
            OutlineBuilderState::Drawing { .. } => Err(OutlineBuilderError::UnfinishedDrawing),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
