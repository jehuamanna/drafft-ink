//! Documents in tabs: their files, unsaved changes, opening and saving.
#[cfg(feature = "native")]
use super::file_ops;
use super::{AppState, TabState, add_tab, close_tab, switch_to_tab};

impl TabState {
    pub(super) fn new(name: String, document: drafftink_core::canvas::CanvasDocument) -> Self {
        static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            name,
            saved_revision: document.revision(),
            document,
            camera: drafftink_core::Camera::new(),
            path: None,
            close_after_save: false,
        }
    }
}

/// The document in tab `index` (the live canvas for the active tab).
pub(super) fn tab_document(
    state: &AppState,
    index: usize,
) -> &drafftink_core::canvas::CanvasDocument {
    if index == state.active_tab {
        &state.canvas.document
    } else {
        &state.tabs[index].document
    }
}

/// Whether tab `index` has changes since it was last saved or opened.
pub(super) fn tab_is_dirty(state: &AppState, index: usize) -> bool {
    tab_document(state, index).revision() != state.tabs[index].saved_revision
}

/// Close a tab, first asking what to do with unsaved changes.
pub(super) fn request_close_tab(state: &mut AppState, index: usize) {
    if index >= state.tabs.len() {
        return;
    }
    if tab_is_dirty(state, index) {
        state.ui_state.close_prompt = Some(index);
    } else {
        close_tab(state, index);
    }
}

/// Add `path` to the front of the recent files.
pub(super) fn remember_recent(state: &mut AppState, path: std::path::PathBuf) {
    let recent = &mut state.ui_state.recent_files;
    recent.retain(|p| p != &path);
    recent.insert(0, path);
    recent.truncate(10);
}

/// Record that tab `index` was saved to `path` at document `revision`; the
/// tab takes the file's name.
pub(super) fn mark_saved(
    state: &mut AppState,
    index: usize,
    path: std::path::PathBuf,
    revision: u64,
) {
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| state.tabs[index].name.clone());
    let tab = &mut state.tabs[index];
    tab.saved_revision = revision;
    tab.path = Some(path.clone());
    tab.name = name.clone();
    tab.document.name = name.clone();
    if index == state.active_tab {
        state.canvas.document.name = name;
    }
    remember_recent(state, path);
}

/// Save tab `index` to its file, or (with `save_as`, or before its first
/// save) to a file chosen in a dialog. Returns true if it was saved now; a
/// dialog save finishes later (see `file_ops::take_pending_saves`).
#[cfg(feature = "native")]
pub(super) fn save_tab(state: &mut AppState, index: usize, save_as: bool) -> bool {
    let Some(tab) = state.tabs.get(index) else {
        return false;
    };
    let document = tab_document(state, index).clone();
    let revision = document.revision();
    match (&tab.path, save_as) {
        (Some(path), false) => match file_ops::write_document(&document, path) {
            Ok(()) => {
                let path = path.clone();
                mark_saved(state, index, path, revision);
                true
            }
            Err(e) => {
                log::error!("Failed to save {path:?}: {e}");
                false
            }
        },
        _ => {
            file_ops::save_document_as(document, tab.name.clone(), tab.id, revision);
            false
        }
    }
}

/// Show an opened document: switching to it if it's already open, in the
/// current tab if that's an empty new one, otherwise in a new tab; with the
/// view fitted to its content.
pub(super) fn open_document(
    state: &mut AppState,
    document: drafftink_core::canvas::CanvasDocument,
    path: Option<std::path::PathBuf>,
) {
    if let Some(path) = &path {
        if let Some(index) = state
            .tabs
            .iter()
            .position(|t| t.path.as_ref() == Some(path))
        {
            switch_to_tab(state, index);
            remember_recent(state, path.clone());
            return;
        }
    }
    let name = if document.name.trim().is_empty() {
        "Untitled".to_string()
    } else {
        document.name.clone()
    };
    let active = state.active_tab;
    let reuse = state.canvas.document.is_empty()
        && state.tabs[active].path.is_none()
        && !tab_is_dirty(state, active);
    if reuse {
        state.tabs[active] = TabState::new(name, document.clone());
        state.canvas.document = document;
        state.canvas.clear_selection();
    } else {
        add_tab(state, name, document);
    }
    let active = state.active_tab;
    state.tabs[active].saved_revision = state.canvas.document.revision();
    if let Some(path) = path {
        state.tabs[active].path = Some(path.clone());
        remember_recent(state, path);
    }
    if let Some(bounds) = state.canvas.document.bounds() {
        state
            .canvas
            .camera
            .fit_to_bounds(bounds, state.canvas.viewport_size, 50.0);
    }
    state.needs_redraw = true;
    state.window.request_redraw();
}

/// Apply file operations finished in the background: a document opened or
/// saved from a file dialog.
#[cfg(feature = "native")]
pub(super) fn take_file_results(state: &mut AppState) {
    if let Some((doc, path)) = file_ops::take_pending_document() {
        open_document(state, doc, path);
    }
    for (tab_id, path, revision) in file_ops::take_pending_saves() {
        if let Some(index) = state.tabs.iter().position(|t| t.id == tab_id) {
            mark_saved(state, index, path, revision);
            if std::mem::take(&mut state.tabs[index].close_after_save) {
                close_tab(state, index);
            }
        }
    }
}
