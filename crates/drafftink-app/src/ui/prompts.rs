//! Prompts about unsaved changes when closing a tab or quitting.

use super::*;

/// Ask what to do with a tab's unsaved changes before closing it.
pub(super) fn render_close_prompt(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let index = ui_state.close_prompt?;
    let Some(tab) = ui_state.tabs.get(index).cloned() else {
        ui_state.close_prompt = None;
        return None;
    };
    let mut action = None;
    egui::Window::new("Unsaved changes")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(format!("Save changes to \"{}\" before closing?", tab.name));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if primary_btn(ui, "Save") {
                    action = Some(UiAction::SaveAndCloseTab(index));
                }
                if secondary_btn(ui, "Don't save") {
                    action = Some(UiAction::ForceCloseTab(index));
                }
                if secondary_btn(ui, "Cancel") || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    ui_state.close_prompt = None;
                }
            });
        });
    if action.is_some() {
        ui_state.close_prompt = None;
    }
    action
}

/// Ask what to do with unsaved changes before quitting.
pub(super) fn render_quit_prompt(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    if !ui_state.quit_prompt {
        return None;
    }
    let dirty: Vec<&TabInfo> = ui_state.tabs.iter().filter(|t| t.dirty).collect();
    let all_have_files = dirty.iter().all(|t| t.path.is_some());
    let mut action = None;
    let mut cancel = false;
    egui::Window::new("Quit with unsaved changes?")
        .collapsible(false)
        .resizable(false)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label("These tabs have unsaved changes:");
            for tab in &dirty {
                let saved_in = match &tab.path {
                    Some(path) => path.display().to_string(),
                    None => "not saved yet".to_string(),
                };
                ui.label(format!("• {} ({saved_in})", tab.name));
            }
            if !all_have_files {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Save new documents with Save As before quitting.")
                        .size(11.0)
                        .color(Color32::from_gray(120)),
                );
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if all_have_files && primary_btn(ui, "Save all and quit") {
                    action = Some(UiAction::SaveAllAndQuit);
                }
                if secondary_btn(ui, "Quit without saving") {
                    action = Some(UiAction::QuitDiscard);
                }
                if secondary_btn(ui, "Cancel") || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    cancel = true;
                }
            });
        });
    if cancel || action.is_some() {
        ui_state.quit_prompt = false;
    }
    action
}
