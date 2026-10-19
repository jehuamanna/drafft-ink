//! Practice sheets: every letter, number and symbol in a handwriting or
//! calligraphy style, on matching guide lines, with an empty row after each
//! line to copy it. Most styles show each brush at each stroke width; the
//! outlined styles are grey letters to trace with any brush.
//!
//! Letters come from single-line stroke fonts (see `assets/practice`), drawn
//! as pen strokes so every brush renders them as it would your writing.

use crate::canvas::CanvasDocument;
use crate::shapes::{Brush, Freehand, SerializableColor, Shape, Text};
use kurbo::Point;

/// Handwriting (3-line paper) or calligraphy (4-line paper).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetKind {
    Handwriting,
    Calligraphy,
}

impl SheetKind {
    pub fn name(self) -> &'static str {
        match self {
            SheetKind::Handwriting => "Handwriting practice",
            SheetKind::Calligraphy => "Calligraphy practice",
        }
    }
}

/// How a style's letters are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Every brush at every stroke width.
    Brushes,
    /// The letters are drawn as outlines (double lines): one block of grey
    /// letters to trace over.
    Trace,
}

/// A lettering style: its font and how to show it.
struct Style {
    kind: SheetKind,
    name: &'static str,
    font: &'static str,
    /// Extra slant applied to the font, in degrees from upright.
    shear: f64,
    mode: Mode,
}

macro_rules! font {
    ($file:literal) => {
        include_str!(concat!("../assets/practice/", $file, ".strokes"))
    };
}

const STYLES: &[Style] = &[
    // Handwriting
    Style {
        kind: SheetKind::Handwriting,
        name: "Print",
        font: font!("romans"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Handwriting,
        name: "Slanted print",
        font: font!("romans"),
        shear: 10.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Handwriting,
        name: "Clean print",
        font: font!("EMSReadability"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Handwriting,
        name: "Clean italic print",
        font: font!("EMSReadabilityItalic"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Handwriting,
        name: "Casual hand",
        font: font!("EMSTech"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Handwriting,
        name: "Typewriter print",
        font: font!("EMSNixish"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Handwriting,
        name: "School cursive",
        font: font!("scripts"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Handwriting,
        name: "Serif print (trace)",
        font: font!("romanc"),
        shear: 0.0,
        mode: Mode::Trace,
    },
    // Calligraphy
    Style {
        kind: SheetKind::Calligraphy,
        name: "Copperplate script",
        font: font!("scripts"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Formal script",
        font: font!("EMSAllure"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Chancery",
        font: font!("EMSFelix"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Slanted roman (italic hand)",
        font: font!("romans"),
        shear: 8.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Italic",
        font: font!("EMSNixishItalic"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Decorative",
        font: font!("EMSElfin"),
        shear: 0.0,
        mode: Mode::Brushes,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Blackletter (trace)",
        font: font!("gotheng"),
        shear: 0.0,
        mode: Mode::Trace,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Fraktur (trace)",
        font: font!("gothger"),
        shear: 0.0,
        mode: Mode::Trace,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Rotunda (trace)",
        font: font!("gothita"),
        shear: 0.0,
        mode: Mode::Trace,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Formal script (trace)",
        font: font!("scriptc"),
        shear: 0.0,
        mode: Mode::Trace,
    },
    Style {
        kind: SheetKind::Calligraphy,
        name: "Italic (trace)",
        font: font!("italicc"),
        shear: 0.0,
        mode: Mode::Trace,
    },
];

/// A practice sheet: one lettering style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PracticeSheet(usize);

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
/// Letters to trace over: light enough that your own strokes stand out.
const TRACE: SerializableColor = SerializableColor {
    r: 148,
    g: 163,
    b: 184,
    a: 255,
};
const LABEL: SerializableColor = SerializableColor {
    r: 100,
    g: 116,
    b: 139,
    a: 255,
};

impl PracticeSheet {
    /// Every sheet, handwriting first.
    pub fn all() -> impl Iterator<Item = PracticeSheet> {
        (0..STYLES.len()).map(PracticeSheet)
    }

    fn style(self) -> &'static Style {
        &STYLES[self.0]
    }

    pub fn kind(self) -> SheetKind {
        self.style().kind
    }

    /// The style's name, e.g. "Chancery".
    pub fn name(self) -> &'static str {
        self.style().name
    }

    /// Full title, e.g. "Calligraphy practice: Chancery".
    pub fn title(self) -> String {
        format!("{}: {}", self.kind().name(), self.name())
    }

    fn font(self) -> Font {
        Font::parse(self.style().font)
    }

    /// Letter size: brush sheets at a comfortable writing size, letters to
    /// trace larger.
    fn x_height(self) -> f64 {
        match self.style().mode {
            Mode::Brushes => 40.0,
            Mode::Trace => 60.0,
        }
    }

    /// The guide lines this sheet is laid out on.
    pub fn guides(self) -> SheetGuides {
        let font = self.font();
        let slant = font.slant + self.style().shear;
        SheetGuides {
            x_height: self.x_height(),
            ascender: font.ascender / font.x_height,
            descender: font.descender / font.x_height,
            row_gap: 0.5,
            // Upright styles get no slant lines.
            slant: if slant.abs() < 3.0 {
                0.0
            } else {
                slant.round()
            },
        }
    }

    /// Build the sheet: for brush styles one block per brush (down) and
    /// stroke width (across); for outlined styles one block to trace.
    pub fn build(self) -> CanvasDocument {
        let font = self.font();
        let guides = self.guides();
        let row = guides.row_height();
        let x_height = guides.x_height;
        let scale = x_height / font.x_height;
        let shear = self.style().shear.to_radians().tan();
        // Letters are spaced apart so each can be traced on its own.
        let spacing = x_height * 0.6;
        let label_width = x_height * 5.0;

        let line_width = |text: &str| -> f64 {
            text.chars()
                .filter_map(|c| font.glyph(c))
                .map(|g| g.advance * scale + spacing)
                .sum()
        };
        let block_width = label_width
            + [UPPERCASE, LOWERCASE, NUMBERS, SYMBOLS]
                .iter()
                .map(|t| line_width(t))
                .fold(0.0, f64::max)
            + x_height * 4.0;

        let mut doc = CanvasDocument::new();
        doc.name = self.title();
        let left = x_height * 2.0;

        let add_text = |doc: &mut CanvasDocument, x: f64, baseline: f64, size: f64, text: &str| {
            let mut label = Text::new(Point::new(x, baseline - size * 1.25), text.to_string())
                .with_font_size(size);
            label.style.stroke_color = LABEL;
            doc.add_shape(Shape::Text(label));
        };
        add_text(&mut doc, left, row, x_height * 1.2, &self.title());
        let instructions = match self.style().mode {
            Mode::Brushes => {
                "Each block is one brush (down) at one stroke width (across): uppercase, \
                 lowercase, numbers and symbols,\neach with an empty row below to write it \
                 yourself. Pick the same brush and width in the pen panel and copy along the lines."
            }
            Mode::Trace => {
                "Trace over the grey letters with any brush, then write each line yourself in \
                 the empty row below it."
            }
        };
        add_text(&mut doc, left, row * 2.0, x_height * 0.5, instructions);

        let blocks: Vec<(Brush, f64, SerializableColor)> = match self.style().mode {
            Mode::Brushes => Brush::ALL
                .into_iter()
                .flat_map(|brush| WIDTHS.into_iter().map(move |w| (brush, w, INK)))
                .collect(),
            Mode::Trace => vec![(Brush::Pen, 1.0, TRACE)],
        };
        for (i, &(brush, width, color)) in blocks.iter().enumerate() {
            let (down, across) = match self.style().mode {
                Mode::Brushes => (i / WIDTHS.len(), i % WIDTHS.len()),
                Mode::Trace => (0, 0),
            };
            let x0 = left + across as f64 * block_width;
            let first_row = HEADER_ROWS + down * BLOCK_ROWS;
            let baseline = |r: usize| (first_row + r) as f64 * row;

            if self.style().mode == Mode::Brushes {
                add_text(
                    &mut doc,
                    x0,
                    baseline(0),
                    x_height * 0.7,
                    &format!("{} · {width} px", brush.name()),
                );
            }
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
                    let Some(g) = font.glyph(c) else {
                        continue;
                    };
                    for stroke in &g.strokes {
                        let points: Vec<Point> = stroke
                            .iter()
                            .map(|p| {
                                // Font y is up from the baseline.
                                Point::new(cursor + (p.x + p.y * shear) * scale, base - p.y * scale)
                            })
                            .collect();
                        if let Some(shape) = pen_stroke(&points, brush, width, color) {
                            doc.add_shape(shape);
                        }
                    }
                    cursor += g.advance * scale + spacing;
                }
            }
        }
        doc
    }
}

/// A single-line font in font units, y up from the baseline.
struct Font {
    x_height: f64,
    /// Above the x-height.
    ascender: f64,
    /// Below the baseline.
    descender: f64,
    /// Degrees from upright.
    slant: f64,
    /// Printable ASCII from space (32).
    glyphs: Vec<Glyph>,
}

struct Glyph {
    advance: f64,
    strokes: Vec<Vec<Point>>,
}

impl Font {
    /// Parse the `.strokes` format (see `assets/practice/README.md`).
    fn parse(data: &str) -> Self {
        let mut font = Font {
            x_height: 1.0,
            ascender: 0.0,
            descender: 0.0,
            slant: 0.0,
            glyphs: Vec::new(),
        };
        for line in data.lines().filter(|l| !l.starts_with('#')) {
            let mut parts = line.splitn(3, ' ');
            let (Some(first), Some(second)) = (parts.next(), parts.next()) else {
                continue;
            };
            if first == "metrics" {
                let values: Vec<f64> = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|v| v.parse().ok())
                    .collect();
                if let [x_height, ascender, descender, slant] = values[..] {
                    font.x_height = x_height.max(f64::EPSILON);
                    font.ascender = ascender;
                    font.descender = descender;
                    font.slant = slant;
                }
                continue;
            }
            let strokes = parts
                .next()
                .unwrap_or("")
                .split('|')
                .map(|stroke| {
                    stroke
                        .split_whitespace()
                        .filter_map(|xy| {
                            let (x, y) = xy.split_once(',')?;
                            Some(Point::new(x.parse().ok()?, y.parse().ok()?))
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|s| !s.is_empty())
                .collect();
            font.glyphs.push(Glyph {
                advance: second.parse().unwrap_or(0.0),
                strokes,
            });
        }
        font
    }

    fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get((c as usize).checked_sub(32)?)
    }
}

/// A pen stroke through a glyph's polyline: curves smoothed, corners kept,
/// and pressure easing in and out like a written stroke.
fn pen_stroke(
    polyline: &[Point],
    brush: Brush,
    width: f64,
    color: SerializableColor,
) -> Option<Shape> {
    // Sparse enough to stay light; the renderer smooths between points.
    let points = smooth(polyline, 4.0);
    if points.len() < 2 {
        // A dot (e.g. in "i" or ":"): a tiny stroke so it still shows.
        let p = *polyline.first()?;
        return pen_stroke(&[p, p + kurbo::Vec2::new(0.5, 0.5)], brush, width, color);
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
    stroke.style.stroke_color = color;
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
        for sheet in PracticeSheet::all() {
            let font = sheet.font();
            assert_eq!(font.glyphs.len(), 95, "{}", sheet.name());
            for c in [UPPERCASE, LOWERCASE, NUMBERS].concat().chars() {
                let g = font.glyph(c).unwrap();
                assert!(!g.strokes.is_empty(), "{c:?} missing in {}", sheet.name());
                assert!(g.advance > 0.0);
            }
        }
    }

    #[test]
    fn test_both_kinds_have_sheets() {
        for kind in [SheetKind::Handwriting, SheetKind::Calligraphy] {
            assert!(PracticeSheet::all().filter(|s| s.kind() == kind).count() >= 5);
        }
    }

    #[test]
    fn test_brush_sheet_has_every_combination() {
        let doc = PracticeSheet(0).build();
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
        // Print "x" spans exactly baseline to x-height.
        let sheet = PracticeSheet(0);
        let font = sheet.font();
        let x = font.glyph('x').unwrap();
        let ys = x.strokes.iter().flatten().map(|p| p.y);
        let (low, high) = ys.fold((f64::MAX, f64::MIN), |(a, b), y| (a.min(y), b.max(y)));
        assert_eq!(low, 0.0);
        assert_eq!(
            high * sheet.guides().x_height / font.x_height,
            sheet.guides().x_height
        );
    }
}
