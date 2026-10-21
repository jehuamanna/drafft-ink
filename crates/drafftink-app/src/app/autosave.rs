//! Debounced native saves. One worker at a time bounds memory and disk work.
use super::{
    AppState,
    documents::{mark_saved, tab_document, tab_is_dirty},
    file_ops,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread::JoinHandle,
    time::{Duration, Instant},
};

const QUIET: Duration = Duration::from_secs(1);
const MAX_DELAY: Duration = Duration::from_secs(5);

struct Pending {
    revision: u64,
    first: Instant,
    changed: Instant,
}

impl Pending {
    fn deadline(&self) -> Instant {
        (self.changed + QUIET).min(self.first + MAX_DELAY)
    }
}

#[derive(Default)]
pub(super) struct Autosave {
    pending: HashMap<u64, Pending>,
    worker: Option<Worker>,
}

struct Worker {
    handle: JoinHandle<(u64, PathBuf, u64, Result<(), String>)>,
    ready: Receiver<()>,
}

/// Join before explicit saves so an older snapshot cannot overwrite them.
pub(super) fn finish(state: &mut AppState) {
    let Some(worker) = state.extra.autosave.worker.take() else {
        return;
    };
    match worker.handle.join() {
        Ok((id, path, revision, result)) => {
            if let Some(index) = state.tabs.iter().position(|t| t.id == id) {
                match result {
                    Ok(()) => {
                        mark_saved(state, index, path, revision);
                        state.needs_redraw = true;
                        state.window.request_redraw();
                    }
                    Err(error) => log::error!("Autosave failed for {path:?}: {error}"),
                }
            }
        }
        Err(_) => log::error!("Autosave worker panicked"),
    }
}

/// Called before sleeping, including after timer wakes. No continuous redraws.
pub(super) fn tick(state: &mut AppState, event_loop: &winit::event_loop::ActiveEventLoop) {
    if state
        .extra
        .autosave
        .worker
        .as_ref()
        .is_some_and(|w| w.ready.try_recv().is_ok() || w.handle.is_finished())
    {
        finish(state);
    }
    let now = Instant::now();
    state
        .extra
        .autosave
        .pending
        .retain(|id, _| state.tabs.iter().any(|t| t.id == *id));
    let mut due = None;
    let mut deadline = None;
    for index in 0..state.tabs.len() {
        let id = state.tabs[index].id;
        if !tab_is_dirty(state, index) {
            state.extra.autosave.pending.remove(&id);
            continue;
        }
        let revision = tab_document(state, index).revision();
        let pending = state.extra.autosave.pending.entry(id).or_insert(Pending {
            revision,
            first: now,
            changed: now,
        });
        if pending.revision != revision {
            pending.revision = revision;
            pending.changed = now;
        }
        let when = pending.deadline();
        deadline = Some(deadline.map_or(when, |d: Instant| d.min(when)));
        if when <= now && due.is_none_or(|(_, earliest)| when < earliest) {
            due = Some((index, when));
        }
    }
    if state.extra.autosave.worker.is_some() {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        return;
    }
    if let Some((index, _)) = due {
        let tab = &state.tabs[index];
        let id = tab.id;
        let path = tab.path.clone().or_else(|| {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some(
                dirs::data_local_dir()?
                    .join("drafftink/documents")
                    .join(format!("Untitled-{stamp}-{id}.json")),
            )
        });
        // Retry failures at a bounded rate, keeping the tab dirty.
        state.extra.autosave.pending.remove(&id);
        if let Some(path) = path {
            let document = tab_document(state, index).clone();
            let revision = document.revision();
            let (done, ready) = mpsc::channel();
            let handle = std::thread::spawn(move || {
                let result = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .map_err(|e| e.to_string())
                    .and_then(|()| file_ops::write_document(&document, &path));
                // Publish completion before waking; the wake can be handled
                // before this thread returns from its closure.
                let _ = done.send(());
                file_ops::wake();
                (id, path, revision, result)
            });
            state.extra.autosave.worker = Some(Worker { handle, ready });
        }
        // Clear the expired debounce timer while the worker runs. If no local
        // data directory was available, retry without spinning the event loop.
        event_loop.set_control_flow(if state.extra.autosave.worker.is_some() {
            winit::event_loop::ControlFlow::Wait
        } else {
            winit::event_loop::ControlFlow::WaitUntil(now + QUIET)
        });
    } else if let Some(deadline) = deadline {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
    } else {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debounce_is_capped_during_continuous_edits() {
        let start = Instant::now();
        let mut pending = Pending {
            revision: 1,
            first: start,
            changed: start,
        };
        assert_eq!(pending.deadline(), start + QUIET);
        pending.changed = start + Duration::from_millis(500);
        assert_eq!(pending.deadline(), start + Duration::from_millis(1500));
        pending.changed = start + Duration::from_millis(4900);
        assert_eq!(pending.deadline(), start + MAX_DELAY);
    }
}
