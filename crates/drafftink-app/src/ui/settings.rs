//! The settings window: keyboard shortcuts, pen and tablet controls.

use super::*;

/// Tabs of the settings window.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SettingsTab {
    #[default]
    Keyboard,
    Pen,
    Controls,
}

/// Settings window: keyboard shortcuts, pen feel, and tablet buttons and dials.
pub(super) fn render_settings(ctx: &Context, ui_state: &mut UiState) {
    let mut open = true;
    egui::Window::new("Settings")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(480.0)
        .default_height(600.0)
        .vscroll(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (SettingsTab::Keyboard, "Keyboard"),
                    (SettingsTab::Pen, "Pen"),
                    (SettingsTab::Controls, "Buttons & dials"),
                ] {
                    ui.selectable_value(&mut ui_state.settings_tab, tab, label);
                }
            });
            ui.separator();
            match ui_state.settings_tab {
                SettingsTab::Keyboard => keyboard_settings(ui, ui_state),
                SettingsTab::Pen => pen_settings(ui, ui_state),
                SettingsTab::Controls => control_settings(ui, ui_state),
            }
        });
    if !open {
        ui_state.settings_open = false;
        ui_state.key_capture = None;
    }
}

/// Keyboard tab: every command and its keys.
fn keyboard_settings(ui: &mut egui::Ui, ui_state: &mut UiState) {
    ui.horizontal(|ui| {
        ui.label("Search");
        ui.text_edit_singleline(&mut ui_state.key_filter);
        if ui.button("Reset all").clicked() {
            ui_state.keybindings = Keybindings::default();
            ui_state.key_capture = None;
            ui_state.key_capture_note = None;
        }
    });
    ui.label(
        egui::RichText::new("Click a key to remove it, + to add one. A key already in use moves to the new command.")
            .small()
            .color(Color32::from_gray(140)),
    );
    if let Some(note) = &ui_state.key_capture_note {
        ui.colored_label(Color32::from_rgb(234, 179, 8), note);
    }
    ui.add_space(4.0);

    let filter = ui_state.key_filter.to_lowercase();
    let defaults = Keybindings::default();
    for &(group, commands) in COMMAND_GROUPS {
        let visible: Vec<Command> = commands
            .iter()
            .copied()
            .filter(|&c| {
                filter.is_empty()
                    || c.label().to_lowercase().contains(&filter)
                    || ui_state
                        .keybindings
                        .keys(c)
                        .iter()
                        .any(|k| k.label().to_lowercase().contains(&filter))
            })
            .collect();
        if visible.is_empty() {
            continue;
        }
        ui.label(egui::RichText::new(group).strong());
        egui::Grid::new(("keys", group))
            .num_columns(3)
            .striped(true)
            .min_col_width(160.0)
            .show(ui, |ui| {
                for command in visible {
                    ui.label(command.label());
                    ui.horizontal_wrapped(|ui| {
                        for chord in ui_state.keybindings.keys(command).to_vec() {
                            if ui
                                .button(egui::RichText::new(chord.label()).monospace())
                                .on_hover_text("Click to remove")
                                .clicked()
                            {
                                ui_state.keybindings.unbind(command, &chord);
                                ui_state.key_capture_note = None;
                            }
                        }
                        if ui_state.key_capture == Some(command) {
                            if ui
                                .button("Press a key…")
                                .on_hover_text("Click to cancel")
                                .clicked()
                            {
                                ui_state.key_capture = None;
                            }
                        } else if ui.button("+").on_hover_text("Add a key").clicked() {
                            ui_state.key_capture = Some(command);
                            ui_state.key_capture_note = None;
                        }
                    });
                    let is_default = ui_state.keybindings.keys(command) == defaults.keys(command);
                    if ui
                        .add_enabled(!is_default, egui::Button::new("↺"))
                        .on_hover_text("Reset to default")
                        .clicked()
                    {
                        ui_state.keybindings.reset(command);
                    }
                    ui.end_row();
                }
            });
        ui.add_space(6.0);
    }
}

/// Pen tab: when the pen-feel settings apply, and the settings themselves.
fn pen_settings(ui: &mut egui::Ui, ui_state: &mut UiState) {
    tablet_status(ui, ui_state);
    ui.separator();
    let s = &mut ui_state.tablet_settings;

    ui.label(egui::RichText::new("Use these pen settings").strong());
    ui.radio_value(
        &mut s.activation,
        Activation::WacomOnly,
        "When a Wacom tablet is connected",
    );
    ui.radio_value(
        &mut s.activation,
        Activation::AnyTablet,
        "For any tablet (force)",
    );
    ui.radio_value(&mut s.activation, Activation::Off, "Never (plain pen)");
    ui.label(
        egui::RichText::new("Buttons and dials (next tab) work with any tablet.")
            .small()
            .color(Color32::from_gray(140)),
    );

    egui::CollapsingHeader::new("Pressure")
        .default_open(true)
        .show(ui, |ui| {
            ui.add(
                egui::Slider::new(&mut s.pressure_curve, 0.3..=3.0)
                    .logarithmic(true)
                    .text("Curve (soft ← → firm)"),
            );
            ui.add(egui::Slider::new(&mut s.pressure_min, 0.0..=0.5).text("Ignore below"));
            ui.add(egui::Slider::new(&mut s.pressure_max, 0.3..=1.0).text("Full at"));
            ui.add(egui::Slider::new(&mut s.pressure_smoothing, 0.0..=0.9).text("Smoothing"));
            pressure_curve_plot(ui, s);
        });

    egui::CollapsingHeader::new("Stabilizer")
        .default_open(true)
        .show(ui, |ui| {
            ui.checkbox(&mut s.stabilizer, "Steady slow strokes");
            ui.add_enabled_ui(s.stabilizer, |ui| {
                ui.add(
                    egui::Slider::new(&mut s.stabilizer_min_cutoff, 0.5..=30.0)
                        .logarithmic(true)
                        .text("Responsiveness at rest (Hz)"),
                )
                .on_hover_text("Lower is steadier, higher follows the pen more closely");
                ui.add(egui::Slider::new(&mut s.stabilizer_speed, 0.0..=0.2).text("Speed boost"))
                    .on_hover_text(
                        "How quickly smoothing backs off as the pen moves faster (less lag)",
                    );
            });
        });

    egui::CollapsingHeader::new("Tilt and calligraphy").show(ui, |ui| {
        ui.add(egui::Slider::new(&mut s.tilt_width, 0.0..=1.0).text("Tilt widens stroke"));
        ui.checkbox(&mut s.calligraphy, "Calligraphy nib");
        ui.add_enabled_ui(s.calligraphy, |ui| {
            egui::ComboBox::from_label("Nib angle from")
                .selected_text(s.nib_angle_source.label())
                .show_ui(ui, |ui| {
                    for source in NibAngle::ALL {
                        ui.selectable_value(&mut s.nib_angle_source, source, source.label());
                    }
                });
            ui.add(egui::Slider::new(&mut s.nib_angle, -90.0..=90.0).text("Fixed nib angle (°)"));
            ui.add(egui::Slider::new(&mut s.nib_thinness, 0.05..=1.0).text("Thin edge width"));
        });
    });

    egui::CollapsingHeader::new("Eraser and hover").show(ui, |ui| {
        ui.checkbox(
            &mut s.eraser_end_switches_tool,
            "Eraser end switches to the eraser",
        );
        ui.add(
            egui::Slider::new(&mut s.eraser_pressure, 0.0..=1.0).text("Pressure sets eraser size"),
        );
        ui.checkbox(&mut s.hover_fade, "Fade brush outline with hover height");
        ui.checkbox(
            &mut s.remember_tool_per_pen,
            "Each pen remembers its tool (by serial)",
        );
    });

    ui.separator();
    if ui.button("Reset pen settings").clicked() {
        let defaults = TabletSettings::default();
        *s = TabletSettings {
            activation: s.activation,
            stylus_buttons: s.stylus_buttons,
            pad_buttons: s.pad_buttons.clone(),
            ring_modes: s.ring_modes.clone(),
            ring_step_degrees: s.ring_step_degrees,
            dial_actions: s.dial_actions.clone(),
            dial_detents_per_step: s.dial_detents_per_step,
            dial_reverse: s.dial_reverse,
            strip_modes: s.strip_modes.clone(),
            pinch_zoom: s.pinch_zoom,
            ..defaults
        };
    }
}

/// Buttons & dials tab: pen buttons, tablet keys, dials, rings, strips and
/// pinch, for any connected device.
fn control_settings(ui: &mut egui::Ui, ui_state: &mut UiState) {
    let pads = &ui_state.tablet_devices.pads;
    let detected_modes = pads.iter().map(|p| p.modes).max().unwrap_or(0) as usize;
    let detected_buttons = pads.iter().map(|p| p.buttons).max().unwrap_or(0) as usize;
    let detected_rings = pads.iter().map(|p| p.rings).sum::<usize>();
    let has_strips = pads.iter().any(|p| p.strips > 0);
    let detected_dials = pads
        .iter()
        .map(|p| p.dials)
        .sum::<usize>()
        .max(ui_state.tablet_evdev_dials);
    let dial_access_denied = ui_state.tablet_evdev_denied && detected_dials == 0;
    let last_pad_button = ui_state.tablet_last_pad_button;
    let s = &mut ui_state.tablet_settings;

    egui::CollapsingHeader::new("Pen buttons")
        .default_open(true)
        .show(ui, |ui| {
            for (i, name) in ["Button 1 (lower)", "Button 2 (upper)", "Button 3"]
                .iter()
                .enumerate()
            {
                button_action_combo(ui, name, &mut s.stylus_buttons[i]);
            }
        });

    egui::CollapsingHeader::new("Tablet keys (ExpressKeys)")
        .default_open(true)
        .show(ui, |ui| {
            match last_pad_button {
                Some(n) => ui.label(format!("Last pressed: key {}", n + 1)),
                None => ui.label("Press a key on the tablet to see its number."),
            };
            let count = detected_buttons.max(s.pad_buttons.len());
            s.pad_buttons.resize(count, ButtonAction::None);
            for (i, action) in s.pad_buttons.iter_mut().enumerate() {
                button_action_combo(ui, &format!("Key {}", i + 1), action);
            }
        });

    egui::CollapsingHeader::new("Dials")
        .default_open(true)
        .show(ui, |ui| {
            if dial_access_denied {
                ui.colored_label(
                    Color32::from_rgb(234, 179, 8),
                    "Your tablet's dials can't be read: no permission. Run \
                     `./install.sh --tablet-access` in the DrafftInk source folder, \
                     then reconnect the tablet.",
                );
            } else if detected_dials == 0 {
                ui.label("No dials detected.");
            }
            let count = detected_dials.max(s.dial_actions.len());
            s.dial_actions.resize(count, RingAction::None);
            for (i, action) in s.dial_actions.iter_mut().enumerate() {
                ring_action_editor(ui, &format!("Dial {}", i + 1), action);
            }
            ui.add(
                egui::Slider::new(&mut s.dial_detents_per_step, 0.25..=4.0).text("Clicks per step"),
            );
            ui.checkbox(&mut s.dial_reverse, "Reverse direction");
        });

    egui::CollapsingHeader::new("Touch ring and strip")
        .default_open(detected_rings > 0 || has_strips)
        .show(ui, |ui| {
            if detected_rings == 0 && !has_strips {
                ui.label("No touch ring or strip detected.");
            }
            ui.label("One action per mode (switched by the tablet's mode button):");
            let modes = detected_modes.max(s.ring_modes.len()).max(1);
            s.ring_modes.resize(modes, RingAction::None);
            for (i, action) in s.ring_modes.iter_mut().enumerate() {
                ring_action_editor(ui, &format!("Ring mode {}", i + 1), action);
            }
            if has_strips || !s.strip_modes.is_empty() {
                let modes = detected_modes.max(s.strip_modes.len()).max(1);
                s.strip_modes.resize(modes, RingAction::None);
                for (i, action) in s.strip_modes.iter_mut().enumerate() {
                    ring_action_editor(ui, &format!("Strip mode {}", i + 1), action);
                }
            }
            ui.add(
                egui::Slider::new(&mut s.ring_step_degrees, 5.0..=90.0)
                    .text("Ring degrees per step"),
            );
        });

    egui::CollapsingHeader::new("Touch gestures").show(ui, |ui| {
        ui.checkbox(
            &mut s.pinch_zoom,
            "Pinch to zoom (tablet touch surface or touchpad)",
        );
    });

    ui.separator();
    if ui.button("Reset buttons & dials").clicked() {
        let defaults = TabletSettings::default();
        s.stylus_buttons = defaults.stylus_buttons;
        s.pad_buttons = defaults.pad_buttons;
        s.ring_modes = defaults.ring_modes;
        s.ring_step_degrees = defaults.ring_step_degrees;
        s.dial_actions = defaults.dial_actions;
        s.dial_detents_per_step = defaults.dial_detents_per_step;
        s.dial_reverse = defaults.dial_reverse;
        s.strip_modes = defaults.strip_modes;
        s.pinch_zoom = defaults.pinch_zoom;
    }
}

/// Detected devices and whether the settings currently apply.
fn tablet_status(ui: &mut egui::Ui, ui_state: &UiState) {
    let tablets = &ui_state.tablet_devices.tablets;
    if tablets.is_empty() {
        ui.label("No tablet detected.");
    }
    for tablet in tablets {
        let vendor = if tablet.vendor_id == WACOM_VENDOR_ID {
            " — Wacom"
        } else {
            ""
        };
        ui.label(format!(
            "{} ({:04x}:{:04x}){vendor}",
            tablet.name, tablet.vendor_id, tablet.product_id
        ));
    }
    for pad in &ui_state.tablet_devices.pads {
        ui.label(format!(
            "Controls: {} keys, {} dial(s), {} ring(s), {} strip(s), {} mode(s)",
            pad.buttons, pad.dials, pad.rings, pad.strips, pad.modes
        ));
    }
    if let Some(pen) = ui_state.tablet_pen {
        let mut axes = Vec::new();
        if pen.has_tilt {
            axes.push("tilt");
        }
        if pen.has_rotation {
            axes.push("rotation");
        }
        if pen.has_distance {
            axes.push("hover height");
        }
        ui.label(format!(
            "Pen: {:?}, serial {:#x}, reports {}",
            pen.kind,
            pen.serial,
            if axes.is_empty() {
                "pressure only".to_string()
            } else {
                format!("pressure, {}", axes.join(", "))
            }
        ));
    }
    let vendors: Vec<u32> = tablets.iter().map(|t| t.vendor_id).collect();
    let active = ui_state.tablet_settings.applies(&vendors);
    let (text, color) = if active {
        ("Settings active", Color32::from_rgb(34, 197, 94))
    } else {
        (
            "Settings inactive — tablet behaves as a plain pen",
            Color32::from_rgb(148, 163, 184),
        )
    };
    ui.colored_label(color, text);
}

/// Small plot of raw → applied pressure.
fn pressure_curve_plot(ui: &mut egui::Ui, settings: &TabletSettings) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(160.0, 90.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_stroke(
        rect,
        2.0,
        egui::Stroke::new(1.0_f32, Color32::from_gray(120)),
        egui::StrokeKind::Inside,
    );
    let points: Vec<Pos2> = (0..=40)
        .map(|i| {
            let raw = i as f64 / 40.0;
            let out = settings.map_pressure(raw);
            Pos2::new(
                rect.left() + raw as f32 * rect.width(),
                rect.bottom() - out as f32 * rect.height(),
            )
        })
        .collect();
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0_f32, Color32::from_rgb(99, 102, 241)),
    ));
}

fn button_action_combo(ui: &mut egui::Ui, label: &str, action: &mut ButtonAction) {
    egui::ComboBox::from_label(label)
        .selected_text(action.label())
        .height(420.0)
        .show_ui(ui, |ui| {
            for (group, choices) in ButtonAction::choice_groups() {
                ui.label(egui::RichText::new(group).small().strong());
                for choice in choices {
                    ui.selectable_value(action, choice, choice.label());
                }
            }
        });
}

/// Choose a dial/ring action; for per-direction commands, also choose them.
fn ring_action_editor(ui: &mut egui::Ui, label: &str, action: &mut RingAction) {
    egui::ComboBox::from_label(label)
        .selected_text(action.label())
        .show_ui(ui, |ui| {
            for choice in RingAction::ALL {
                // Keep the chosen commands when re-selecting "Commands".
                let selected = action.same_kind(choice);
                if ui.selectable_label(selected, choice.label()).clicked() && !selected {
                    *action = choice;
                }
            }
        });
    if let RingAction::Commands {
        clockwise,
        anticlockwise,
    } = action
    {
        ui.indent(label, |ui| {
            command_combo(ui, &format!("{label}: clockwise"), clockwise);
            command_combo(ui, &format!("{label}: anticlockwise"), anticlockwise);
        });
    }
}

fn command_combo(ui: &mut egui::Ui, label: &str, command: &mut Command) {
    egui::ComboBox::from_label(label)
        .selected_text(command.label())
        .height(420.0)
        .show_ui(ui, |ui| {
            for &(group, commands) in COMMAND_GROUPS {
                ui.label(egui::RichText::new(group).small().strong());
                for &choice in commands {
                    ui.selectable_value(command, choice, choice.label());
                }
            }
        });
}
