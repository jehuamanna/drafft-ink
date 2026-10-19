//! Line proportions for the handwriting and calligraphy grid styles.

/// Proportions of the handwriting and calligraphy guide lines. Sizes are in
/// canvas units, so the guides zoom with your writing.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct WritingGuides {
    /// Height of lowercase letters such as "x" (baseline to waist line).
    pub x_height: f64,
    /// Ascender height above the waist line, as a multiple of the x-height.
    pub ascender: f64,
    /// Descender depth below the baseline, as a multiple of the x-height.
    pub descender: f64,
    /// Space between rows, as a multiple of the x-height.
    pub row_gap: f64,
    /// Slant guide angle in degrees from vertical (0 = no slant lines).
    pub slant: f64,
}

impl Default for WritingGuides {
    fn default() -> Self {
        Self {
            x_height: 40.0,
            ascender: 1.0,
            descender: 1.0,
            row_gap: 0.5,
            slant: 0.0,
        }
    }
}

impl WritingGuides {
    /// Classic proportions to start from: (name, ascender, descender, slant).
    pub const PRESETS: [(&'static str, f64, f64, f64); 4] = [
        ("Even (1:1:1)", 1.0, 1.0, 0.0),
        ("Italic (5:5:5)", 1.0, 1.0, 7.0),
        ("Foundational (3:4:3)", 0.75, 0.75, 0.0),
        ("Copperplate (3:2:3, 55°)", 1.5, 1.5, 35.0),
    ];

    /// Distance from one baseline to the next.
    pub fn row_height(&self) -> f64 {
        self.x_height.max(1.0) * (self.ascender + 1.0 + self.descender + self.row_gap).max(0.5)
    }
}
