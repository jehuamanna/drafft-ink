//! Preferences remembered between sessions: grid style, writing guide
//! proportions, the pen brush, and the open and recent documents. Saved to
//! `~/.config/drafftink/preferences.json`.

use drafftink_core::shapes::{Brush, DEFAULT_NIB_ANGLE};
use drafftink_render::{GridStyle, WritingGuides};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub grid_style: GridStyle,
    pub writing_guides: WritingGuides,
    pub brush: Brush,
    pub nib_angle: f64,
    /// Files open in tabs, reopened on start.
    pub open_documents: Vec<std::path::PathBuf>,
    /// The file in the active tab.
    pub active_document: Option<std::path::PathBuf>,
    /// Recently saved or opened files, newest first.
    pub recent_files: Vec<std::path::PathBuf>,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            grid_style: GridStyle::Lines,
            writing_guides: WritingGuides::default(),
            brush: Brush::default(),
            nib_angle: DEFAULT_NIB_ANGLE,
            open_documents: Vec::new(),
            active_document: None,
            recent_files: Vec::new(),
        }
    }
}

impl Preferences {
    #[cfg(not(target_arch = "wasm32"))]
    fn path() -> Option<std::path::PathBuf> {
        Some(
            dirs::config_dir()?
                .join("drafftink")
                .join("preferences.json"),
        )
    }

    /// Load saved preferences, or the defaults.
    pub fn load() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(json) = Self::path().and_then(|p| std::fs::read_to_string(p).ok()) {
            match serde_json::from_str(&json) {
                Ok(preferences) => return preferences,
                Err(e) => log::warn!("Ignoring invalid preferences: {e}"),
            }
        }
        Self::default()
    }

    /// Save preferences (native only).
    pub fn save(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(path) = Self::path() else {
                return;
            };
            let result = path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| {
                    let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
                    std::fs::write(&path, json)
                });
            if let Err(e) = result {
                log::warn!("Failed to save preferences: {e}");
            }
        }
    }
}
