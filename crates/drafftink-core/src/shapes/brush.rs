//! Brushes for freehand strokes, and how pressure sets their width.

use serde::{Deserialize, Serialize};

/// How a freehand stroke is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Brush {
    /// Round ink pen; pressure sets the width.
    #[default]
    Pen,
    /// Flexible pointed nib (copperplate, Spencerian): hairline upstrokes,
    /// downstrokes swell with pressure.
    PointedNib,
    /// Broad-edge nib (italic, blackletter, foundational): width depends on
    /// the direction of travel against the nib angle.
    BroadNib,
    /// Brush pen: strong pressure response and tapered ends.
    BrushPen,
    /// Felt marker: even width, round ends.
    Marker,
    /// Graphite pencil: thin, grainy, light.
    Pencil,
    /// Soft airbrush-like edges.
    Soft,
}

impl Brush {
    pub const ALL: [Brush; 7] = [
        Brush::Pen,
        Brush::PointedNib,
        Brush::BroadNib,
        Brush::BrushPen,
        Brush::Marker,
        Brush::Pencil,
        Brush::Soft,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Brush::Pen => "Pen",
            Brush::PointedNib => "Pointed nib",
            Brush::BroadNib => "Broad nib",
            Brush::BrushPen => "Brush pen",
            Brush::Marker => "Marker",
            Brush::Pencil => "Pencil",
            Brush::Soft => "Soft brush",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Brush::Pen => "Round ink pen; pressure sets the width",
            Brush::PointedNib => {
                "Copperplate and Spencerian: hairline upstrokes, downstrokes swell with pressure"
            }
            Brush::BroadNib => {
                "Italic, blackletter and foundational hands: thick and thin by direction"
            }
            Brush::BrushPen => "Brush lettering and inking: very pressure-sensitive, tapered ends",
            Brush::Marker => "Even width regardless of pressure",
            Brush::Pencil => "Light, grainy graphite for sketching",
            Brush::Soft => "Soft-edged strokes for shading",
        }
    }

    /// Stable number for storage.
    pub fn to_index(self) -> i64 {
        Self::ALL.iter().position(|&b| b == self).unwrap_or(0) as i64
    }

    pub fn from_index(index: i64) -> Self {
        usize::try_from(index)
            .ok()
            .and_then(|i| Self::ALL.get(i).copied())
            .unwrap_or_default()
    }
}

/// Default angle of a broad nib, in degrees from horizontal.
pub const DEFAULT_NIB_ANGLE: f64 = 40.0;

pub(super) fn default_nib_angle() -> f64 {
    DEFAULT_NIB_ANGLE
}

/// Width of a pressure-sensitive freehand stroke at `pressure` (0.0-1.0).
///
/// Full pressure (and mouse input) gives `stroke_width * 2.3`; a light touch
/// thins down to 15% of that, so pressure is clearly visible like real ink.
pub fn pressure_width(stroke_width: f64, pressure: f64) -> f64 {
    const MIN_FRACTION: f64 = 0.15;
    // Slightly below 1 so normal writing pressure isn't too thin.
    const GAMMA: f64 = 0.8;
    let p = pressure.clamp(0.0, 1.0).powf(GAMMA);
    stroke_width * 2.3 * (MIN_FRACTION + (1.0 - MIN_FRACTION) * p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brush_index_round_trip() {
        for brush in Brush::ALL {
            assert_eq!(Brush::from_index(brush.to_index()), brush);
        }
        assert_eq!(Brush::from_index(99), Brush::Pen);
    }
}
