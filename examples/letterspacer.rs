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

        let units_per_em: f64 =
            ufo.font_info.as_ref().map_or(1000.0, |info| match info.units_per_em {
                Some(a) => a.get(),
                None => 1000.0,
            });
        let angle: f64 = ufo.font_info.as_ref().map_or(0.0, |info| match info.italic_angle {
            Some(a) => -a.get(),
            None => 0.0,
        });
        let xheight: f64 = ufo.font_info.as_ref().map_or(0.0, |info| match info.x_height {
            Some(a) => a.get(),
            None => 0.0,
        });
        let param_area: f64 = 400.0;
        let param_depth: f64 = 15.0;
        let param_overshoot: f64 = 0.0;
        let overshoot = xheight * param_overshoot / 100.0;
        let param_sample_frequency: usize = 5;

        // TODO: fetch actual reference glyph.
        let default_layer = ufo.get_default_layer().unwrap();
        let mut background_glyphs: Vec<Glyph> = Vec::new();

        let reference_uppercase = default_layer.get_glyph("H").expect("Need an 'H'.");
        let reference_uppercase_factor = 1.25;
        let reference_lowercase = default_layer.get_glyph("x").expect("Need an 'x'.");
        let reference_lowercase_factor = 1.0;

        for glyph in default_layer.iter_contents() {
            // TODO: Write `impl From<Glyph> for BezPath` which decomposes implicitly? Needs glyphset parameter though.
            let glyph = match &glyph.outline {
                Some(outline) => match (outline.components.is_empty(), outline.contours.is_empty())
                {
                    (true, true) => continue,
                    (false, _) => decompose(&glyph, &default_layer),
                    _ => Glyph::clone(&glyph),
                },
                _ => continue,
            };

            let (glyph_reference, factor) = match determine_unicode(&glyph, &default_layer) {
                Some(u) => {
                    if u.is_uppercase() {
                        (reference_uppercase, reference_uppercase_factor)
                    } else if u.is_lowercase() {
                        (reference_lowercase, reference_lowercase_factor)
                    } else {
                        // TODO: introduce proper configuration so we can space more than upper- and lowercase glyphs.
                        // also handle .sc
                        continue;
                    }
                }
                _ => continue,
            };

            let paths = path_for_glyph(&glyph).unwrap();
            let bounds = paths.bounding_box();
            let paths_reference = path_for_glyph(&glyph_reference).unwrap();
            let bounds_reference = paths_reference.bounding_box();

            let lower_bound_reference = (bounds_reference.min_y() - overshoot).round() as isize;
            let upper_bound_reference = (bounds_reference.max_y() + overshoot).round() as isize;

            let (left, extreme_left_full, extreme_left, right, extreme_right_full, extreme_right) =
                spacing_polygons(
                    &paths,
                    &bounds,
                    lower_bound_reference,
                    upper_bound_reference,
                    angle,
                    xheight,
                    param_sample_frequency,
                    param_depth,
                );

            let background_glyph = draw_glyph_outer_outline_into_glyph(&glyph, (&left, &right));
            background_glyphs.push(background_glyph);

            // Difference between extreme points full and in zone.
            let distance_left = (extreme_left.x - extreme_left_full.x).ceil();
            let distance_right = (extreme_right_full.x - extreme_right.x).ceil();

            let new_left = (-distance_left
                + calculate_sidebearing_value(
                    factor,
                    lower_bound_reference as f64,
                    upper_bound_reference as f64,
                    param_area,
                    &left,
                    units_per_em,
                    xheight,
                ))
            .ceil();
            let new_right = (-distance_right
                + calculate_sidebearing_value(
                    factor,
                    lower_bound_reference as f64,
                    upper_bound_reference as f64,
                    param_area,
                    &right,
                    units_per_em,
                    xheight,
                ))
            .ceil();

            // XXX: Merriweather lots of suspicious same-y values
            println!(
                "Glyph {}, reference {}, new left {}, new right {}",
                glyph.name, glyph_reference.name, new_left, new_right
            );
        }

        // Write out background layer.
        // TODO: write Ufo::new_layer method.
        let mut background_layer = norad::LayerInfo {
            name: "public.background".into(),
            path: std::path::PathBuf::from("glyphs.background"),
            layer: Layer::default(),
        };
        for glyph in background_glyphs {
            background_layer.layer.insert_glyph(glyph)
        }
        ufo.layers.push(background_layer);

        ufo.meta.creator = "org.linebender.norad".into();
        let output_path = std::path::PathBuf::from(&arg);
        ufo.save(std::path::PathBuf::from("/tmp").join(output_path.file_name().unwrap())).unwrap();
    }
}

fn determine_unicode(glyph: &Glyph, glyphset: &Layer) -> Option<char> {
    if let Some(codepoints) = &glyph.codepoints {
        if let Some(codepoint) = codepoints.first() {
            return Some(*codepoint);
        }
    }

    let base_name = glyph.name.split(".").nth(0).unwrap();
    if let Some(base_glyph) = glyphset.get_glyph(base_name) {
        if let Some(codepoints) = &base_glyph.codepoints {
            if let Some(codepoint) = codepoints.first() {
                return Some(*codepoint);
            }
        }
    }

    None
}

fn calculate_sidebearing_value(
    factor: f64,
    lower_bound_reference: f64,
    upper_bound_reference: f64,
    param_area: f64,
    polygon: &Vec<Point>,
    units_per_em: f64,
    xheight: f64,
) -> f64 {
    let amplitude_y = lower_bound_reference - upper_bound_reference;
    let area_upm = param_area * (units_per_em / 1000.0).powi(2);
    let white_area = area_upm * factor * 100.0;
    let prop_area = (amplitude_y * white_area) / xheight;
    let valor = prop_area - area(&polygon);
    valor / amplitude_y
}

fn area(points: &Vec<Point>) -> f64 {
    // https://mathopenref.com/coordpolygonarea2.html
    let mut s = 0.0;
    for i in 0..points.len() {
        let (prev, next) = (points[i], points[i % points.len()]);
        s += prev.x * next.y - next.x * prev.y
    }
    s.abs() / 2.0
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
        let in_reference_zone = lower_bound_reference <= y && y <= upper_bound_reference;

        let mut hits = intersections_for_line(paths, line);
        if hits.is_empty() {
            if in_reference_zone {
                // Treat no hits as hits deep off the other side.
                left.push(Point::new(f64::INFINITY, y as f64));
                right.push(Point::new(-f64::INFINITY, y as f64));
            }
        } else {
            hits.sort_by_key(|k| k.x.round() as i32);
            let mut first = hits.first().unwrap().clone(); // XXX: don't clone but own?
            let mut last = hits.last().unwrap().clone();
            if angle != 0.0 {
                first = Point::new(first.x - (y as f64 - skew_offset) * tan_angle, first.y);
                last = Point::new(last.x - (y as f64 - skew_offset) * tan_angle, last.y);
            }
            if in_reference_zone {
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

    left.insert(0, Point { x: extreme_left.x, y: lower_bound_reference as f64 });
    left.push(Point { x: extreme_left.x, y: upper_bound_reference as f64 });
    right.insert(0, Point { x: extreme_right.x, y: lower_bound_reference as f64 });
    right.push(Point { x: extreme_right.x, y: upper_bound_reference as f64 });

    (left, extreme_left_full, extreme_left, right, extreme_right_full, extreme_right)
}

fn intersections_for_line(paths: &BezPath, line: Line) -> Vec<Point> {
    paths
        .segments()
        .flat_map(|s| s.intersect_line(line).into_iter().map(move |h| s.eval(h.segment_t).round()))
        .collect()
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
