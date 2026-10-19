//! State and per-frame hooks for this fork's features: saved settings,
//! queued shortcut actions, the brush outline and the quick toolbars.

use std::collections::VecDeque;

use drafftink_core::tools::ToolKind;
use kurbo::Point;

use super::AppState;
use super::commands::ViewToggle;
#[cfg(feature = "native")]
use super::documents::open_document;
#[cfg(feature = "native")]
use super::file_ops;
#[cfg(feature = "native")]
use super::switch_to_tab;
use crate::keybindings::Keybindings;
use crate::preferences::Preferences;
use crate::tablet_settings::TabletSettings;
use crate::ui::QueuedAction;

/// [`AppState`] fields for this fork's features, kept together so upstream
/// changes to `AppState` merge cleanly.
pub(super) struct ExtraState {
    /// Whether the pointer is over the window (hides the brush outline).
    pub(super) pointer_in_window: bool,
    /// Whether the window has keyboard focus; tablet pad controls are
    /// ignored while it doesn't.
    pub(super) focused: bool,
    /// Brush outline opacity; fades with pen hover height when enabled.
    pub(super) brush_cursor_opacity: f64,
    /// UI actions triggered by shortcuts or tablet controls, applied one per
    /// UI pass.
    pub(super) queued_actions: VecDeque<QueuedAction>,
    /// Tablet settings as last saved, to save again when they change.
    pub(super) saved_tablet_settings: TabletSettings,
    /// Keyboard shortcuts as last saved.
    pub(super) saved_keybindings: Keybindings,
    /// Grid and brush preferences as last saved.
    pub(super) saved_preferences: Preferences,
    /// Last "… / back" view command, to undo it if pressed again.
    pub(super) view_toggle: Option<ViewToggle>,
    /// Ignore the next mouse release (its press closed the quick toolbars).
    pub(super) suppress_mouse_release: bool,
    /// Quit after this frame (unsaved changes were dealt with).
    pub(super) quit_requested: bool,
}

impl Default for ExtraState {
    fn default() -> Self {
        Self {
            pointer_in_window: false,
            focused: true,
            brush_cursor_opacity: 1.0,
            queued_actions: VecDeque::new(),
            saved_tablet_settings: TabletSettings::default(),
            saved_keybindings: Keybindings::default(),
            saved_preferences: Preferences::default(),
            view_toggle: None,
            suppress_mouse_release: false,
            quit_requested: false,
        }
    }
}

/// Load saved settings and preferences, and reopen the documents open last
/// time.
pub(super) fn load_settings(state: &mut AppState) {
    let settings = TabletSettings::load();
    state.ui_state.tablet_settings = settings.clone();
    state.extra.saved_tablet_settings = settings;
    let keybindings = Keybindings::load();
    state.ui_state.keybindings = keybindings.clone();
    state.extra.saved_keybindings = keybindings;
    let preferences = Preferences::load();
    state.config.grid_style = preferences.grid_style;
    state.ui_state.grid_style = preferences.grid_style;
    state.ui_state.writing_guides = preferences.writing_guides;
    state.canvas.tool_manager.brush = preferences.brush;
    state.canvas.tool_manager.nib_angle = preferences.nib_angle;
    state.ui_state.recent_files = preferences.recent_files.clone();
    // Reopen the documents open last time instead of the intro.
    #[cfg(feature = "native")]
    {
        let documents: Vec<_> = preferences
            .open_documents
            .iter()
            .filter_map(|path| match file_ops::read_document(path) {
                Ok(doc) => Some((doc, path.clone())),
                Err(e) => {
                    log::warn!("Couldn't reopen {path:?}: {e}");
                    None
                }
            })
            .collect();
        if !documents.is_empty() {
            state.canvas.document = drafftink_core::canvas::CanvasDocument::new();
            for (doc, path) in documents {
                log::info!("Reopened {path:?}");
                open_document(state, doc, Some(path));
            }
            let active = preferences.active_document.as_ref().and_then(|path| {
                state
                    .tabs
                    .iter()
                    .position(|t| t.path.as_ref() == Some(path))
            });
            if let Some(index) = active {
                switch_to_tab(state, index);
            }
            // Reopening isn't a change: keep the recent files order.
            state.ui_state.recent_files = preferences.recent_files.clone();
        }
    }
    state.extra.saved_preferences = preferences;
}

/// Save settings changed in this UI pass, and keep applying queued actions.
pub(super) fn after_ui_pass(state: &mut AppState) {
    // Persist tablet settings edited in the settings window, and
    // keep applying queued actions (one per UI pass).
    if state.ui_state.tablet_settings != state.extra.saved_tablet_settings {
        state.ui_state.tablet_settings.save();
        state.extra.saved_tablet_settings = state.ui_state.tablet_settings.clone();
    }
    if state.ui_state.keybindings != state.extra.saved_keybindings {
        state.ui_state.keybindings.save();
        state.extra.saved_keybindings = state.ui_state.keybindings.clone();
    }
    let preferences = Preferences {
        grid_style: state.config.grid_style,
        writing_guides: state.ui_state.writing_guides,
        brush: state.canvas.tool_manager.brush,
        nib_angle: state.canvas.tool_manager.nib_angle,
        open_documents: state.tabs.iter().filter_map(|t| t.path.clone()).collect(),
        active_document: state.tabs[state.active_tab].path.clone(),
        recent_files: state.ui_state.recent_files.clone(),
    };
    if preferences != state.extra.saved_preferences {
        preferences.save();
        state.extra.saved_preferences = preferences;
    }
    if !state.extra.queued_actions.is_empty() {
        state.window.request_redraw();
    }
    // Once a "… / back" command's view change has been applied,
    // remember the resulting view.
    if let Some(toggle) = &mut state.extra.view_toggle {
        if toggle.after.is_none() && state.extra.queued_actions.is_empty() {
            let camera = &state.canvas.camera;
            toggle.after = Some((camera.offset, camera.zoom));
        }
    }
}

/// The brush outline (position, width, opacity): at full-pressure width
/// while hovering with a drawing tool over the canvas.
pub(super) fn brush_cursor(state: &AppState) -> Option<(Point, f64, f64)> {
    let tool = state.canvas.tool_manager.current_tool;
    let drawing_tool = matches!(tool, ToolKind::Freehand | ToolKind::Highlighter);
    if drawing_tool
        && !state.canvas.tool_manager.is_active()
        && !state.egui_ctx.is_pointer_over_area()
        && state.extra.pointer_in_window
    {
        let mut stroke_width = state.ui_state.stroke_width as f64;
        if tool == ToolKind::Highlighter {
            stroke_width = stroke_width.max(12.0);
        }
        let world = state
            .canvas
            .camera
            .screen_to_world(state.input.mouse_position());
        Some((
            world,
            drafftink_core::shapes::pressure_width(stroke_width, 1.0),
            state.extra.brush_cursor_opacity,
        ))
    } else {
        None
    }
}
