use kurbo::{BezPath, Line, ParamCurve, Point, Shape};
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

        let default_layer = ufo.get_default_layer().unwrap();
        let decomposed_glyphs: Vec<Glyph> = default_layer
            .iter_contents()
            .map(|glyph| {
                let glyph = if glyph.outline.as_ref().map_or(false, |o| !o.components.is_empty()) {
                    decompose(&glyph, default_layer)
                } else {
                    Glyph::clone(&glyph)
                };
                let paths = path_for_glyph(&glyph).unwrap();
                let outlines = glyph_outer_outline(&paths, 5);
                draw_glyph_outer_outline_into_glyph(&glyph, outlines)
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

fn draw_glyph_outer_outline_into_glyph(glyph: &Glyph, outlines: (Vec<Point>, Vec<Point>)) -> Glyph {
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

fn glyph_outer_outline(paths: &BezPath, scan_frequency: usize) -> (Vec<Point>, Vec<Point>) {
    let bounds = paths.bounding_box();
    let mut left = Vec::new();
    let mut right = Vec::new();
    for y in
        (bounds.min_y().round() as usize..bounds.max_y().round() as usize).step_by(scan_frequency)
    {
        let line = Line::new((bounds.min_x(), y as f64), (bounds.max_x(), y as f64));
        let mut hits = intersections_for_line(paths, line);
        hits.sort_by_key(|k| k.x.round() as i32);
        if let Some(first) = hits.first() {
            left.push(first.clone());
        }
        if let Some(last) = hits.last() {
            right.push(last.clone());
        }
    }
    (left, right)
}

fn intersections_for_line(paths: &BezPath, line: Line) -> Vec<Point> {
    paths
        .segments()
        .flat_map(|s| s.intersect_line(line).into_iter().map(move |h| s.eval(h.segment_t).round()))
        .collect()
}

// fn calculate_spacing(
//     glyph: &Glyph,
//     glyph_ref: &Glyph,
//     glyphset: &Layer,
//     angle: f32,
//     factor: f32,
//     param_area: u32,
//     param_depth: u32,
//     param_over: u32,
//     param_freq: u32,
//     tabular_width: Option<u32>,
//     upm: u16,
//     xheight: u32,
// ) -> Option<(i32, i32, u32)> {
//     let glyph_decomposed = decompose(glyph, glyphset);
//     let glyph_paths = path_for_glyph(&glyph_decomposed);
//     if glyph_paths.is_none() {
//         return None;
//     }
//     let glyph_paths = glyph_paths.unwrap();
//     let (bounds_ymin, bounds_ymax) = {
//         let bounds = glyph_paths.bounding_box();
//         (bounds.min_y(), bounds.max_y())
//     };

//     // TODO: handle this outside the function and just pass in the ref bounds.
//     let glyph_ref_decomposed = decompose(glyph_ref, glyphset);
//     let glyph_ref_paths = path_for_glyph(&glyph_ref_decomposed);
//     let (ref_bounds_ymin, ref_bounds_ymax) = match glyph_ref_paths {
//         Some(p) => {
//             // Use reference glyph lower and upper bounds.
//             let bounds = p.bounding_box();
//             (bounds.min_y(), bounds.max_y())
//         }
//         None => {
//             // Use glyph's own lower and upper bounds.
//             let bounds = glyph_paths.bounding_box();
//             (bounds.min_y(), bounds.max_y())
//         }
//     };

//     // The reference glyph provides the lower and upper bound of the vertical
//     // zone to use for spacing. Overshoot lets us measure a bit above and below.
//     let overshoot = xheight * param_over / 100;
//     let ref_ymin: i32 = ref_bounds_ymin.round() as i32 - overshoot as i32;
//     let ref_ymax: i32 = ref_bounds_ymax.round() as i32 + overshoot as i32;

//     // Feel out the outer outline of the glyph and deslant if it's slanted.
//     let (mut margins_left_full, mut margins_right_full) = margin_list(glyph_paths, param_freq);
//     if angle != 0.0 {
//         margins_left_full = deslant(margins_left_full, angle, xheight);
//         margins_right_full = deslant(margins_right_full, angle, xheight);
//     }

//     // Determine the extreme outer left and right points on the outline as
//     // the line from which to feel into the glyph.

//     //

//     Some((0, 0, 0))
// }

// fn margin_list(
//     glyph: &Glyph,
//     glyph_ymax: i32,
//     param_freq: u32,
// ) -> (Vec<(f32, f32)>, Vec<(f32, f32)>) {
// }

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
