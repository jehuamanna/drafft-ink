//! Practice sheets: every letter, number and symbol written with each brush
//! at each stroke width, laid out on handwriting or calligraphy guide lines,
//! with an empty row after each line to copy it.
//!
//! Letters come from two Hershey stroke fonts (see `assets/hershey`), drawn
//! as pen strokes so every brush renders them as it would your writing.

use crate::canvas::CanvasDocument;
use crate::shapes::{Brush, Freehand, SerializableColor, Shape, Text};
use kurbo::Point;

/// Which practice sheet to build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeSheet {
    /// Print letters on three-line handwriting paper.
    Handwriting,
    /// Cursive script on four-line calligraphy paper, slanted.
    Calligraphy,
}

/// Guide line proportions a sheet is laid out on. Show the matching writing
/// guides grid with these values so the letters sit on the lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetGuides {
    pub x_height: f64,
    pub ascender: f64,
    pub descender: f64,
    pub row_gap: f64,
    /// Degrees from vertical.
    pub slant: f64,
}

impl SheetGuides {
    /// Distance between baselines (same formula as the guides grid).
    pub fn row_height(&self) -> f64 {
        self.x_height.max(1.0) * (self.ascender + 1.0 + self.descender + self.row_gap).max(0.5)
    }
}

/// Stroke widths shown, matching the toolbar presets.
pub const WIDTHS: [f64; 7] = [1.0, 1.25, 1.5, 1.75, 2.0, 4.0, 8.0];

const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const NUMBERS: &str = "0123456789";
const SYMBOLS: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

/// Rows of each block: the title, then each character line followed by an
/// empty practice row, then a spacer.
const BLOCK_ROWS: usize = 10;
/// Rows above the first block for the sheet title and instructions.
const HEADER_ROWS: usize = 3;

const INK: SerializableColor = SerializableColor {
    r: 30,
    g: 41,
    b: 59,
    a: 255,
};
const LABEL: SerializableColor = SerializableColor {
    r: 100,
    g: 116,
    b: 139,
    a: 255,
};

impl PracticeSheet {
    pub fn title(self) -> &'static str {
        match self {
            PracticeSheet::Handwriting => "Handwriting practice",
            PracticeSheet::Calligraphy => "Calligraphy practice",
        }
    }

    /// Font data and its metrics in font units: (data, x-height,
    /// ascender above the x-height, descender below the baseline).
    fn font(self) -> (&'static str, f64, f64, f64) {
        match self {
            PracticeSheet::Handwriting => {
                (include_str!("../assets/hershey/romans.jhf"), 14.0, 7.0, 7.0)
            }
            PracticeSheet::Calligraphy => (
                include_str!("../assets/hershey/scripts.jhf"),
                9.0,
                12.0,
                12.0,
            ),
        }
    }

    /// Canvas units per font unit.
    fn scale(self) -> f64 {
        match self {
            PracticeSheet::Handwriting => 3.0,
            PracticeSheet::Calligraphy => 4.5,
        }
    }

    /// The guide lines this sheet is laid out on.
    pub fn guides(self) -> SheetGuides {
        let (_, x_height, ascender, descender) = self.font();
        SheetGuides {
            x_height: x_height * self.scale(),
            ascender: ascender / x_height,
            descender: descender / x_height,
            row_gap: 0.5,
            slant: match self {
                PracticeSheet::Handwriting => 0.0,
                PracticeSheet::Calligraphy => 12.0,
            },
        }
    }

    /// Build the sheet: one block per brush (down) and stroke width (across).
    pub fn build(self) -> CanvasDocument {
        let (data, ..) = self.font();
        let glyphs = parse_font(data);
        let guides = self.guides();
        let row = guides.row_height();
        let x_height = guides.x_height;
        let scale = self.scale();
        // Letters are spaced apart so each can be traced on its own.
        let spacing = x_height * 0.6;
        let label_width = x_height * 5.0;

        let line_width = |text: &str| -> f64 {
            text.chars()
                .filter_map(|c| glyph(&glyphs, c))
                .map(|g| (g.right - g.left) * scale + spacing)
                .sum()
        };
        let block_width = label_width
            + [UPPERCASE, LOWERCASE, NUMBERS, SYMBOLS]
                .iter()
                .map(|t| line_width(t))
                .fold(0.0, f64::max)
            + x_height * 4.0;

        let mut doc = CanvasDocument::new();
        doc.name = self.title().to_string();
        let left = x_height * 2.0;

        // Title and instructions.
        let mut add_text =
            |doc: &mut CanvasDocument, x: f64, baseline: f64, size: f64, text: &str| {
                let mut label = Text::new(Point::new(x, baseline - size * 1.25), text.to_string())
                    .with_font_size(size);
                label.style.stroke_color = LABEL;
                doc.add_shape(Shape::Text(label));
            };
        add_text(&mut doc, left, row, x_height * 1.2, self.title());
        add_text(
            &mut doc,
            left,
            row * 2.0,
            x_height * 0.5,
            "Each block is one brush (down) at one stroke width (across): uppercase, lowercase, \
             numbers and symbols,\neach with an empty row below to copy it. Pick the same \
             brush and width in the pen panel and write along the lines.",
        );

        for (b, brush) in Brush::ALL.into_iter().enumerate() {
            for (w, width) in WIDTHS.into_iter().enumerate() {
                let x0 = left + w as f64 * block_width;
                let first_row = HEADER_ROWS + b * BLOCK_ROWS;
                let baseline = |r: usize| (first_row + r) as f64 * row;

                add_text(
                    &mut doc,
                    x0,
                    baseline(0),
                    x_height * 0.7,
                    &format!("{} · {width} px", brush.name()),
                );
                let lines = [
                    ("Uppercase", UPPERCASE),
                    ("Lowercase", LOWERCASE),
                    ("Numbers", NUMBERS),
                    ("Symbols", SYMBOLS),
                ];
                for (l, (name, text)) in lines.into_iter().enumerate() {
                    let base = baseline(1 + l * 2);
                    add_text(&mut doc, x0, base, x_height * 0.4, name);
                    let mut cursor = x0 + label_width;
                    for c in text.chars() {
                        let Some(g) = glyph(&glyphs, c) else {
                            continue;
                        };
                        for stroke in &g.strokes {
                            let points: Vec<Point> = stroke
                                .iter()
                                .map(|p| {
                                    Point::new(
                                        cursor + (p.x - g.left) * scale,
                                        // Font baseline is at y = 9.
                                        base + (p.y - 9.0) * scale,
                                    )
                                })
                                .collect();
                            if let Some(shape) = pen_stroke(&points, brush, width) {
                                doc.add_shape(shape);
                            }
                        }
                        cursor += (g.right - g.left) * scale + spacing;
                    }
                }
            }
        }
        doc
    }
}

/// One Hershey glyph, in font units (y down, baseline at 9).
struct Glyph {
    left: f64,
    right: f64,
    strokes: Vec<Vec<Point>>,
}

/// Parse `.jhf` lines numbered by character code (32 = space).
fn parse_font(data: &str) -> Vec<Glyph> {
    data.lines()
        .filter(|line| line.len() >= 10)
        .map(|line| {
            let coord = |c: u8| f64::from(c) - f64::from(b'R');
            let bytes = &line.as_bytes()[8..];
            let mut strokes = vec![Vec::new()];
            for pair in bytes[2..].chunks_exact(2) {
                if pair == b" R" {
                    strokes.push(Vec::new());
                } else {
                    strokes
                        .last_mut()
                        .unwrap()
                        .push(Point::new(coord(pair[0]), coord(pair[1])));
                }
            }
            strokes.retain(|s| !s.is_empty());
            Glyph {
                left: coord(bytes[0]),
                right: coord(bytes[1]),
                strokes,
            }
        })
        .collect()
}

fn glyph(glyphs: &[Glyph], c: char) -> Option<&Glyph> {
    glyphs.get((c as usize).checked_sub(32)?)
}

/// A pen stroke through a glyph's polyline: curves smoothed, corners kept,
/// and pressure easing in and out like a written stroke.
fn pen_stroke(polyline: &[Point], brush: Brush, width: f64) -> Option<Shape> {
    // Sparse enough to stay light; the renderer smooths between points.
    let points = smooth(polyline, 4.0);
    if points.len() < 2 {
        // A dot (e.g. in "i" or ":"): a tiny stroke so it still shows.
        let p = *polyline.first()?;
        return pen_stroke(&[p, p + kurbo::Vec2::new(0.5, 0.5)], brush, width);
    }
    let lengths: Vec<f64> = std::iter::once(0.0)
        .chain(points.windows(2).scan(0.0, |total, w| {
            *total += w[0].distance(w[1]);
            Some(*total)
        }))
        .collect();
    let total = lengths.last().copied().unwrap_or(0.0).max(f64::EPSILON);
    let pressures = lengths
        .iter()
        .map(|d| 0.45 + 0.55 * (std::f64::consts::PI * d / total).sin().max(0.0).sqrt())
        .collect();

    let mut stroke = Freehand::from_points_with_pressure(points, pressures);
    stroke.brush = brush;
    stroke.style.stroke_width = width;
    stroke.style.stroke_color = INK;
    Some(Shape::Freehand(stroke))
}

/// Interpolate a Catmull-Rom spline through `points` (at most `step` apart),
/// split at sharp corners so they stay sharp.
fn smooth(points: &[Point], step: f64) -> Vec<Point> {
    let mut points = points.to_vec();
    points.dedup();
    let n = points.len();
    if n < 3 {
        return if n == 2 {
            linear(&points, step)
        } else {
            points
        };
    }
    // Corners: turns sharper than 50 degrees.
    let mut cuts = vec![0];
    for i in 1..n - 1 {
        let (a, b) = (points[i] - points[i - 1], points[i + 1] - points[i]);
        if a.dot(b) < a.hypot() * b.hypot() * 50f64.to_radians().cos() {
            cuts.push(i);
        }
    }
    cuts.push(n - 1);

    let mut out = vec![points[0]];
    for piece in cuts.windows(2) {
        let pts = &points[piece[0]..=piece[1]];
        let m = pts.len();
        for i in 0..m - 1 {
            let p0 = pts[i.saturating_sub(1)].to_vec2();
            let p1 = pts[i].to_vec2();
            let p2 = pts[i + 1].to_vec2();
            let p3 = pts[(i + 2).min(m - 1)].to_vec2();
            let samples = ((p2 - p1).hypot() / step).ceil().max(1.0) as usize;
            for j in 1..=samples {
                let t = j as f64 / samples as f64;
                let (t2, t3) = (t * t, t * t * t);
                let p = 0.5
                    * (2.0 * p1
                        + (p2 - p0) * t
                        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3);
                out.push(p.to_point());
            }
        }
    }
    out
}

/// Points along a straight segment at most `step` apart.
fn linear(points: &[Point], step: f64) -> Vec<Point> {
    let (a, b) = (points[0], points[1]);
    let samples = (a.distance(b) / step).ceil().max(1.0) as usize;
    (0..=samples)
        .map(|j| a.lerp(b, j as f64 / samples as f64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fonts_cover_printable_ascii() {
        for sheet in [PracticeSheet::Handwriting, PracticeSheet::Calligraphy] {
            let glyphs = parse_font(sheet.font().0);
            assert_eq!(glyphs.len(), 95);
            for c in [UPPERCASE, LOWERCASE, NUMBERS, SYMBOLS].concat().chars() {
                let g = glyph(&glyphs, c).unwrap();
                assert!(!g.strokes.is_empty(), "{c:?} has no strokes");
                assert!(g.right > g.left);
            }
        }
    }

    #[test]
    fn test_sheet_has_every_combination() {
        let doc = PracticeSheet::Handwriting.build();
        let strokes: Vec<_> = doc
            .shapes_ordered()
            .filter_map(|s| match s {
                Shape::Freehand(f) => Some(f),
                _ => None,
            })
            .collect();
        for brush in Brush::ALL {
            for width in WIDTHS {
                assert!(
                    strokes
                        .iter()
                        .any(|f| f.brush == brush && f.style.stroke_width == width)
                );
            }
        }
    }

    #[test]
    fn test_letters_sit_on_the_guides() {
        // "x" spans exactly baseline to x-height.
        let sheet = PracticeSheet::Handwriting;
        let glyphs = parse_font(sheet.font().0);
        let x = glyph(&glyphs, 'x').unwrap();
        let ys = x.strokes.iter().flatten().map(|p| p.y);
        let (top, bottom) = ys.fold((f64::MAX, f64::MIN), |(a, b), y| (a.min(y), b.max(y)));
        assert_eq!((bottom - top) * sheet.scale(), sheet.guides().x_height);
        assert_eq!(bottom, 9.0);
    }
}
