//! Pen tablet input: turns tablet events into canvas input according to the
//! user's tablet settings (see [`crate::tablet_settings`]).

use std::collections::HashMap;

use drafftink_core::tools::ToolKind;
use kurbo::Vec2;
use winit::application::ApplicationHandler;
use winit::event::{DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;

use super::App;
use crate::tablet::{
    Devices, OneEuroFilter, PenInfo, PenKind, PenSample, PinchPhase, StylusButton, TabletEvent,
};
use crate::tablet_settings::{ButtonAction, NibAngle, RingAction, TabletSettings};
use crate::ui::{QueuedAction, UiAction};

/// Which physical button triggered an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ButtonSource {
    Stylus(StylusButton),
    Pad(u32),
}

/// Runtime state of pen tablet input.
pub(super) struct PenInput {
    devices: Devices,
    pen: PenInfo,
    /// Pressure after mapping and smoothing (0.0-1.0).
    pressure: f64,
    filter: OneEuroFilter,
    /// Whether the tip is touching the tablet.
    down: bool,
    /// Last filtered position, for the stroke direction.
    last_position: Option<(f64, f64)>,
    /// Direction of travel in radians (screen space).
    stroke_angle: f64,
    /// Tool to restore when the pen is flipped back from its eraser end.
    tool_before_eraser: Option<ToolKind>,
    /// Button held for a temporary tool, and the tool to return to.
    held_tool: Option<(ButtonSource, ToolKind)>,
    /// Tool to return to once the pen lifts (button released mid-stroke).
    restore_tool_on_up: Option<ToolKind>,
    /// Tool to go back to when a toggle button is pressed again.
    toggled_from: Option<ToolKind>,
    /// Actions of buttons currently held, as assigned when they were pressed.
    pressed: Vec<(ButtonSource, ButtonAction)>,
    /// Last tool used by each pen, by serial number.
    pen_tools: HashMap<u64, ToolKind>,
    /// Eraser size before pressure started scaling it.
    eraser_base_radius: Option<f64>,
    /// Last touch ring angle / strip position and unconsumed movement, per
    /// control.
    rings: HashMap<usize, (f64, f64)>,
    strips: HashMap<usize, (f64, f64)>,
    /// Unconsumed dial movement in detents, per dial.
    dials: HashMap<usize, f64>,
    /// Pinch scale at the previous update.
    pinch_scale: f64,
}

impl Default for PenInput {
    fn default() -> Self {
        Self {
            devices: Devices::default(),
            pen: PenInfo::default(),
            pressure: 1.0,
            filter: OneEuroFilter::new(1.0, 0.0, 1.0),
            down: false,
            last_position: None,
            stroke_angle: 0.0,
            tool_before_eraser: None,
            held_tool: None,
            restore_tool_on_up: None,
            toggled_from: None,
            pressed: Vec::new(),
            pen_tools: HashMap::new(),
            eraser_base_radius: None,
            rings: HashMap::new(),
            strips: HashMap::new(),
            dials: HashMap::new(),
            pinch_scale: 1.0,
        }
    }
}

impl PenInput {
    fn vendor_ids(&self) -> Vec<u32> {
        self.devices.tablets.iter().map(|t| t.vendor_id).collect()
    }
}

/// Signed shortest difference between two angles in degrees.
fn angle_delta(from: f64, to: f64) -> f64 {
    (to - from + 540.0).rem_euclid(360.0) - 180.0
}

impl App {
    /// Start listening for tablet input, once the display connection exists.
    pub(super) fn start_tablet_input(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_os = "linux")]
        if let Some(proxy) = self.tablet_proxy.take() {
            crate::tablet_evdev::spawn(proxy.clone());
            use winit::raw_window_handle::{HasDisplayHandle, RawDisplayHandle};
            if let Ok(handle) = event_loop.display_handle() {
                if let RawDisplayHandle::Wayland(wayland) = handle.as_raw() {
                    // SAFETY: winit keeps its wl_display connected for the
                    // lifetime of the event loop.
                    unsafe { crate::tablet::spawn(wayland.display.as_ptr(), proxy) };
                }
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = event_loop;
    }

    /// Handle a tablet event: pen input becomes the equivalent mouse events
    /// (so the canvas and UI work with the pen), with pressure passed to the
    /// freehand tools; buttons, ring, strip and pinch run their configured
    /// actions.
    pub(super) fn handle_tablet_event(&mut self, event_loop: &ActiveEventLoop, event: TabletEvent) {
        let Some(state) = &mut self.state else {
            return;
        };

        if let TabletEvent::Devices(devices) = event {
            self.pen.devices = devices.clone();
            state.ui_state.tablet_devices = devices;
            state.window.request_redraw();
            return;
        }

        // Pad controls belong to the focused window. Dials read from the
        // device arrive whatever window is focused, so drop them here; button
        // releases still go through so held buttons don't stay held.
        if !state.extra.focused {
            match event {
                TabletEvent::PadButton { pressed: true, .. }
                | TabletEvent::Dial { .. }
                | TabletEvent::EvdevDial { .. } => return,
                TabletEvent::Ring { index, .. } => {
                    self.pen.rings.remove(&index);
                    return;
                }
                TabletEvent::Strip { index, .. } => {
                    self.pen.strips.remove(&index);
                    return;
                }
                _ => {}
            }
        }

        let active = state
            .ui_state
            .tablet_settings
            .applies(&self.pen.vendor_ids());
        state.ui_state.tablet_active = active;
        let settings = state.ui_state.tablet_settings.effective(active);

        match event {
            TabletEvent::Devices(_) => {}
            TabletEvent::ProximityIn(info) => self.pen_proximity_in(info, &settings),
            TabletEvent::Motion(sample) => self.pen_motion(event_loop, sample, &settings),
            TabletEvent::Down => self.pen_down(event_loop, &settings),
            TabletEvent::Up => self.pen_up(event_loop),
            TabletEvent::Button { button, pressed } => {
                let action = settings.stylus_buttons[button.index()];
                self.button(event_loop, ButtonSource::Stylus(button), action, pressed);
            }
            TabletEvent::ProximityOut => self.pen_proximity_out(event_loop, &settings),
            TabletEvent::PadButton { index, pressed } => {
                if pressed {
                    state.ui_state.tablet_last_pad_button = Some(index);
                    state.window.request_redraw();
                }
                let action = settings
                    .pad_buttons
                    .get(index as usize)
                    .copied()
                    .unwrap_or_default();
                self.button(event_loop, ButtonSource::Pad(index), action, pressed);
            }
            TabletEvent::Ring { index, mode, angle } => {
                let action = settings.ring_modes.get(mode as usize).copied();
                let steps = Self::control_steps(
                    &mut self.pen.rings,
                    index,
                    angle,
                    settings.ring_step_degrees,
                    angle_delta,
                );
                self.ring_action(action.unwrap_or_default(), steps);
            }
            TabletEvent::Strip {
                index,
                mode,
                position,
            } => {
                let action = settings.strip_modes.get(mode as usize).copied();
                // The strip's length counts as a full turn of the ring.
                let steps = Self::control_steps(
                    &mut self.pen.strips,
                    index,
                    position.map(|p| p * 360.0),
                    settings.ring_step_degrees,
                    |from, to| to - from,
                );
                self.ring_action(action.unwrap_or_default(), steps);
            }
            TabletEvent::Dial { index, detents, .. } => self.dial(index, detents, &settings),
            TabletEvent::EvdevStatus {
                dials,
                permission_denied,
            } => {
                state.ui_state.tablet_evdev_dials = dials;
                state.ui_state.tablet_evdev_denied = permission_denied;
                state.window.request_redraw();
            }
            TabletEvent::EvdevDial { index, detents } => {
                // Only needed when the compositor doesn't deliver dials itself.
                let protocol_dials = self.pen.devices.pads.iter().any(|p| p.dials > 0);
                if !protocol_dials {
                    self.dial(index, detents, &settings);
                }
            }
            TabletEvent::Pinch(phase) => {
                if settings.pinch_zoom {
                    self.pinch(phase);
                }
            }
        }
    }

    /// Deliver a synthesized window event as if it came from the window.
    fn send_window_event(&mut self, event_loop: &ActiveEventLoop, event: WindowEvent) {
        let Some(state) = &self.state else {
            return;
        };
        let window_id = state.window.id();
        self.window_event(event_loop, window_id, event);
    }

    fn send_mouse_button(
        &mut self,
        event_loop: &ActiveEventLoop,
        button: MouseButton,
        pressed: bool,
    ) {
        let event = WindowEvent::MouseInput {
            device_id: DeviceId::dummy(),
            state: if pressed {
                ElementState::Pressed
            } else {
                ElementState::Released
            },
            button,
        };
        self.send_window_event(event_loop, event);
    }

    fn set_tool(&mut self, tool: ToolKind) {
        if let Some(state) = &mut self.state {
            state.canvas.set_tool(tool);
            state.needs_redraw = true;
            state.window.request_redraw();
        }
    }

    fn current_tool(&self) -> ToolKind {
        self.state
            .as_ref()
            .map_or(ToolKind::default(), |s| s.canvas.tool_manager.current_tool)
    }

    fn pen_proximity_in(&mut self, info: PenInfo, settings: &TabletSettings) {
        self.pen.filter.reset();
        self.pen.last_position = None;
        self.pen.pen = info;
        if let Some(state) = &mut self.state {
            state.ui_state.tablet_pen = Some(info);
        }

        let current = self.current_tool();
        if settings.remember_tool_per_pen && info.serial != 0 {
            if let Some(&tool) = self.pen.pen_tools.get(&info.serial) {
                self.set_tool(tool);
                return;
            }
        }
        // The pen's eraser end selects the eraser; the tip restores the
        // previous tool unless the user picked another one meanwhile.
        if settings.eraser_end_switches_tool {
            if info.kind == PenKind::Eraser {
                if current != ToolKind::Eraser {
                    self.pen.tool_before_eraser = Some(current);
                    self.set_tool(ToolKind::Eraser);
                }
            } else if let Some(previous) = self.pen.tool_before_eraser.take() {
                if current == ToolKind::Eraser {
                    self.set_tool(previous);
                }
            }
        }
    }

    fn pen_proximity_out(&mut self, event_loop: &ActiveEventLoop, settings: &TabletSettings) {
        if settings.remember_tool_per_pen && self.pen.pen.serial != 0 {
            let tool = self.current_tool();
            self.pen.pen_tools.insert(self.pen.pen.serial, tool);
        }
        if let Some(state) = &mut self.state {
            state.extra.brush_cursor_opacity = 1.0;
        }
        if self.pen.down {
            self.pen_up(event_loop);
        }
    }

    fn pen_motion(
        &mut self,
        event_loop: &ActiveEventLoop,
        sample: PenSample,
        settings: &TabletSettings,
    ) {
        // Position: optionally steadied against jitter.
        let (x, y) = if settings.stabilizer {
            self.pen.filter.min_cutoff = settings.stabilizer_min_cutoff.max(0.01);
            self.pen.filter.beta = settings.stabilizer_speed.max(0.0);
            self.pen.filter.filter((sample.x, sample.y), sample.time)
        } else {
            self.pen.filter.reset();
            (sample.x, sample.y)
        };
        if let Some((px, py)) = self.pen.last_position {
            // Ignore sub-pixel moves so the direction doesn't jitter.
            if (x - px).hypot(y - py) > 0.5 {
                self.pen.stroke_angle = (y - py).atan2(x - px);
                self.pen.last_position = Some((x, y));
            }
        } else {
            self.pen.last_position = Some((x, y));
        }

        // Pressure: mapped through the curve, shaped by tilt and nib.
        let mut pressure = settings.map_pressure(sample.pressure);
        if self.pen.pen.has_tilt {
            pressure *= settings.tilt_factor(sample.tilt);
        }
        let nib_angle = match settings.nib_angle_source {
            NibAngle::TiltDirection if sample.tilt != (0.0, 0.0) => {
                sample.tilt.1.atan2(sample.tilt.0)
            }
            NibAngle::PenRotation if self.pen.pen.has_rotation => sample.rotation.to_radians(),
            _ => settings.nib_angle.to_radians(),
        };
        // The Broad nib brush shapes strokes by direction itself.
        let broad_nib = self.state.as_ref().is_some_and(|state| {
            state.canvas.tool_manager.brush == drafftink_core::shapes::Brush::BroadNib
        });
        if !broad_nib {
            pressure *= settings.nib_factor(self.pen.stroke_angle, nib_angle);
        }

        let Some(state) = &mut self.state else {
            return;
        };
        if self.pen.down {
            // Smoothing removes flicker; touch-down uses the raw value so
            // strokes start crisp.
            let alpha = 1.0 - settings.pressure_smoothing.clamp(0.0, 0.95);
            self.pen.pressure += alpha * (pressure - self.pen.pressure);
            state.canvas.tool_manager.stylus_pressure = Some(self.pen.pressure);
            if let Some(base) = self.pen.eraser_base_radius {
                let amount = settings.eraser_pressure.clamp(0.0, 1.0);
                state.event_handler.eraser_radius =
                    base * ((1.0 - amount) + amount * self.pen.pressure);
            }
        } else {
            self.pen.pressure = pressure;
        }

        state.extra.brush_cursor_opacity =
            if settings.hover_fade && self.pen.pen.has_distance && !self.pen.down {
                (1.0 - sample.distance).clamp(0.15, 1.0)
            } else {
                1.0
            };

        let scale = state.window.scale_factor();
        let event = WindowEvent::CursorMoved {
            device_id: DeviceId::dummy(),
            position: winit::dpi::PhysicalPosition::new(x * scale, y * scale),
        };
        self.send_window_event(event_loop, event);
    }

    fn pen_down(&mut self, event_loop: &ActiveEventLoop, settings: &TabletSettings) {
        self.pen.down = true;
        let Some(state) = &mut self.state else {
            return;
        };
        state.canvas.tool_manager.stylus_pressure = Some(self.pen.pressure);
        if settings.eraser_pressure > 0.0
            && state.canvas.tool_manager.current_tool == ToolKind::Eraser
        {
            let base = state.event_handler.eraser_radius;
            self.pen.eraser_base_radius = Some(base);
            let amount = settings.eraser_pressure.clamp(0.0, 1.0);
            state.event_handler.eraser_radius =
                base * ((1.0 - amount) + amount * self.pen.pressure);
        }
        self.send_mouse_button(event_loop, MouseButton::Left, true);
    }

    fn pen_up(&mut self, event_loop: &ActiveEventLoop) {
        self.pen.down = false;
        self.send_mouse_button(event_loop, MouseButton::Left, false);
        if let Some(state) = &mut self.state {
            state.canvas.tool_manager.stylus_pressure = None;
            if let Some(base) = self.pen.eraser_base_radius.take() {
                state.event_handler.eraser_radius = base;
            }
        }
        if let Some(tool) = self.pen.restore_tool_on_up.take() {
            self.set_tool(tool);
        }
    }

    /// Run a pen or tablet button's action. Releases use the action assigned
    /// when the button was pressed, even if the settings changed since.
    fn button(
        &mut self,
        event_loop: &ActiveEventLoop,
        source: ButtonSource,
        action: ButtonAction,
        pressed: bool,
    ) {
        let action = if pressed {
            self.pen.pressed.retain(|(s, _)| *s != source);
            self.pen.pressed.push((source, action));
            action
        } else {
            match self.pen.pressed.iter().position(|(s, _)| *s == source) {
                Some(i) => self.pen.pressed.remove(i).1,
                None => return,
            }
        };

        let current = self.current_tool();
        match action {
            ButtonAction::None => {}
            ButtonAction::Pan => self.send_mouse_button(event_loop, MouseButton::Middle, pressed),
            ButtonAction::RightClick => {
                self.send_mouse_button(event_loop, MouseButton::Right, pressed)
            }
            ButtonAction::HoldTool(tool) => {
                if pressed {
                    // Switching mid-stroke would break the stroke in progress.
                    if self.pen.down || self.pen.held_tool.is_some() {
                        return;
                    }
                    self.pen.held_tool = Some((source, current));
                    self.set_tool(tool);
                } else if let Some((held, previous)) = self.pen.held_tool {
                    if held == source {
                        self.pen.held_tool = None;
                        if self.pen.down {
                            self.pen.restore_tool_on_up = Some(previous);
                        } else {
                            self.set_tool(previous);
                        }
                    }
                }
            }
            _ if !pressed => {}
            ButtonAction::SwitchTool(tool) => {
                if !self.pen.down {
                    self.set_tool(tool);
                }
            }
            ButtonAction::ToggleTool(tool) => {
                if self.pen.down {
                    return;
                }
                if current == tool {
                    let previous = self.pen.toggled_from.take().unwrap_or(ToolKind::Freehand);
                    self.set_tool(previous);
                } else {
                    self.pen.toggled_from = Some(current);
                    self.set_tool(tool);
                }
            }
            ButtonAction::Command(command) => {
                self.run_command(command, false);
            }
        }
    }

    /// Whole steps moved by a ring or strip since the last event, keeping the
    /// remainder for next time. `value` is `None` when the finger lifts.
    fn control_steps(
        controls: &mut HashMap<usize, (f64, f64)>,
        index: usize,
        value: Option<f64>,
        step: f64,
        delta: impl Fn(f64, f64) -> f64,
    ) -> i32 {
        let Some(value) = value else {
            controls.remove(&index);
            return 0;
        };
        let step = step.max(1.0);
        let Some((last, remainder)) = controls.get(&index).copied() else {
            controls.insert(index, (value, 0.0));
            return 0;
        };
        let total = remainder + delta(last, value);
        let steps = (total / step).trunc();
        controls.insert(index, (value, total - steps * step));
        steps as i32
    }

    /// Apply `steps` of dial/ring/strip movement (positive is clockwise /
    /// down).
    fn ring_action(&mut self, action: RingAction, steps: i32) {
        if steps == 0 {
            return;
        }
        if let RingAction::Commands {
            clockwise,
            anticlockwise,
        } = action
        {
            let command = if steps > 0 { clockwise } else { anticlockwise };
            for _ in 0..steps.unsigned_abs() {
                self.run_command(command, false);
            }
            return;
        }
        let Some(state) = &mut self.state else {
            return;
        };
        let cursor = state.input.mouse_position();
        match action {
            RingAction::None | RingAction::Commands { .. } => {}
            RingAction::Zoom => state.canvas.camera.zoom_at(cursor, 1.1f64.powi(steps)),
            RingAction::ScrollVertical => state
                .canvas
                .camera
                .pan(Vec2::new(0.0, -40.0 * steps as f64)),
            RingAction::ScrollHorizontal => state
                .canvas
                .camera
                .pan(Vec2::new(-40.0 * steps as f64, 0.0)),
            RingAction::StrokeWidth => state
                .extra
                .queued_actions
                .push_back(QueuedAction::StepWidth(steps as isize)),
            RingAction::StrokeColor => state
                .extra
                .queued_actions
                .push_back(QueuedAction::CycleColor(steps as isize)),
            RingAction::UndoRedo => {
                let action = if steps > 0 {
                    UiAction::Redo
                } else {
                    UiAction::Undo
                };
                for _ in 0..steps.unsigned_abs() {
                    state
                        .extra
                        .queued_actions
                        .push_back(QueuedAction::Ui(action.clone()));
                }
            }
        }
        state.needs_redraw = true;
        state.window.request_redraw();
    }

    /// Turn dial `index` by `detents`, running its action once per step.
    fn dial(&mut self, index: usize, detents: f64, settings: &TabletSettings) {
        let detents = if settings.dial_reverse {
            -detents
        } else {
            detents
        };
        let action = settings.dial_actions.get(index).copied();
        let per_step = settings.dial_detents_per_step.max(0.1);
        let total = self.pen.dials.get(&index).copied().unwrap_or(0.0) + detents;
        let steps = (total / per_step).trunc();
        self.pen.dials.insert(index, total - steps * per_step);
        self.ring_action(action.unwrap_or_default(), steps as i32);
    }

    fn pinch(&mut self, phase: PinchPhase) {
        let Some(state) = &mut self.state else {
            return;
        };
        match phase {
            PinchPhase::Begin | PinchPhase::End => self.pen.pinch_scale = 1.0,
            PinchPhase::Update { scale } => {
                if scale > 0.0 && self.pen.pinch_scale > 0.0 {
                    let cursor = state.input.mouse_position();
                    state
                        .canvas
                        .camera
                        .zoom_at(cursor, scale / self.pen.pinch_scale);
                    self.pen.pinch_scale = scale;
                    state.needs_redraw = true;
                    state.window.request_redraw();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_angle_delta_wraps() {
        assert_eq!(angle_delta(350.0, 10.0), 20.0);
        assert_eq!(angle_delta(10.0, 350.0), -20.0);
        assert_eq!(angle_delta(90.0, 100.0), 10.0);
    }

    #[test]
    fn test_control_steps_accumulate() {
        let mut controls = HashMap::new();
        let mut steps = |v| App::control_steps(&mut controls, 0, v, 15.0, angle_delta);
        assert_eq!(steps(Some(0.0)), 0);
        assert_eq!(steps(Some(10.0)), 0);
        assert_eq!(steps(Some(20.0)), 1); // 20° total: one step, 5° left over
        assert_eq!(steps(Some(355.0)), -1); // back 25°: -20° total
        assert_eq!(steps(None), 0);
        assert_eq!(steps(Some(100.0)), 0); // fresh start after lift
    }
}
