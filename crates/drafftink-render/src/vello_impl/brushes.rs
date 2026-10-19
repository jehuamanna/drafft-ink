//! Brush stroke geometry for freehand shapes: pressure, brush and nib
//! widths, smoothing, and cached outlines.

use super::*;

/// Pen stroke outlines by shape: (content hash, frame last used, filled layers).
type FreehandCache =
    std::collections::HashMap<drafftink_core::shapes::ShapeId, (u64, u64, Vec<(BezPath, Color)>)>;

/// Cached pen stroke outlines; building them is the costly part of drawing
/// strokes.
pub(super) struct StrokeCache {
    outlines: FreehandCache,
    /// Frames built, for evicting unused entries.
    frame: u64,
    /// Off while drawing the in-progress stroke, which is new every frame.
    pub(super) enabled: bool,
}

impl Default for StrokeCache {
    fn default() -> Self {
        Self {
            outlines: FreehandCache::new(),
            frame: 0,
            enabled: true,
        }
    }
}

impl StrokeCache {
    /// Start a frame. Forgets outlines of strokes not drawn for a while
    /// (deleted, edited, or long off-screen), checking only now and then.
    pub(super) fn next_frame(&mut self) {
        self.frame += 1;
        if self.frame % 120 == 0 {
            let frame = self.frame;
            self.outlines.retain(|_, (_, used, _)| frame - *used < 600);
        }
    }

    /// A stroke's filled outline layers at `zoom`.
    pub(super) fn layers(
        &mut self,
        freehand: &drafftink_core::shapes::Freehand,
        zoom: f64,
    ) -> &[(BezPath, Color)] {
        cached_freehand_layers(&mut self.outlines, freehand, zoom, self.frame)
    }
}

/// A run of same-coloured pen strokes, filled as one path: far fewer paths
/// for the GPU when many strokes are on screen. They all wind the same way,
/// so the non-zero fill draws them as before.
#[derive(Default)]
pub(super) struct StrokeBatch(Option<(Color, BezPath)>);

impl VelloRenderer {
    /// Add a pen stroke to the batch, filling the batch first if the colour
    /// changes. Returns false for shapes that must be drawn on their own.
    pub(super) fn batch_stroke(
        &mut self,
        batch: &mut StrokeBatch,
        shape: &Shape,
        is_selected: bool,
        transform: Affine,
    ) -> bool {
        let Shape::Freehand(freehand) = shape else {
            return false;
        };
        let layered = !freehand.closed
            && (freehand.has_pressure() || freehand.brush != drafftink_core::shapes::Brush::Pen);
        if !layered || is_selected || shape.rotation().abs() > 0.001 {
            return false;
        }
        let zoom = self.zoom;
        for (path, color) in self.strokes.layers(freehand, zoom) {
            match &mut batch.0 {
                Some((batch_color, batch_path)) if batch_color == color => {
                    batch_path.extend(path.iter());
                }
                _ => {
                    batch.fill(&mut self.scene, transform);
                    batch.0 = Some((*color, path.clone()));
                }
            }
        }
        true
    }
}

impl StrokeBatch {
    /// Fill the batched strokes, if any.
    pub(super) fn fill(&mut self, scene: &mut Scene, transform: Affine) {
        if let Some((color, path)) = self.0.take() {
            scene.fill(Fill::NonZero, transform, color, None, &path);
        }
    }
}

/// Stroke detail (screen pixels per canvas unit, at most 1) for a zoom
/// level. Zoomed out, strokes need fewer points to look the same; it changes
/// in steps of two so cached outlines aren't rebuilt on every zoom change.
pub(super) fn detail_for_zoom(zoom: f64) -> f64 {
    2f64.powf(zoom.log2().round()).min(1.0)
}

/// Outlines of a freehand stroke from the cache, rebuilt when the stroke (or
/// the zoom detail) changed since they were made.
fn cached_freehand_layers<'a>(
    cache: &'a mut FreehandCache,
    freehand: &drafftink_core::shapes::Freehand,
    zoom: f64,
    frame: u64,
) -> &'a [(BezPath, Color)] {
    use std::hash::{Hash, Hasher};

    let detail = detail_for_zoom(zoom);
    // Hashing a sample of the points keeps this cheap for long strokes;
    // moving, scaling or erasing changes the endpoints or the count anyway.
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let n = freehand.points.len();
    n.hash(&mut hasher);
    let stride = (n / 16).max(1);
    for i in (0..n).step_by(stride).chain(n.checked_sub(1)) {
        freehand.points[i].x.to_bits().hash(&mut hasher);
        freehand.points[i].y.to_bits().hash(&mut hasher);
        freehand.pressure_at(i).to_bits().hash(&mut hasher);
    }
    freehand.pressures.len().hash(&mut hasher);
    freehand.style.stroke_width.to_bits().hash(&mut hasher);
    freehand.style.opacity.to_bits().hash(&mut hasher);
    let c = freehand.style.stroke_color;
    (c.r, c.g, c.b, c.a).hash(&mut hasher);
    freehand.brush.hash(&mut hasher);
    freehand.nib_angle.to_bits().hash(&mut hasher);
    detail.to_bits().hash(&mut hasher);
    let key = hasher.finish();

    let entry = cache
        .entry(freehand.id())
        .or_insert_with(|| (key.wrapping_add(1), frame, Vec::new()));
    if entry.0 != key {
        *entry = (key, frame, freehand_layers(freehand, detail));
    }
    entry.1 = frame;
    &entry.2
}

/// Filled outlines (with their colours) that draw a freehand stroke with its
/// brush: variable width from pressure, direction and position along it.
///
/// `detail` is screen pixels per canvas unit (at most 1): sample spacing
/// grows as it shrinks, so zoomed-out strokes stay cheap to draw.
pub(super) fn freehand_layers(
    freehand: &drafftink_core::shapes::Freehand,
    detail: f64,
) -> Vec<(BezPath, Color)> {
    use drafftink_core::shapes::Brush as PenBrush;

    let n = freehand.points.len();
    if n < 2 {
        return Vec::new();
    }

    let style = &freehand.style;
    let sw = style.stroke_width;
    let color = style.stroke_with_opacity();
    let pressures: Vec<f64> = (0..n).map(|i| freehand.pressure_at(i)).collect();

    if freehand.brush == PenBrush::BroadNib {
        // Interpolate a spline through the input so fast strokes don't
        // show facets, then sweep the nib along it.
        let (points, pressures) = catmull_rom_resample(&freehand.points, &pressures, 1.0 / detail);
        let path = broad_nib_outline(&points, &pressures, sw, freehand.nib_angle);
        return vec![(path, color)];
    }

    // A single outline pinches or notches at sharp turns (v, n, w), so
    // each section between them becomes its own round-capped piece.
    let reach = (sw * 2.3).max(4.0);
    let mut sections = Vec::new();
    let mut start = 0;
    for end in sharp_corners(&freehand.points, reach)
        .into_iter()
        .chain([n - 1])
    {
        // Input points can be far apart on fast strokes; interpolate a
        // spline through them so curves don't render as facets.
        sections.push(catmull_rom_resample(
            &freehand.points[start..=end],
            &pressures[start..=end],
            1.5 / detail,
        ));
        start = end;
    }
    let total_length: f64 = sections
        .iter()
        .map(|(points, _)| polyline_length(points))
        .sum();

    // Width of each sample, from the brush.
    let widths: Vec<Vec<f64>> = {
        let mut travelled = 0.0;
        sections
            .iter()
            .map(|(points, pressures)| {
                let directions = smoothed_directions(points, 3);
                (0..points.len())
                    .map(|i| {
                        if i > 0 {
                            travelled += points[i].distance(points[i - 1]);
                        }
                        let ends = travelled.min(total_length - travelled);
                        brush_width(freehand.brush, sw, pressures[i], directions[i], ends)
                    })
                    .collect()
            })
            .collect()
    };

    // Translucent layers (scale of the width, share of the opacity).
    // All pieces of one layer wind the same way, so the non-zero fill
    // unions them without double-blending.
    let layers: &[(f64, f32)] = match freehand.brush {
        PenBrush::Soft => &[(1.0, 0.22), (0.8, 0.22), (0.6, 0.22), (0.4, 0.22)],
        PenBrush::Pencil => &[(1.0, 0.5), (0.7, 0.45)],
        _ => &[(1.0, 1.0)],
    };
    let mut filled = Vec::with_capacity(layers.len());
    for (layer, &(scale, alpha)) in layers.iter().enumerate() {
        let mut path = BezPath::new();
        for ((points, _), widths) in sections.iter().zip(&widths) {
            let widths: Vec<f64> = widths.iter().map(|w| w * scale).collect();
            if freehand.brush == PenBrush::Pencil {
                // Graphite: each layer wanders slightly off the line.
                let points = jitter(points, &widths, layer as f64);
                append_stroke_outline(&mut path, &points, &widths);
            } else {
                append_stroke_outline(&mut path, points, &widths);
            }
        }
        filled.push((path, color.multiply_alpha(alpha)));
    }
    filled
}

/// Indices of sharp turns (over 90°) in a stroke. Directions are measured
/// `reach` along the stroke on each side so sensor jitter doesn't count, and
/// only the sharpest point of each turn is returned.
pub(super) fn sharp_corners(points: &[Point], reach: f64) -> Vec<usize> {
    let mut corners = Vec::new();
    // Sharpest point of the turn in progress: (index, cosine of turn angle).
    let mut sharpest: Option<(usize, f64)> = None;
    for i in 1..points.len().saturating_sub(1) {
        let p = points[i];
        let back = points[..i].iter().rev().find(|q| q.distance(p) >= reach);
        let ahead = points[i + 1..].iter().find(|q| q.distance(p) >= reach);
        let cos = match (back, ahead) {
            (Some(&b), Some(&a)) => {
                let (incoming, outgoing) = (p - b, a - p);
                incoming.dot(outgoing) / (incoming.hypot() * outgoing.hypot())
            }
            _ => 1.0,
        };
        if cos < 0.0 {
            if sharpest.is_none_or(|(_, c)| cos < c) {
                sharpest = Some((i, cos));
            }
        } else if let Some((corner, _)) = sharpest.take() {
            corners.push(corner);
        }
    }
    corners.extend(sharpest.map(|(corner, _)| corner));
    corners
}

/// Append the filled outline of a variable-width stroke through `points`
/// (`widths` are full widths): left side forward, round end cap, right side
/// backward, round start cap.
pub(super) fn append_stroke_outline(path: &mut BezPath, points: &[Point], widths: &[f64]) {
    use kurbo::Vec2;

    let mut left_points: Vec<Point> = Vec::new();
    let mut right_points: Vec<Point> = Vec::new();
    // Centre point and half width of the first and last outline samples,
    // used to draw round caps.
    let mut first_cap: Option<(Point, f64)> = None;
    let mut last_cap: Option<(Point, f64)> = None;

    for i in 0..points.len() {
        let point = points[i];
        let half_width = widths[i] / 2.0;

        // Direction of travel: to the next point at the start, from the
        // previous one at the end, and across both in between.
        let prev = points[i.saturating_sub(1)];
        let next = points[(i + 1).min(points.len() - 1)];
        let dir = next - prev;
        let len = dir.hypot();
        if len < f64::EPSILON {
            continue;
        }

        // Perpendicular unit vector
        let perp = Vec2::new(-dir.y / len, dir.x / len);
        left_points.push(point + perp * half_width);
        right_points.push(point - perp * half_width);
        first_cap.get_or_insert((point, half_width));
        last_cap = Some((point, half_width));
    }

    let (Some(first_cap), Some(last_cap)) = (first_cap, last_cap) else {
        return;
    };

    // Sides are smoothed with quadratic curves through segment midpoints so
    // remaining unevenness doesn't show as corners.
    path.move_to(left_points[0]);
    append_smoothed(path, &left_points);
    append_round_cap(path, last_cap, left_points[left_points.len() - 1]);
    right_points.reverse();
    append_smoothed(path, &right_points);
    append_round_cap(path, first_cap, right_points[right_points.len() - 1]);
    path.close_path();
}

/// Full width of a brush stroke at one sample. `direction` is the unit
/// direction of travel and `ends` the distance to the nearer end of the stroke.
pub(super) fn brush_width(
    brush: drafftink_core::shapes::Brush,
    stroke_width: f64,
    pressure: f64,
    direction: kurbo::Vec2,
    ends: f64,
) -> f64 {
    use drafftink_core::shapes::{Brush, pressure_width};

    let p = pressure.clamp(0.0, 1.0);
    match brush {
        Brush::Pen | Brush::BroadNib => pressure_width(stroke_width, p),
        Brush::Marker => stroke_width * 2.3,
        Brush::PointedNib => {
            // The tines only spread when pulled towards the writer, so
            // upstrokes stay hairlines however hard you press.
            let hairline = (stroke_width * 0.3).max(0.6);
            let full = stroke_width * 4.0;
            let down = direction.y.max(0.0);
            hairline + (full - hairline) * p.powf(1.3) * down.powf(1.5)
        }
        Brush::BrushPen => {
            let full = stroke_width * 3.2 * (0.05 + 0.95 * p.powf(1.4));
            // Pointed entry and exit.
            let taper_length = (stroke_width * 6.0).max(8.0);
            let t = (ends / taper_length).clamp(0.0, 1.0);
            full * (0.08 + 0.92 * t * t * (3.0 - 2.0 * t))
        }
        Brush::Pencil => stroke_width * 1.2 * (0.55 + 0.45 * p),
        Brush::Soft => pressure_width(stroke_width * 1.8, p),
    }
}

/// Outline of a broad (chisel) nib swept along `points`: thick when moving
/// across the nib, thin along it, with crisp angled ends.
pub(super) fn broad_nib_outline(
    points: &[Point],
    pressures: &[f64],
    stroke_width: f64,
    angle: f64,
) -> BezPath {
    use kurbo::Vec2;

    // Nib edge direction in y-down coordinates (angle counter-clockwise
    // from horizontal, as on paper).
    let angle = angle.to_radians();
    let edge = Vec2::new(angle.cos(), -angle.sin());
    let across = Vec2::new(-edge.y, edge.x);
    let nib = |i: usize| {
        let width = stroke_width * 4.0 * (0.75 + 0.25 * pressures[i].clamp(0.0, 1.0));
        let thickness = (width * 0.08).max(0.5);
        (edge * (width / 2.0), across * (thickness / 2.0))
    };

    let mut path = BezPath::new();
    for i in 0..points.len().saturating_sub(1) {
        let mut corners = Vec::with_capacity(8);
        for j in [i, i + 1] {
            let (half_edge, half_across) = nib(j);
            for (e, a) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
                corners.push(points[j] + half_edge * e + half_across * a);
            }
        }
        // Every hull winds the same way, so the non-zero fill unions them.
        let hull = convex_hull(corners);
        if let Some((&first, rest)) = hull.split_first() {
            path.move_to(first);
            for &p in rest {
                path.line_to(p);
            }
            path.close_path();
        }
    }
    path
}

/// Convex hull (Andrew's monotone chain), counter-clockwise in y-up terms.
pub(super) fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    let cross = |o: Point, a: Point, b: Point| (a - o).cross(b - o);
    let mut hull: Vec<Point> = Vec::with_capacity(points.len() + 1);
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &Point>> = if pass == 0 {
            Box::new(points.iter())
        } else {
            Box::new(points.iter().rev())
        };
        for &p in iter {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

/// Total length of a polyline.
pub(super) fn polyline_length(points: &[Point]) -> f64 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Unit direction of travel at each point, measured `span` points either side
/// so small wobbles don't flip it.
pub(super) fn smoothed_directions(points: &[Point], span: usize) -> Vec<kurbo::Vec2> {
    let n = points.len();
    (0..n)
        .map(|i| {
            let d = points[(i + span).min(n - 1)] - points[i.saturating_sub(span)];
            let len = d.hypot();
            if len < f64::EPSILON {
                kurbo::Vec2::ZERO
            } else {
                d / len
            }
        })
        .collect()
}

/// Shift points sideways by a smooth, repeatable wobble (up to a quarter of
/// the local width), different for each `layer`.
pub(super) fn jitter(points: &[Point], widths: &[f64], layer: f64) -> Vec<Point> {
    let directions = smoothed_directions(points, 2);
    points
        .iter()
        .zip(widths)
        .zip(directions)
        .enumerate()
        .map(|(i, ((&p, &w), d))| {
            let t = i as f64;
            let wobble =
                0.6 * (t * 0.37 + layer * 1.7).sin() + 0.4 * (t * 0.11 + layer * 0.9).sin();
            p + kurbo::Vec2::new(-d.y, d.x) * (wobble * w * 0.25)
        })
        .collect()
}

/// Sample a Catmull-Rom spline through `pts` (and linearly interpolated
/// pressures) at most `step` apart.
pub(super) fn catmull_rom_resample(
    pts: &[Point],
    pressure: &[f64],
    step: f64,
) -> (Vec<Point>, Vec<f64>) {
    let n = pts.len();
    let mut points = vec![pts[0]];
    let mut pressures = vec![pressure[0]];

    for i in 0..n - 1 {
        let p0 = pts[i.saturating_sub(1)].to_vec2();
        let p1 = pts[i].to_vec2();
        let p2 = pts[i + 1].to_vec2();
        let p3 = pts[(i + 2).min(n - 1)].to_vec2();
        let (w1, w2) = (pressure[i], pressure[i + 1]);

        let samples = ((p2 - p1).hypot() / step).ceil().max(1.0) as usize;
        for j in 1..=samples {
            let t = j as f64 / samples as f64;
            let (t2, t3) = (t * t, t * t * t);
            let p = 0.5
                * (2.0 * p1
                    + (p2 - p0) * t
                    + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                    + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3);
            points.push(p.to_point());
            pressures.push(w1 + (w2 - w1) * t);
        }
    }
    (points, pressures)
}

/// Continue `path` (currently at `points[0]`) through `points`, using the
/// points as quadratic control points and segment midpoints as on-curve points.
pub(super) fn append_smoothed(path: &mut BezPath, points: &[Point]) {
    let n = points.len();
    if n < 3 {
        if let Some(&last) = points.get(1) {
            path.line_to(last);
        }
        return;
    }
    for i in 1..n - 1 {
        path.quad_to(points[i], points[i].midpoint(points[i + 1]));
    }
    path.line_to(points[n - 1]);
}

/// Append a half circle around `center` from `from` (on the circle) to the
/// opposite point, turning clockwise in y-down coordinates.
pub(super) fn append_round_cap(path: &mut BezPath, (center, radius): (Point, f64), from: Point) {
    if radius <= f64::EPSILON {
        return;
    }
    let start_angle = (from - center).atan2();
    let arc = kurbo::Arc::new(
        center,
        kurbo::Vec2::new(radius, radius),
        start_angle,
        -std::f64::consts::PI,
        0.0,
    );
    path.extend(arc.append_iter(0.1));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convex_hull_winds_consistently() {
        let points = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        let hull = convex_hull(points);
        assert_eq!(hull.len(), 4);
        assert!(!hull.contains(&Point::new(1.0, 1.0)));
        let area: f64 = (0..hull.len())
            .map(|i| {
                hull[i]
                    .to_vec2()
                    .cross(hull[(i + 1) % hull.len()].to_vec2())
            })
            .sum();
        assert!(area > 0.0);
    }

    #[test]
    fn test_pointed_nib_shades_downstrokes_only() {
        use drafftink_core::shapes::Brush;
        let up = brush_width(
            Brush::PointedNib,
            2.0,
            1.0,
            kurbo::Vec2::new(0.0, -1.0),
            50.0,
        );
        let down = brush_width(
            Brush::PointedNib,
            2.0,
            1.0,
            kurbo::Vec2::new(0.0, 1.0),
            50.0,
        );
        let light = brush_width(
            Brush::PointedNib,
            2.0,
            0.1,
            kurbo::Vec2::new(0.0, 1.0),
            50.0,
        );
        assert!(down > up * 5.0);
        assert!(light < down / 4.0);
    }

    #[test]
    fn test_brush_pen_tapers_at_ends() {
        use drafftink_core::shapes::Brush;
        let dir = kurbo::Vec2::new(1.0, 0.0);
        let tip = brush_width(Brush::BrushPen, 2.0, 1.0, dir, 0.0);
        let middle = brush_width(Brush::BrushPen, 2.0, 1.0, dir, 100.0);
        assert!(tip < middle * 0.1);
    }

    #[test]
    fn test_broad_nib_thick_across_thin_along() {
        // Nib at 0°: horizontal moves run along the edge (thin), vertical
        // moves across it (thick).
        let p = [1.0, 1.0];
        let along = broad_nib_outline(&[Point::new(0.0, 0.0), Point::new(10.0, 0.0)], &p, 2.0, 0.0);
        let across =
            broad_nib_outline(&[Point::new(0.0, 0.0), Point::new(0.0, 10.0)], &p, 2.0, 0.0);
        assert!(along.bounding_box().height() < 1.0);
        assert!(across.bounding_box().width() > 5.0);
    }
}
