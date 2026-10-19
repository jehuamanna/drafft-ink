//! UI actions added by this fork that need the whole [`AppState`], so they
//! run after the egui pass rather than inside it.

use drafftink_core::shapes::Shape;
use drafftink_render::GridStyle;

#[cfg(feature = "native")]
use super::documents::{open_document, save_tab};
use super::documents::{request_close_tab, tab_is_dirty};
use super::{AppState, add_tab, broadcast_doc_changes, close_tab, file_ops, switch_to_tab};
use crate::ui::UiAction;

/// Run one of this fork's actions (any the egui pass doesn't handle).
pub(super) fn handle(state: &mut AppState, action: UiAction) {
    match action {
        UiAction::CloseActiveTab => request_close_tab(state, state.active_tab),
        UiAction::ForceCloseTab(i) => close_tab(state, i),
        UiAction::CloseOtherTabs(keep) => {
            // Close the clean ones; ask about the first with
            // unsaved changes (the rest stay open).
            let mut keep = keep;
            for i in (0..state.tabs.len()).rev() {
                if i != keep && !tab_is_dirty(state, i) {
                    close_tab(state, i);
                    if i < keep {
                        keep -= 1;
                    }
                }
            }
            switch_to_tab(state, keep);
            state.ui_state.close_prompt =
                (0..state.tabs.len()).find(|&i| i != keep && tab_is_dirty(state, i));
        }
        action @ (UiAction::NextTab | UiAction::PreviousTab) => {
            let n = state.tabs.len();
            let step = if matches!(action, UiAction::NextTab) {
                1
            } else {
                n - 1
            };
            switch_to_tab(state, (state.active_tab + step) % n);
        }
        UiAction::RenameTab(i, name) => {
            if let Some(tab) = state.tabs.get_mut(i) {
                tab.name = name.clone();
                tab.document.name = name.clone();
            }
            if i == state.active_tab {
                state.canvas.document.name = name;
            }
        }
        #[cfg(feature = "native")]
        UiAction::Save => {
            save_tab(state, state.active_tab, false);
        }
        #[cfg(feature = "native")]
        UiAction::SaveAs => {
            save_tab(state, state.active_tab, true);
        }
        #[cfg(feature = "native")]
        UiAction::SaveTab(i) => {
            save_tab(state, i, false);
        }
        #[cfg(feature = "native")]
        UiAction::SaveTabAs(i) => {
            save_tab(state, i, true);
        }
        #[cfg(feature = "native")]
        UiAction::SaveAndCloseTab(i) => {
            if save_tab(state, i, false) {
                close_tab(state, i);
            } else if let Some(tab) = state.tabs.get_mut(i) {
                // Closes once saved from the file dialog.
                tab.close_after_save = true;
            }
        }
        #[cfg(feature = "native")]
        UiAction::OpenFile => file_ops::load_document(),
        #[cfg(feature = "native")]
        UiAction::OpenPath(path) => match file_ops::read_document(&path) {
            Ok(doc) => open_document(state, doc, Some(path)),
            Err(e) => {
                log::error!("Couldn't open {path:?}: {e}");
                state.ui_state.recent_files.retain(|p| p != &path);
            }
        },
        #[cfg(feature = "native")]
        UiAction::SaveAllAndQuit => {
            let dirty: Vec<usize> = (0..state.tabs.len())
                .filter(|&i| tab_is_dirty(state, i))
                .collect();
            let saved_all = dirty.into_iter().all(|i| save_tab(state, i, false));
            if saved_all {
                state.extra.quit_requested = true;
            }
        }
        #[cfg(target_arch = "wasm32")]
        UiAction::Save => {
            file_ops::save_document(&state.canvas.document, &state.canvas.document.name);
        }
        UiAction::QuitDiscard => state.extra.quit_requested = true,
        UiAction::NewDocument => {
            // "Untitled", or "Untitled 2", 3... if taken.
            let name = (1..)
                .map(|n| match n {
                    1 => "Untitled".to_string(),
                    n => format!("Untitled {n}"),
                })
                .find(|name| state.tabs.iter().all(|t| &t.name != name))
                .unwrap_or_default();
            let mut doc = drafftink_core::canvas::CanvasDocument::new();
            doc.name = name.clone();
            add_tab(state, name, doc);
            log::info!("New document");
        }
        UiAction::ShowPractice(sheet) => {
            // Its own tab (reused if still open), on the
            // guide lines the letters were laid out on.
            let title = sheet.title();
            match state.tabs.iter().position(|t| t.name == title) {
                Some(index) => switch_to_tab(state, index),
                None => {
                    let mut doc = sheet.build();
                    doc.name = title.clone();
                    add_tab(state, title.clone(), doc);
                    state.canvas.camera.reset();
                }
            }
            let guides = sheet.guides();
            state.ui_state.writing_guides = drafftink_render::WritingGuides {
                x_height: guides.x_height,
                ascender: guides.ascender,
                descender: guides.descender,
                row_gap: guides.row_gap,
                slant: guides.slant,
            };
            let style = match sheet.kind() {
                drafftink_core::practice::SheetKind::Handwriting => GridStyle::Handwriting,
                drafftink_core::practice::SheetKind::Calligraphy => GridStyle::Calligraphy,
            };
            state.config.grid_style = style;
            state.ui_state.grid_style = style;
            log::info!("Opened {title}");
        }
        UiAction::SetGridStyle(style) => {
            state.config.grid_style = style;
            state.ui_state.grid_style = style;
        }
        action @ (UiAction::SetBrush(_) | UiAction::SetNibAngle(_)) => set_brush(state, action),
        _ => {}
    }
}

/// Change the brush or nib angle of the selected pen strokes or, with none
/// selected, of new strokes.
fn set_brush(state: &mut AppState, action: UiAction) {
    let selected: Vec<_> = state.canvas.selection.clone();
    let mut changed = false;
    for shape_id in selected {
        if let Some(Shape::Freehand(freehand)) = state.canvas.document.get_shape_mut(shape_id) {
            match action {
                UiAction::SetBrush(brush) => freehand.brush = brush,
                UiAction::SetNibAngle(angle) => freehand.nib_angle = angle,
                _ => {}
            }
            changed = true;
        }
    }
    if changed {
        broadcast_doc_changes(
            &mut state.collab,
            &state.canvas.document,
            state.websocket.as_ref(),
        );
    } else {
        match action {
            UiAction::SetBrush(brush) => state.canvas.tool_manager.brush = brush,
            UiAction::SetNibAngle(angle) => state.canvas.tool_manager.nib_angle = angle,
            _ => {}
        }
    }
}
