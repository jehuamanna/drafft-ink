//! Native document files: reading and writing them, saving from a file
//! dialog, and waking the event loop when background file work finishes.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use drafftink_core::canvas::CanvasDocument;

/// Saves finished from a file dialog: (tab id, file, document revision).
static PENDING_SAVES: Mutex<Vec<(u64, PathBuf, u64)>> = Mutex::new(Vec::new());

/// Wakes the event loop when a background file operation finishes, so its
/// result shows without waiting for the next input event.
static WAKER: Mutex<Option<winit::event_loop::EventLoopProxy<crate::tablet::UserEvent>>> =
    Mutex::new(None);

pub fn set_waker(proxy: winit::event_loop::EventLoopProxy<crate::tablet::UserEvent>) {
    if let Ok(mut waker) = WAKER.lock() {
        *waker = Some(proxy);
    }
}

pub(super) fn wake() {
    if let Some(proxy) = WAKER.lock().ok().and_then(|w| w.clone()) {
        let _ = proxy.send_event(crate::tablet::UserEvent::Wake);
    }
}

/// Read a saved document, named after its file.
pub fn read_document(path: &Path) -> Result<CanvasDocument, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut doc = CanvasDocument::from_json(&content).map_err(|e| e.to_string())?;
    if let Some(stem) = path.file_stem() {
        doc.name = stem.to_string_lossy().to_string();
    }
    Ok(doc)
}

/// Write a document to `path`. Writes a temporary file first and renames
/// it over the old one, so a failed save never leaves a half-written file.
pub fn write_document(document: &CanvasDocument, path: &Path) -> Result<(), String> {
    let json = document.to_json().map_err(|e| e.to_string())?;
    let mut temp = path.as_os_str().to_owned();
    temp.push(".saving");
    let temp = PathBuf::from(temp);
    std::fs::write(&temp, json).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
    log::info!("Saved document to: {path:?}");
    Ok(())
}

/// Ask where to save a tab's document, then save it there (async,
/// non-blocking). Finished saves come back from `take_pending_saves()`.
pub fn save_document_as(document: CanvasDocument, name: String, tab_id: u64, revision: u64) {
    std::thread::spawn(move || {
        let dialog = rfd::FileDialog::new()
            .set_title("Save Document")
            .set_file_name(format!("{name}.json"))
            .add_filter("DrafftInk Document", &["json"]);
        if let Some(mut path) = dialog.save_file() {
            if path.extension().is_none() {
                path.set_extension("json");
            }
            match write_document(&document, &path) {
                Ok(()) => {
                    if let Ok(mut saves) = PENDING_SAVES.lock() {
                        saves.push((tab_id, path, revision));
                    }
                    wake();
                }
                Err(e) => log::error!("Failed to save {path:?}: {e}"),
            }
        }
    });
}

/// Take saves finished from a file dialog: (tab id, file, revision).
pub fn take_pending_saves() -> Vec<(u64, PathBuf, u64)> {
    PENDING_SAVES
        .lock()
        .map(|mut saves| std::mem::take(&mut *saves))
        .unwrap_or_default()
}
