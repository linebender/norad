use kurbo::{BezPath, Line, ParamCurve, Point, Rect, Shape};
use norad::glyph::{Contour, ContourPoint, Glyph, PointType};
use norad::{GlifVersion, GlyphBuilder, Layer, OutlineBuilder};

fn main() {
    for arg in std::env::args().skip(1) {
        let mut ufo = match norad::Ufo::load(&arg) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Loading UFO failed: {}", e);
                std::process::exit(1);
            }
        };

        let angle: f64 = ufo.font_info.as_ref().map_or(0.0, |info| match info.italic_angle {
            Some(a) => -a.get(),
            None => 0.0,
        });
        let xheight: f64 = ufo.font_info.as_ref().map_or(0.0, |info| match info.x_height {
            Some(a) => a.get(),
            None => 0.0,
        });
        let param_overshoot: f64 = 0.0;
        let param_depth: f64 = 15.0;
        let param_sample_frequency: usize = 5;

        // TODO: fetch actual reference glyph.
        let default_layer = ufo.get_default_layer().unwrap();
        let decomposed_glyphs: Vec<Glyph> = default_layer
            .iter_contents()
            .filter(|glyph| {
                glyph
                    .outline
                    .as_ref()
                    .map_or(false, |o| !o.components.is_empty() || !o.contours.is_empty())
            })
            .map(|glyph| {
                draw_polygon(
                    &glyph,
                    &glyph,
                    &default_layer,
                    angle,
                    xheight,
                    param_overshoot,
                    param_depth,
                    param_sample_frequency,
                )
            })
            .collect();

        // Write out background layer.
        let mut decomposed_layer = norad::LayerInfo {
            name: "public.background".into(),
            path: std::path::PathBuf::from("glyphs.background"),
            layer: Layer::default(),
        };
        for glyph in decomposed_glyphs {
            decomposed_layer.layer.insert_glyph(glyph)
        }
        ufo.layers.push(decomposed_layer);

        ufo.meta.creator = "org.linebender.norad".into();
        let output_path = std::path::PathBuf::from(&arg);
        ufo.save(std::path::PathBuf::from("/tmp").join(output_path.file_name().unwrap())).unwrap();
    }
}

fn spacing_polygons(
    paths: &BezPath,
    bounds: &Rect,
    lower_bound_reference: isize,
    upper_bound_reference: isize,
    angle: f64,
    xheight: f64,
    scan_frequency: usize,
    depth_cut: f64,
) -> (Vec<Point>, Point, Point, Vec<Point>, Point, Point) {
    // For deskewing angled glyphs. Makes subsequent processing easier.
    let skew_offset = xheight / 2.0;
    let tan_angle = angle.to_radians().tan();

    // First pass: Collect the outer intersections of a horizontal line with the glyph on both sides, going bottom
    // to top. The spacing polygon is vertically limited to lower_bound_reference..=upper_bound_reference,
    // but we need to collect the extreme points on both sides for the full stretch for spacing later.

    // A glyph can over- or undershoot its reference bounds. Measure the tallest stretch.
    let lower_bound_sampling = (bounds.min_y().round() as isize).min(lower_bound_reference);
    let upper_bound_sampling = (bounds.max_y().round() as isize).max(upper_bound_reference);
    let mut left = Vec::new();
    let left_bounds = bounds.min_x();
    let mut extreme_left_full: Option<Point> = None;
    let mut extreme_left: Option<Point> = None;
    let mut right = Vec::new();
    let right_bounds = bounds.max_x();
    let mut extreme_right_full: Option<Point> = None;
    let mut extreme_right: Option<Point> = None;
    for y in (lower_bound_sampling..=upper_bound_sampling).step_by(scan_frequency) {
        let line = Line::new((left_bounds, y as f64), (right_bounds, y as f64));

        let mut hits = intersections_for_line(paths, line);
        if hits.is_empty() {
            // Treat no hits as hits deep off the other side.
            left.push(Point::new(f64::INFINITY, y as f64));
            right.push(Point::new(-f64::INFINITY, y as f64));
        } else {
            hits.sort_by_key(|k| k.x.round() as i32);
            let mut first = hits.first().unwrap().clone(); // XXX: don't clone but own?
            let mut last = hits.last().unwrap().clone();
            if angle != 0.0 {
                first = Point::new(first.x - (y as f64 - skew_offset) * tan_angle, first.y);
                last = Point::new(last.x - (y as f64 - skew_offset) * tan_angle, last.y);
            }
            if lower_bound_reference <= y && y <= upper_bound_reference {
                left.push(first);
                right.push(last);

                extreme_left = extreme_left
                    .map(|l| if l.x < first.x { l } else { first.clone() })
                    .or(Some(first.clone()));
                extreme_right = extreme_right
                    .map(|r| if r.x > last.x { r } else { last.clone() })
                    .or(Some(last.clone()));
            }

            extreme_left_full = extreme_left_full
                .map(|l| if l.x < first.x { l } else { first.clone() })
                .or(Some(first.clone()));
            extreme_right_full = extreme_right_full
                .map(|r| if r.x > last.x { r } else { last.clone() })
                .or(Some(last.clone()));
        }
    }

    let extreme_left_full = extreme_left_full.unwrap();
    let extreme_left = extreme_left.unwrap();
    let extreme_right_full = extreme_right_full.unwrap();
    let extreme_right = extreme_right.unwrap();

    // Second pass: Cap the margin samples to a maximum depth from the outermost point in to get our depth cut-in.
    let depth = xheight * depth_cut / 100.0;
    let max_depth = extreme_left.x + depth;
    let min_depth = extreme_right.x - depth;
    left.iter_mut().for_each(|s| s.x = s.x.min(max_depth));
    right.iter_mut().for_each(|s| s.x = s.x.max(min_depth));

    // Third pass: Close open counterforms at 45 degrees.
    let dx_max = scan_frequency as f64;

    for i in 0..left.len() - 1 {
        if left[i + 1].x - left[i].x > dx_max {
            left[i + 1].x = left[i].x + dx_max;
        }
        if right[i + 1].x - right[i].x < -dx_max {
            right[i + 1].x = right[i].x - dx_max;
        }
    }
    for i in (0..left.len() - 1).rev() {
        if left[i].x - left[i + 1].x > dx_max {
            left[i].x = left[i + 1].x + dx_max;
        }
        if right[i].x - right[i + 1].x < -dx_max {
            right[i].x = right[i + 1].x - dx_max;
        }
    }

    left.insert(0, Point { x: extreme_left.x, y: bounds.min_y() });
    left.push(Point { x: extreme_left.x, y: bounds.max_y() });
    right.insert(0, Point { x: extreme_right.x, y: bounds.min_y() });
    right.push(Point { x: extreme_right.x, y: bounds.max_y() });

    (left, extreme_left_full, extreme_left, right, extreme_right_full, extreme_right)
}

fn intersections_for_line(paths: &BezPath, line: Line) -> Vec<Point> {
    paths
        .segments()
        .flat_map(|s| s.intersect_line(line).into_iter().map(move |h| s.eval(h.segment_t).round()))
        .collect()
}

fn draw_polygon(
    glyph: &Glyph,
    glyph_reference: &Glyph,
    glyphset: &Layer,
    angle: f64,
    xheight: f64,
    param_overshoot: f64,
    param_depth: f64,
    param_sample_frequency: usize,
) -> Glyph {
    let glyph = if glyph.outline.as_ref().map_or(false, |o| !o.components.is_empty()) {
        decompose(&glyph, glyphset)
    } else {
        Glyph::clone(&glyph)
    };
    let glyph_reference =
        if glyph_reference.outline.as_ref().map_or(false, |o| !o.components.is_empty()) {
            decompose(&glyph_reference, glyphset)
        } else {
            Glyph::clone(&glyph_reference)
        };

    println!("Drawing polygon for {}", glyph.name);

    let paths = path_for_glyph(&glyph).unwrap();
    let bounds = paths.bounding_box();
    let paths_reference = path_for_glyph(&glyph_reference).unwrap();
    let bounds_reference = paths_reference.bounding_box();

    let overshoot = xheight * param_overshoot / 100.0;
    let lower_bound_reference = (bounds_reference.min_y() - overshoot).round() as isize;
    let upper_bound_reference = (bounds_reference.max_y() + overshoot).round() as isize;

    let (samples_left, _, _, samples_right, _, _) = spacing_polygons(
        &paths,
        &bounds,
        lower_bound_reference,
        upper_bound_reference,
        angle,
        xheight,
        param_sample_frequency,
        param_depth,
    );

    draw_glyph_outer_outline_into_glyph(&glyph, (&samples_left, &samples_right))
}

fn draw_glyph_outer_outline_into_glyph(
    glyph: &Glyph,
    outlines: (&Vec<Point>, &Vec<Point>),
) -> Glyph {
    let mut builder = GlyphBuilder::new(glyph.name.clone(), GlifVersion::V2);
    if let Some(width) = glyph.advance_width() {
        builder.width(width).unwrap();
    }
    let mut outline_builder = OutlineBuilder::new();
    outline_builder.begin_path(None).unwrap();
    for left in outlines.0 {
        outline_builder
            .add_point((left.x as f32, left.y as f32), PointType::Line, false, None, None)
            .unwrap();
    }
    outline_builder.end_path().unwrap();
    outline_builder.begin_path(None).unwrap();
    for right in outlines.1 {
        outline_builder
            .add_point((right.x as f32, right.y as f32), PointType::Line, false, None, None)
            .unwrap();
    }
    outline_builder.end_path().unwrap();
    let (outline, identifiers) = outline_builder.finish().unwrap();
    builder.outline(outline, identifiers).unwrap();
    builder.finish().unwrap()
}

/// Decompose a (composite) glyph. Ignores incoming identifiers and libs.
fn decompose(glyph: &Glyph, glyphset: &Layer) -> Glyph {
    let mut decomposed_glyph = Glyph::new_named(glyph.name.clone());

    if let Some(outline) = &glyph.outline {
        let mut decomposed_contours = outline.contours.clone();

        let mut queue: std::collections::VecDeque<(&norad::Component, kurbo::Affine)> =
            std::collections::VecDeque::new();
        for component in &outline.components {
            let component_transformation = affine_norad_to_kurbo(&component.transform);
            queue.push_front((component, component_transformation));
            while let Some((component, component_transformation)) = queue.pop_front() {
                let new_glyph = glyphset.get_glyph(&component.base).expect(
                    format!(
                        "Glyph '{}': component '{}' points to non-existant glyph.",
                        glyph.name, component.base
                    )
                    .as_str(),
                );
                if let Some(new_outline) = &new_glyph.outline {
                    // decomposed_contours.extend(new_outline.contours.clone());
                    for new_contour in &new_outline.contours {
                        let mut new_decomposed_contour = norad::Contour::default();
                        for new_point in &new_contour.points {
                            let kurbo_point = component_transformation
                                * kurbo::Point::new(new_point.x as f64, new_point.y as f64);
                            new_decomposed_contour.points.push(norad::ContourPoint::new(
                                kurbo_point.x as f32,
                                kurbo_point.y as f32,
                                new_point.typ.clone(),
                                new_point.smooth,
                                new_point.name.clone(),
                                None,
                                None,
                            ))
                        }
                        decomposed_contours.push(new_decomposed_contour);
                    }

                    for new_component in new_outline.components.iter().rev() {
                        let new_component_transformation =
                            affine_norad_to_kurbo(&new_component.transform);
                        queue.push_front((
                            new_component,
                            component_transformation * new_component_transformation,
                        ));
                    }
                }
            }
        }

        decomposed_glyph.outline =
            Some(norad::Outline { contours: decomposed_contours, components: Vec::new() });
    }

    decomposed_glyph
}

// XXX: Copy-pasta! Make a separate "kurbo" feature that the "druid" feature depends on and move the From impl there.
fn affine_norad_to_kurbo(src: &norad::AffineTransform) -> kurbo::Affine {
    kurbo::Affine::new([
        src.x_scale as f64,
        src.xy_scale as f64,
        src.yx_scale as f64,
        src.y_scale as f64,
        src.x_offset as f64,
        src.y_offset as f64,
    ])
}

fn path_for_glyph(glyph: &Glyph) -> Option<BezPath> {
    /// An outline can have multiple contours, which correspond to subpaths
    fn add_contour(path: &mut BezPath, contour: &Contour) {
        let mut close: Option<&ContourPoint> = None;

        let start_idx = match contour.points.iter().position(|pt| pt.typ != PointType::OffCurve) {
            Some(idx) => idx,
            None => return,
        };

        let first = &contour.points[start_idx];
        path.move_to((first.x as f64, first.y as f64));
        if first.typ != PointType::Move {
            close = Some(first);
        }

        let mut controls = Vec::with_capacity(2);

        let mut add_curve = |to_point: Point, controls: &mut Vec<Point>| {
            match controls.as_slice() {
                &[] => path.line_to(to_point),
                &[a] => path.quad_to(a, to_point),
                &[a, b] => path.curve_to(a, b, to_point),
                _illegal => panic!("existence of second point implies first"),
            };
            controls.clear();
        };

        let mut idx = (start_idx + 1) % contour.points.len();
        while idx != start_idx {
            let next = &contour.points[idx];
            let point = Point::new(next.x as f64, next.y as f64);
            match next.typ {
                PointType::OffCurve => controls.push(point),
                PointType::Line => {
                    debug_assert!(controls.is_empty(), "line type cannot follow offcurve");
                    add_curve(point, &mut controls);
                }
                PointType::Curve => add_curve(point, &mut controls),
                PointType::QCurve => {
                    // XXX
                    // log::warn!("quadratic curves are currently ignored");
                    add_curve(point, &mut controls);
                }
                PointType::Move => debug_assert!(false, "illegal move point in path?"),
            }
            idx = (idx + 1) % contour.points.len();
        }

        if let Some(to_close) = close.take() {
            add_curve((to_close.x as f64, to_close.y as f64).into(), &mut controls);
        }
    }

    if let Some(outline) = glyph.outline.as_ref() {
        let mut path = BezPath::new();
        outline.contours.iter().for_each(|c| add_contour(&mut path, c));
        Some(path)
    } else {
        None
    }
}
