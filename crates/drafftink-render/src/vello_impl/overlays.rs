//! Overlays drawn by this fork: the brush outline and writing guides.

use super::*;

impl VelloRenderer {
    /// Render the brush outline: a thin dark ring with a light halo so it
    /// stays visible on any background.
    pub(super) fn render_brush_cursor(
        &mut self,
        pos: Point,
        diameter: f64,
        opacity: f64,
        transform: Affine,
    ) {
        let circle = kurbo::Circle::new(pos, (diameter / 2.0).max(1.0 / self.zoom));
        let alpha = |a: f64| (a * opacity.clamp(0.0, 1.0) * 255.0) as u8;
        self.scene.stroke(
            &Stroke::new(3.0 / self.zoom),
            transform,
            Color::from_rgba8(255, 255, 255, alpha(0.7)),
            None,
            &circle,
        );
        self.scene.stroke(
            &Stroke::new(1.0 / self.zoom),
            transform,
            Color::from_rgba8(40, 40, 40, alpha(0.8)),
            None,
            &circle,
        );
    }

    /// Render handwriting (3-line) or calligraphy (4-line) practice guides:
    /// rows of ascender, waist (x-height), base and descender lines, with
    /// optional slant lines.
    pub(super) fn render_writing_guides(
        &mut self,
        viewport: Rect,
        transform: Affine,
        guides: &crate::WritingGuides,
        four_lines: bool,
    ) {
        let inv = transform.inverse();
        let top_left = inv * Point::new(viewport.x0, viewport.y0);
        let bottom_right = inv * Point::new(viewport.x1, viewport.y1);
        let x_height = guides.x_height.max(1.0);
        let ascender = guides.ascender.max(0.0) * x_height;
        let descender = guides.descender.max(0.0) * x_height;
        let row = guides.row_height();
        // Too dense to be useful (or to draw quickly) when zoomed far out.
        if row * self.zoom < 6.0 {
            return;
        }

        // Lines stay hairline-thin on screen at any zoom.
        let px = 1.0 / self.zoom;
        let base_color = Color::from_rgba8(70, 110, 200, 170);
        let waist_color = Color::from_rgba8(70, 110, 200, 120);
        let outer_color = Color::from_rgba8(220, 90, 90, 110);
        let band_color = Color::from_rgba8(70, 110, 200, 14);
        let slant_color = Color::from_rgba8(150, 150, 150, 70);

        let (x0, x1) = (top_left.x, bottom_right.x);
        let first = ((top_left.y - descender) / row).floor() as i64;
        let last = ((bottom_right.y + x_height + ascender) / row).ceil() as i64;

        let mut bases = BezPath::new();
        let mut waists = BezPath::new();
        let mut outers = BezPath::new();
        let mut slants = BezPath::new();
        // Slant lines are one x-height apart; hide them once they'd be a
        // blur of lines only a few pixels apart.
        let slant = if x_height * self.zoom >= 8.0 {
            guides.slant.clamp(-60.0, 60.0).to_radians().tan()
        } else {
            0.0
        };
        for k in first..=last {
            let base = k as f64 * row;
            let waist = base - x_height;
            let top = waist - ascender;
            let bottom = base + descender;

            let band = Rect::new(x0, waist, x1, base);
            self.scene
                .fill(Fill::NonZero, transform, band_color, None, &band);

            bases.move_to(Point::new(x0, base));
            bases.line_to(Point::new(x1, base));
            waists.move_to(Point::new(x0, waist));
            waists.line_to(Point::new(x1, waist));
            if ascender > 0.0 {
                outers.move_to(Point::new(x0, top));
                outers.line_to(Point::new(x1, top));
            }
            if four_lines && descender > 0.0 {
                outers.move_to(Point::new(x0, bottom));
                outers.line_to(Point::new(x1, bottom));
            }

            // Slant lines across the whole row, spaced one x-height apart
            // and anchored to the canvas so they don't drift when panning.
            if slant != 0.0 {
                let lean = (bottom - top) * slant;
                let spacing = x_height;
                let mut x = ((x0 - lean.max(0.0)) / spacing).floor() * spacing;
                while x <= x1 - lean.min(0.0) {
                    slants.move_to(Point::new(x, bottom));
                    slants.line_to(Point::new(x + lean, top));
                    x += spacing;
                }
            }
        }

        self.scene
            .stroke(&Stroke::new(1.3 * px), transform, base_color, None, &bases);
        // Handwriting paper has a dashed midline; calligraphy a solid one.
        let waist_stroke = if four_lines {
            Stroke::new(px)
        } else {
            Stroke::new(px).with_dashes(0.0, [6.0 * px, 4.0 * px])
        };
        self.scene
            .stroke(&waist_stroke, transform, waist_color, None, &waists);
        self.scene
            .stroke(&Stroke::new(px), transform, outer_color, None, &outers);
        self.scene
            .stroke(&Stroke::new(px), transform, slant_color, None, &slants);
    }
}
