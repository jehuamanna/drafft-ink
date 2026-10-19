//! This fork's main menu items: documents in tabs, recent files, practice
//! sheets and settings, with the submenus that open beside them.

use super::*;

/// Which menu items open on hover, and the open one's position; tracked
/// while drawing the menu.
#[derive(Default)]
pub(super) struct Submenus {
    /// The open submenu and its parent item's rect.
    anchor: Option<(MenuSubmenu, Rect)>,
    /// Whether the pointer is over a submenu's parent item.
    over_parent: bool,
}

impl Submenus {
    /// A menu item that opens `submenu` on hover (or click).
    fn item(
        &mut self,
        ui: &mut egui::Ui,
        ui_state: &mut UiState,
        label: &str,
        submenu: MenuSubmenu,
    ) {
        let item = ui.scope(|ui| menu_item(ui, label, ""));
        let hovered = ui.rect_contains_pointer(item.response.rect);
        self.over_parent |= hovered;
        if item.inner || hovered {
            ui_state.open_submenu = Some(submenu);
        }
        if ui_state.open_submenu == Some(submenu) {
            self.anchor = Some((submenu, item.response.rect));
        }
    }

    /// Call after the menu's items: hovering another item closes the submenu.
    pub(super) fn finish(&mut self, ui: &egui::Ui, ui_state: &mut UiState) {
        if !self.over_parent && ui.ui_contains_pointer() {
            ui_state.open_submenu = None;
            self.anchor = None;
        }
    }

    /// Show the open submenu beside its parent item.
    pub(super) fn show(self, ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
        let (submenu, rect) = self.anchor?;
        let mut action = None;
        egui::Area::new(egui::Id::new("menu_submenu"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.right_top() + Vec2::new(10.0, -8.0))
            .constrain(true)
            .show(ctx, |ui| {
                panel_frame().show(ui, |ui| {
                    ui.set_width(220.0);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                    match submenu {
                        MenuSubmenu::Practice(kind) => {
                            for sheet in PracticeSheet::all().filter(|s| s.kind() == kind) {
                                if menu_item(ui, sheet.name(), "") {
                                    action = Some(UiAction::ShowPractice(sheet));
                                }
                            }
                        }
                        MenuSubmenu::Recent => {
                            if ui_state.recent_files.is_empty() {
                                ui.label(
                                    egui::RichText::new("No recent files")
                                        .size(12.0)
                                        .color(Color32::from_gray(140)),
                                );
                            }
                            for path in &ui_state.recent_files {
                                let name = path
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_default();
                                let item = ui.scope(|ui| menu_item(ui, &name, ""));
                                item.response.on_hover_text(path.display().to_string());
                                if item.inner {
                                    action = Some(UiAction::OpenPath(path.clone()));
                                }
                            }
                        }
                    }
                });
            });
        if action.is_some() {
            ui_state.menu_open = false;
            ui_state.open_submenu = None;
        }
        action
    }
}

/// A menu item that closes the menu and returns `action` when clicked.
fn action_item(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    label: &str,
    command: Command,
    action: UiAction,
) -> Option<UiAction> {
    let hint = ui_state.keybindings.hint(command);
    menu_item(ui, label, &hint).then(|| {
        ui_state.menu_open = false;
        action
    })
}

/// New, open and save items for documents in tabs.
pub(super) fn document_items(
    ui: &mut egui::Ui,
    ui_state: &mut UiState,
    submenus: &mut Submenus,
) -> Option<UiAction> {
    let mut action = action_item(
        ui,
        ui_state,
        "New Document",
        Command::NewDocument,
        UiAction::NewDocument,
    );
    #[cfg(not(target_arch = "wasm32"))]
    {
        let open = action_item(ui, ui_state, "Open...", Command::Open, UiAction::OpenFile);
        submenus.item(ui, ui_state, "Open Recent  ›", MenuSubmenu::Recent);
        let save = action_item(ui, ui_state, "Save", Command::Save, UiAction::Save);
        let save_as = action_item(
            ui,
            ui_state,
            "Save As...",
            Command::SaveAs,
            UiAction::SaveAs,
        );
        let close = action_item(
            ui,
            ui_state,
            "Close Tab",
            Command::CloseTab,
            UiAction::CloseActiveTab,
        );
        action = action.or(open).or(save).or(save_as).or(close);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = submenus;
    action
}

/// Items opening the practice sheet submenus.
pub(super) fn practice_items(ui: &mut egui::Ui, ui_state: &mut UiState, submenus: &mut Submenus) {
    for kind in [SheetKind::Handwriting, SheetKind::Calligraphy] {
        let label = format!("{}  ›", kind.name());
        submenus.item(ui, ui_state, &label, MenuSubmenu::Practice(kind));
    }
}

/// Items opening the settings window.
pub(super) fn settings_items(ui: &mut egui::Ui, ui_state: &mut UiState) {
    let settings = ui_state.keybindings.hint(Command::Settings);
    let tabs = [
        ("Settings...", settings.as_str(), None),
        ("Keyboard Shortcuts", "", Some(SettingsTab::Keyboard)),
        ("Pen & Tablet", "", Some(SettingsTab::Controls)),
    ];
    for (label, hint, tab) in tabs {
        if menu_item(ui, label, hint) {
            ui_state.settings_open = true;
            if let Some(tab) = tab {
                ui_state.settings_tab = tab;
            }
            ui_state.menu_open = false;
        }
    }
}
