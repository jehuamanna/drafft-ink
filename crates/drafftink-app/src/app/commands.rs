//! Keyboard (and tablet-mapped) commands: what each [`Command`] does.

use drafftink_core::shapes::Shape;
use drafftink_core::tools::ToolKind;
use kurbo::Vec2;

#[cfg(target_arch = "wasm32")]
use super::spawn_png_export_async;
use super::{App, AppState, file_ops, place_shapes_centered_at};
#[cfg(not(target_arch = "wasm32"))]
use super::{encode_png, render_scene_to_png};
use winit::event::{ElementState, KeyEvent};

use crate::keybindings::{Command, KeyChord};
use crate::ui::command_action;

/// A "… / back" view command in effect: the view (camera offset and zoom)
/// before it ran, and the view it produced once applied.
pub(super) struct ViewToggle {
    pub(super) command: Command,
    pub(super) before: (Vec2, f64),
    pub(super) after: Option<(Vec2, f64)>,
}

/// Whether two views (offset, zoom) are the same, allowing for rounding.
fn same_view(a: (Vec2, f64), b: (Vec2, f64)) -> bool {
    (a.0 - b.0).hypot() < 0.5 && (a.1 - b.1).abs() < 1e-6
}

/// Open the quick toolbars at the pointer, or close them if open.
fn toggle_quick_toolbars(state: &mut AppState) {
    let now = state.egui_ctx.input(|i| i.time);
    state.ui_state.quick_bars = match state.ui_state.quick_bars {
        Some(_) => None,
        None => Some(crate::ui::QuickBars::new(now)),
    };
    state.needs_redraw = true;
    state.window.request_redraw();
}

/// Keys handled before anything else: capturing a new shortcut, and the
/// quick toolbars key. Returns true if the key was used.
pub(super) fn intercept_key(state: &mut AppState, event: &KeyEvent) -> bool {
    // Capturing a new shortcut in the settings: the next key
    // press (with modifiers) becomes the binding.
    if state.ui_state.key_capture.is_some() && event.state == ElementState::Pressed {
        let chord = KeyChord::from_event(
            event,
            state.input.ctrl(),
            state.input.shift(),
            state.input.alt(),
        );
        if let (Some(chord), Some(command)) = (chord, state.ui_state.key_capture) {
            state.ui_state.key_capture = None;
            let label = chord.label();
            state.ui_state.key_capture_note = state
                .ui_state
                .keybindings
                .bind(command, chord)
                .map(|previous| format!("{label} was moved from \"{}\"", previous.label()));
        }
        state.window.request_redraw();
        return true;
    }

    // The quick toolbars key toggles them even with the pointer
    // over the toolbars it opened (unless typing in a text field).
    if state.event_handler.editing_text.is_none()
        && event.state == ElementState::Pressed
        && !state.egui_ctx.wants_keyboard_input()
    {
        let chord = KeyChord::from_event(
            event,
            state.input.ctrl(),
            state.input.shift(),
            state.input.alt(),
        );
        if chord.is_some_and(|chord| {
            state.ui_state.keybindings.command_for(&chord) == Some(Command::QuickToolbars)
        }) {
            if !event.repeat {
                toggle_quick_toolbars(state);
            }
            return true;
        }
    }
    false
}

/// Mouse buttons handled before the canvas: a press outside the quick
/// toolbars closes them. Returns true if the button was used.
pub(super) fn intercept_mouse_button(state: &mut AppState, btn_state: ElementState) -> bool {
    // A press outside the quick toolbars just closes them.
    if btn_state == ElementState::Pressed && state.ui_state.quick_bars.is_some() {
        state.ui_state.quick_bars = None;
        state.extra.suppress_mouse_release = true;
        state.window.request_redraw();
        return true;
    }
    if btn_state == ElementState::Released
        && std::mem::take(&mut state.extra.suppress_mouse_release)
    {
        return true;
    }
    false
}

impl App {
    /// Run a keyboard (or tablet-mapped) command. Returns false if it
    /// did nothing because the app isn't ready.
    pub(super) fn run_command(&mut self, command: Command, egui_wants_input: bool) -> bool {
        let Some(state) = &mut self.state else {
            return false;
        };
        if command == Command::QuickToolbars {
            toggle_quick_toolbars(state);
            return true;
        }
        if command == Command::Cancel && state.ui_state.quick_bars.take().is_some() {
            state.window.request_redraw();
            return true;
        }
        // "… / back" view commands: pressed again with the view unchanged
        // since, return to the view before the first press.
        if let Some(base) = command.view_toggle_base() {
            let camera = &mut state.canvas.camera;
            let current = (camera.offset, camera.zoom);
            if let Some(toggle) = state.extra.view_toggle.take() {
                if toggle.command == command && toggle.after.is_some_and(|a| same_view(a, current))
                {
                    (camera.offset, camera.zoom) = toggle.before;
                    state.ui_state.zoom_level = camera.zoom;
                    state.needs_redraw = true;
                    state.window.request_redraw();
                    return true;
                }
            }
            state.extra.view_toggle = Some(ViewToggle {
                command,
                before: current,
                after: None,
            });
            return self.run_command(base, egui_wants_input);
        }

        // Commands that are also UI controls run like clicking them.
        if let Some(action) = command_action(command) {
            state.extra.queued_actions.push_back(action);
            state.window.request_redraw();
            return true;
        }
        match command {
            Command::Tool(tool) => {
                state.canvas.set_tool(tool);
                log::info!("Tool: {tool:?}");
            }
            Command::Brush(brush) => {
                state.canvas.set_tool(ToolKind::Freehand);
                state.canvas.tool_manager.brush = brush;
                state.needs_redraw = true;
                state.window.request_redraw();
                log::info!("Brush: {brush:?}");
            }
            Command::Settings => {
                state.ui_state.settings_open = true;
            }
            Command::SelectAll => {
                state.canvas.select_all();
                log::info!("Selected all {} shapes", state.canvas.selection.len());
            }
            #[cfg(target_arch = "wasm32")]
            Command::Save => {
                file_ops::save_document(&state.canvas.document, &state.canvas.document.name);
            }
            Command::Open => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    file_ops::load_document();
                }
                #[cfg(target_arch = "wasm32")]
                {
                    file_ops::load_document_async();
                }
            }
            Command::ExportPng => {
                if let Some(render_cx) = self.render_cx.as_ref() {
                    let device_handle = &render_cx.devices[state.surface.dev_id];
                    let device = &device_handle.device;
                    let queue = &device_handle.queue;
                    let export_scale = state.ui_state.export_scale as f64;

                    let (scene, bounds) = if state.canvas.selection.is_empty() {
                        state
                            .shape_renderer
                            .build_export_scene(&state.canvas.document, export_scale)
                    } else {
                        state.shape_renderer.build_export_scene_selection(
                            &state.canvas.document,
                            &state.canvas.selection,
                            export_scale,
                        )
                    };
                    if let Some(bounds) = bounds {
                        let width = bounds.width().ceil() as u32;
                        let height = bounds.height().ceil() as u32;

                        log::info!(
                            "Exporting PNG at {}x scale: {}x{}",
                            state.ui_state.export_scale,
                            width,
                            height
                        );

                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            if let Some(result) = render_scene_to_png(
                                device,
                                queue,
                                &mut state.vello_renderer,
                                &scene,
                                width,
                                height,
                            ) {
                                let scene_json = state.canvas.document.to_json().ok();
                                if let Some(png_data) = encode_png(
                                    &result.rgba_data,
                                    result.width,
                                    result.height,
                                    scene_json.as_deref(),
                                ) {
                                    file_ops::export_png(&png_data, &state.canvas.document.name);
                                }
                            }
                        }

                        #[cfg(target_arch = "wasm32")]
                        {
                            let filename = format!("{}.png", state.canvas.document.name);
                            let scene_json = state.canvas.document.to_json().ok();
                            spawn_png_export_async(
                                device, queue, scene, width, height, filename, false, scene_json,
                            );
                        }
                    } else {
                        log::info!("Nothing to export - document is empty");
                    }
                }
            }
            Command::CopyPng => {
                if let Some(render_cx) = self.render_cx.as_ref() {
                    let device_handle = &render_cx.devices[state.surface.dev_id];
                    let device = &device_handle.device;
                    let queue = &device_handle.queue;
                    let export_scale = state.ui_state.export_scale as f64;

                    let (scene, bounds) = if state.canvas.selection.is_empty() {
                        state
                            .shape_renderer
                            .build_export_scene(&state.canvas.document, export_scale)
                    } else {
                        state.shape_renderer.build_export_scene_selection(
                            &state.canvas.document,
                            &state.canvas.selection,
                            export_scale,
                        )
                    };

                    if let Some(bounds) = bounds {
                        let width = bounds.width().ceil() as u32;
                        let height = bounds.height().ceil() as u32;

                        log::info!(
                            "Copying PNG at {}x scale: {}x{}",
                            state.ui_state.export_scale,
                            width,
                            height
                        );

                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            if let Some(result) = render_scene_to_png(
                                device,
                                queue,
                                &mut state.vello_renderer,
                                &scene,
                                width,
                                height,
                            ) {
                                file_ops::copy_png_to_clipboard(
                                    &result.rgba_data,
                                    result.width,
                                    result.height,
                                );
                            }
                        }

                        #[cfg(target_arch = "wasm32")]
                        {
                            spawn_png_export_async(
                                device,
                                queue,
                                scene,
                                width,
                                height,
                                "selection.png".to_string(),
                                true,
                                None, // No metadata for clipboard copy
                            );
                        }
                    } else {
                        log::info!("Nothing to copy - selection is empty");
                    }
                }
            }
            Command::Undo => {
                // Undo
                if state.canvas.document.undo() {
                    state.canvas.clear_selection();
                    log::info!("Undo performed");
                } else {
                    log::info!("Nothing to undo");
                }
            }
            Command::Redo => {
                if state.canvas.document.redo() {
                    state.canvas.clear_selection();
                    log::info!("Redo performed");
                } else {
                    log::info!("Nothing to redo");
                }
            }
            Command::Copy => {
                if !state.canvas.selection.is_empty() {
                    let shapes: Vec<Shape> = state
                        .canvas
                        .selection
                        .iter()
                        .filter_map(|&id| state.canvas.document.get_shape(id).cloned())
                        .collect();
                    if let Ok(json) = serde_json::to_string(&shapes) {
                        state.ui_state.clipboard_shapes = Some(json);
                        log::info!("Copied {} shapes", shapes.len());
                    }
                }
            }
            Command::Cut => {
                if !state.canvas.selection.is_empty() {
                    let shapes: Vec<Shape> = state
                        .canvas
                        .selection
                        .iter()
                        .filter_map(|&id| state.canvas.document.get_shape(id).cloned())
                        .collect();
                    if let Ok(json) = serde_json::to_string(&shapes) {
                        state.ui_state.clipboard_shapes = Some(json);
                        log::info!("Cut {} shapes", shapes.len());
                        state.canvas.document.push_undo();
                        for &id in &state.canvas.selection.clone() {
                            state.canvas.document.remove_shape(id);
                            if state.collab.is_in_room() {
                                let _ = state.collab.crdt_mut().remove_shape(&id.to_string());
                            }
                        }
                        state.canvas.clear_selection();
                    }
                }
            }
            Command::Paste => {
                // First try to paste shapes from the internal
                // clipboard, centered on the cursor so pastes
                // (including across tabs) always land in view.
                let mut pasted = false;
                if let Some(json) = &state.ui_state.clipboard_shapes.clone() {
                    if let Ok(shapes) = serde_json::from_str::<Vec<Shape>>(json) {
                        let cursor_world = state
                            .canvas
                            .camera
                            .screen_to_world(state.input.mouse_position());
                        let shapes: Vec<Shape> = shapes
                            .into_iter()
                            .map(|mut s| {
                                s.regenerate_id();
                                s
                            })
                            .collect();
                        place_shapes_centered_at(
                            &mut state.canvas,
                            &mut state.collab,
                            shapes,
                            cursor_world,
                        );
                        log::info!("Pasted shapes");
                        pasted = true;
                    }
                }

                // Try structured text formats from the
                // system clipboard (native): Excalidraw
                // scene JSON first, then Mermaid diagram
                // source. Both import as native shapes
                // centered on the mouse cursor.
                #[cfg(not(target_arch = "wasm32"))]
                if !pasted {
                    if let Ok(mut cb) = arboard::Clipboard::new() {
                        if let Ok(text) = cb.get_text() {
                            let cursor_world = state
                                .canvas
                                .camera
                                .screen_to_world(state.input.mouse_position());
                            if let Some(shapes) = drafftink_core::canvas::CanvasDocument::shapes_from_excalidraw_clipboard(&text) {
                                place_shapes_centered_at(
                                    &mut state.canvas,
                                    &mut state.collab,
                                    shapes,
                                    cursor_world,
                                );
                                log::info!("Pasted shapes from Excalidraw clipboard");
                                pasted = true;
                            } else if let Some(shapes) = drafftink_core::shapes_from_mermaid(&text) {
                                place_shapes_centered_at(
                                    &mut state.canvas,
                                    &mut state.collab,
                                    shapes,
                                    cursor_world,
                                );
                                log::info!("Imported Mermaid diagram from clipboard");
                                pasted = true;
                            }
                        }
                    }
                }

                // If no shapes, try to paste image from system clipboard
                #[cfg(not(target_arch = "wasm32"))]
                if !pasted {
                    if let Some(image_shape) = file_ops::paste_image_from_clipboard(&state.canvas) {
                        state.canvas.document.push_undo();
                        state.canvas.clear_selection();
                        let new_id = image_shape.id();
                        state.canvas.document.add_shape(image_shape.clone());
                        state.canvas.add_to_selection(new_id);
                        if state.collab.is_in_room() {
                            let _ = state.collab.crdt_mut().add_shape(&image_shape);
                        }
                        log::info!("Pasted image from clipboard");
                    }
                }

                // WASM: Try Excalidraw clipboard text first, then image
                #[cfg(target_arch = "wasm32")]
                if !pasted {
                    let cursor_world = state
                        .canvas
                        .camera
                        .screen_to_world(state.input.mouse_position());
                    file_ops::paste_shapes_from_clipboard_async(cursor_world);
                    let vw = state.canvas.viewport_size.width;
                    let vh = state.canvas.viewport_size.height;
                    let cox = state.canvas.camera.offset.x;
                    let coy = state.canvas.camera.offset.y;
                    let cz = state.canvas.camera.zoom;
                    file_ops::paste_image_from_clipboard_async(vw, vh, cox, coy, cz);
                }
            }
            Command::Duplicate => {
                if !state.canvas.selection.is_empty() {
                    state.canvas.document.push_undo();
                    let mut new_selection = Vec::new();
                    for &id in &state.canvas.selection.clone() {
                        if let Some(shape) = state.canvas.document.get_shape(id) {
                            let mut new_shape = shape.clone();
                            new_shape.regenerate_id();
                            new_shape
                                .transform(kurbo::Affine::translate(kurbo::Vec2::new(20.0, 20.0)));
                            let new_id = new_shape.id();
                            state.canvas.document.add_shape(new_shape.clone());
                            new_selection.push(new_id);
                            if state.collab.is_in_room() {
                                let _ = state.collab.crdt_mut().add_shape(&new_shape);
                            }
                        }
                    }
                    state.canvas.clear_selection();
                    for id in new_selection {
                        state.canvas.add_to_selection(id);
                    }
                    log::info!("Duplicated shapes");
                }
            }
            Command::Group => {
                // Group
                if let Some(group_id) = state.canvas.group_selected() {
                    log::info!("Grouped shapes into {:?}", group_id);
                } else {
                    log::info!("Select at least 2 shapes to group");
                }
            }
            Command::Ungroup => {
                // Ungroup
                let ungrouped = state.canvas.ungroup_selected();
                if !ungrouped.is_empty() {
                    log::info!("Ungrouped {} shapes", ungrouped.len());
                } else {
                    log::info!("No groups to ungroup");
                }
            }
            Command::Delete => {
                if !state.canvas.selection.is_empty() {
                    state.canvas.document.push_undo();
                    state.canvas.delete_selected();
                }
            }
            Command::Cancel => {
                // Skip if egui is handling this (e.g., closing its own dialogs)
                if egui_wants_input {
                    return true;
                }
                // First, close any open dialogs/popovers
                let had_open_dialog = state.ui_state.color_popover != crate::ui::ColorPopover::None
                    || state.ui_state.menu_open
                    || state.ui_state.collab_modal_open
                    || state.ui_state.settings_open
                    || state.ui_state.save_dialog_open
                    || state.ui_state.open_dialog_open
                    || state.ui_state.open_recent_dialog_open;

                if had_open_dialog {
                    state.ui_state.color_popover = crate::ui::ColorPopover::None;
                    state.ui_state.menu_open = false;
                    state.ui_state.collab_modal_open = false;
                    state.ui_state.settings_open = false;
                    state.ui_state.key_capture = None;
                    state.ui_state.save_dialog_open = false;
                    state.ui_state.open_dialog_open = false;
                    state.ui_state.open_recent_dialog_open = false;
                } else {
                    // No dialog open - switch to Select tool
                    state.canvas.tool_manager.cancel();
                    state.canvas.clear_selection();
                    state.canvas.set_tool(ToolKind::Select);
                    state.ui_state.current_tool = ToolKind::Select;
                }
            }
            Command::NudgeUp | Command::NudgeDown | Command::NudgeLeft | Command::NudgeRight => {
                let direction = match command {
                    Command::NudgeUp => kurbo::Vec2::new(0.0, -1.0),
                    Command::NudgeDown => kurbo::Vec2::new(0.0, 1.0),
                    Command::NudgeLeft => kurbo::Vec2::new(-1.0, 0.0),
                    _ => kurbo::Vec2::new(1.0, 0.0),
                };
                if !state.canvas.selection.is_empty() {
                    use drafftink_core::GRID_SIZE;
                    let delta = direction * GRID_SIZE;
                    state.canvas.document.push_undo();
                    let translation = kurbo::Affine::translate(delta);
                    for &id in &state.canvas.selection {
                        if let Some(shape) = state.canvas.document.get_shape_mut(id) {
                            shape.transform(translation);
                        }
                    }
                }
            }
            _ => {}
        }
        true
    }
}
