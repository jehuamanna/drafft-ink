//! This fork's additions to the tab strip: unsaved-change marks, renaming,
//! a context menu, and the new document button.

use super::*;

/// A tab's label; a dot marks unsaved changes.
pub(super) fn label(tab: &TabInfo) -> String {
    if tab.dirty {
        format!("{} ●", tab.name)
    } else {
        tab.name.clone()
    }
}

/// A tab's tooltip: its file and whether it has unsaved changes.
pub(super) fn tooltip(tab: &TabInfo) -> String {
    let mut tooltip = match &tab.path {
        Some(path) => path.display().to_string(),
        None => "Not saved yet".to_string(),
    };
    if tab.dirty {
        tooltip.push_str("\nUnsaved changes");
    }
    tooltip.push_str("\nDouble-click to rename; right-click for more");
    tooltip
}

/// While tab `i` is being renamed, an inline text field in its place.
/// Returns true if it was shown instead of the tab.
pub(super) fn rename_field(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    i: usize,
    action: &mut Option<UiAction>,
) -> bool {
    let Some((index, name)) = &mut ui_state.renaming_tab else {
        return false;
    };
    if *index != i {
        return false;
    }
    let edit = ui.add(
        egui::TextEdit::singleline(name)
            .desired_width(140.0)
            .font(egui::FontId::proportional(12.0)),
    );
    edit.request_focus();
    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    if enter && !name.trim().is_empty() {
        *action = Some(UiAction::RenameTab(i, name.trim().to_string()));
        ui_state.renaming_tab = None;
    } else if escape || (edit.lost_focus() && !enter) {
        ui_state.renaming_tab = None;
    }
    true
}

/// Double-click renames tab `i`, middle-click closes it, and right-click
/// opens its menu.
pub(super) fn interactions(
    response: &egui::Response,
    ui_state: &mut UiState,
    tabs: &[TabInfo],
    i: usize,
    action: &mut Option<UiAction>,
) {
    if response.double_clicked() {
        ui_state.renaming_tab = Some((i, tabs[i].name.clone()));
    }
    if response.middle_clicked() {
        *action = Some(UiAction::CloseTab(i));
    }
    response.context_menu(|ui| {
        let mut item = |ui: &mut egui::Ui, text: &str, picked: UiAction| {
            if ui.button(text).clicked() {
                *action = Some(picked);
                ui.close();
            }
        };
        item(ui, "Save", UiAction::SaveTab(i));
        item(ui, "Save As...", UiAction::SaveTabAs(i));
        if ui.button("Rename...").clicked() {
            ui_state.renaming_tab = Some((i, tabs[i].name.clone()));
            ui.close();
        }
        ui.separator();
        item(ui, "Close", UiAction::CloseTab(i));
        if tabs.len() > 1 {
            item(ui, "Close Others", UiAction::CloseOtherTabs(i));
        }
    });
}

/// The "+ New" button, for a new document in its own tab.
pub(super) fn new_button(ui: &mut egui::Ui, ui_state: &UiState) -> Option<UiAction> {
    let new = egui::Button::new(
        egui::RichText::new("+ New")
            .size(12.0)
            .color(Color32::from_gray(90)),
    )
    .fill(Color32::TRANSPARENT)
    .corner_radius(egui::CornerRadius::same(4));
    let tooltip = ui_state
        .keybindings
        .with_hint("New document in its own tab", Command::NewDocument);
    ui.add(new)
        .on_hover_text(tooltip)
        .clicked()
        .then_some(UiAction::NewDocument)
}
