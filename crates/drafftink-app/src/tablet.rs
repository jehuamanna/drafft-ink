//! Graphics-tablet (stylus) input.
//!
//! winit doesn't bind the Wayland tablet protocol, and compositors such as
//! Hyprland don't emulate pointer buttons for a pen tip, so without this a pen
//! can move the cursor but never draw. On Wayland we share winit's
//! `wl_display`, listen for tablet (and touch pinch gesture) events on a
//! background thread and forward them to the event loop as user events.

/// Event loop user event.
#[derive(Debug, Clone)]
pub enum UserEvent {
    Tablet(TabletEvent),
}

/// A tablet event. Positions are surface-local logical coordinates.
#[derive(Debug, Clone)]
pub enum TabletEvent {
    /// The set of connected tablets and pads changed.
    Devices(Devices),
    /// A pen (or its eraser end) entered the window.
    ProximityIn(PenInfo),
    /// Pen moved or its axes changed.
    Motion(PenSample),
    /// Tip touched the surface.
    Down,
    /// Tip lifted off the surface.
    Up,
    /// Barrel button pressed or released.
    Button { button: StylusButton, pressed: bool },
    /// Pen left the window.
    ProximityOut,
    /// Tablet button (ExpressKey) pressed or released.
    PadButton { index: u32, pressed: bool },
    /// Touch ring position in degrees, or `None` when the finger lifts.
    Ring {
        index: usize,
        mode: u32,
        angle: Option<f64>,
    },
    /// Dial turned by `detents` (fractional on high-resolution dials);
    /// positive is clockwise.
    Dial {
        index: usize,
        mode: u32,
        detents: f64,
    },
    /// Pad dials found by reading the device directly, and whether reading
    /// a pad was refused for lack of permission.
    EvdevStatus {
        dials: usize,
        permission_denied: bool,
    },
    /// Dial turned, read directly from the device.
    EvdevDial { index: usize, detents: f64 },
    /// Touch strip position (0.0-1.0), or `None` when the finger lifts.
    Strip {
        index: usize,
        mode: u32,
        position: Option<f64>,
    },
    /// Two-finger pinch: `scale` is relative to the start of the gesture.
    Pinch(PinchPhase),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PinchPhase {
    Begin,
    Update { scale: f64 },
    End,
}

/// Connected tablets and their control pads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Devices {
    pub tablets: Vec<TabletInfo>,
    pub pads: Vec<PadInfo>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TabletInfo {
    pub name: String,
    pub vendor_id: u32,
    pub product_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PadInfo {
    pub buttons: u32,
    pub rings: usize,
    pub strips: usize,
    pub dials: usize,
    /// Modes of the first button group (switched by e.g. the ring button).
    pub modes: u32,
}

/// The physical tool in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PenKind {
    #[default]
    Pen,
    Eraser,
    Brush,
    Pencil,
    Airbrush,
    Finger,
    Mouse,
    Lens,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PenInfo {
    pub kind: PenKind,
    /// Hardware serial number (0 if the pen doesn't report one).
    pub serial: u64,
    pub has_tilt: bool,
    pub has_rotation: bool,
    pub has_distance: bool,
}

/// Pen state at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PenSample {
    pub x: f64,
    pub y: f64,
    /// 0.0-1.0
    pub pressure: f64,
    /// Degrees from vertical along x and y.
    pub tilt: (f64, f64),
    /// Barrel rotation in degrees.
    pub rotation: f64,
    /// Hover height, 0.0 (touching) to 1.0 (furthest detectable).
    pub distance: f64,
    /// Compositor timestamp in milliseconds.
    pub time: u32,
}

/// Stylus barrel buttons, as numbered by the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StylusButton {
    /// `BTN_STYLUS`
    First,
    /// `BTN_STYLUS2`
    Second,
    /// `BTN_STYLUS3`, only on three-button pens.
    Third,
}

impl StylusButton {
    pub fn index(self) -> usize {
        self as usize
    }
}

/// One-euro filter (Casiez et al. 2012) for 2D positions: smooths jitter
/// strongly when the pen moves slowly and barely at all when it moves fast,
/// so careful writing steadies without adding lag to quick strokes.
pub struct OneEuroFilter {
    /// Cutoff frequency (Hz) when stationary; lower removes more jitter.
    pub min_cutoff: f64,
    /// How quickly the cutoff rises with speed (per px/s); higher means less lag.
    pub beta: f64,
    /// Cutoff frequency (Hz) for the speed estimate.
    pub derivative_cutoff: f64,
    /// Last (position, velocity, time in ms).
    state: Option<((f64, f64), (f64, f64), u32)>,
}

impl OneEuroFilter {
    pub const fn new(min_cutoff: f64, beta: f64, derivative_cutoff: f64) -> Self {
        Self {
            min_cutoff,
            beta,
            derivative_cutoff,
            state: None,
        }
    }

    /// Forget history, e.g. when the pen leaves and re-enters.
    pub fn reset(&mut self) {
        self.state = None;
    }

    /// Filter a position sampled at `time` (ms).
    pub fn filter(&mut self, (x, y): (f64, f64), time: u32) -> (f64, f64) {
        let Some(((px, py), (pvx, pvy), last)) = self.state else {
            self.state = Some(((x, y), (0.0, 0.0), time));
            return (x, y);
        };
        // Several events can share a millisecond timestamp.
        let dt = (time.wrapping_sub(last).max(1) as f64 / 1000.0).min(0.1);
        let alpha = |cutoff: f64| {
            let tau = 1.0 / (2.0 * std::f64::consts::PI * cutoff);
            1.0 / (1.0 + tau / dt)
        };

        let a = alpha(self.derivative_cutoff);
        let vx = pvx + a * ((x - px) / dt - pvx);
        let vy = pvy + a * ((y - py) / dt - pvy);

        let a = alpha(self.min_cutoff + self.beta * vx.hypot(vy));
        let filtered = (px + a * (x - px), py + a * (y - py));
        self.state = Some((filtered, (vx, vy), time));
        filtered
    }
}

#[cfg(target_os = "linux")]
pub use wayland::spawn;

#[cfg(target_os = "linux")]
mod wayland {
    use super::{
        Devices, PadInfo, PenInfo, PenKind, PenSample, PinchPhase, StylusButton, TabletEvent,
        TabletInfo, UserEvent,
    };
    use std::collections::HashMap;
    use wayland_backend::client::{Backend, ObjectId};
    use wayland_client::protocol::{wl_pointer, wl_registry, wl_seat};
    use wayland_client::{
        Connection, Dispatch, EventQueue, Proxy, QueueHandle, WEnum, event_created_child,
    };
    use wayland_protocols::wp::pointer_gestures::zv1::client::{
        zwp_pointer_gesture_pinch_v1, zwp_pointer_gestures_v1,
    };
    use wayland_protocols::wp::tablet::zv2::client::{
        zwp_tablet_manager_v2, zwp_tablet_pad_dial_v2, zwp_tablet_pad_group_v2,
        zwp_tablet_pad_ring_v2, zwp_tablet_pad_strip_v2, zwp_tablet_pad_v2, zwp_tablet_seat_v2,
        zwp_tablet_tool_v2, zwp_tablet_v2,
    };
    use winit::event_loop::EventLoopProxy;

    /// linux/input-event-codes.h
    const BTN_STYLUS: u32 = 0x14b;
    const BTN_STYLUS2: u32 = 0x14c;
    const BTN_STYLUS3: u32 = 0x149;

    /// A pen tool: its description and events accumulated until `frame`.
    #[derive(Default)]
    struct Tool {
        info: PenInfo,
        sample: PenSample,
        moved: bool,
        proximity_in: bool,
        down: Option<bool>,
        buttons: Vec<(StylusButton, bool)>,
        proximity_out: bool,
    }

    /// A touch ring or strip: its index and pending value until `frame`.
    struct Control {
        index: usize,
        group: ObjectId,
        value: Option<f64>,
        stopped: bool,
    }

    struct State {
        proxy: EventLoopProxy<UserEvent>,
        seat: Option<wl_seat::WlSeat>,
        manager: Option<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
        gestures: Option<zwp_pointer_gestures_v1::ZwpPointerGesturesV1>,
        pinch: Option<zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1>,
        tablets: HashMap<ObjectId, TabletInfo>,
        pads: HashMap<ObjectId, PadInfo>,
        /// Pad each button group belongs to, and the group's current mode.
        groups: HashMap<ObjectId, (ObjectId, u32)>,
        rings: HashMap<ObjectId, Control>,
        strips: HashMap<ObjectId, Control>,
        dials: HashMap<ObjectId, Control>,
        tools: HashMap<ObjectId, Tool>,
    }

    impl State {
        fn send(&self, event: TabletEvent) {
            let _ = self.proxy.send_event(UserEvent::Tablet(event));
        }

        fn send_devices(&self) {
            self.send(TabletEvent::Devices(Devices {
                tablets: self.tablets.values().cloned().collect(),
                pads: self.pads.values().cloned().collect(),
            }));
        }

        fn group_mode(&self, group: &ObjectId) -> u32 {
            self.groups.get(group).map_or(0, |&(_, mode)| mode)
        }
    }

    /// Start listening for tablet events on winit's Wayland connection.
    ///
    /// # Safety
    /// `display` must be a valid `wl_display` pointer that outlives the process'
    /// use of it (winit keeps its connection open until exit).
    pub unsafe fn spawn(display: *mut std::ffi::c_void, proxy: EventLoopProxy<UserEvent>) {
        let backend = unsafe { Backend::from_foreign_display(display.cast()) };
        let conn = Connection::from_backend(backend);
        let queue: EventQueue<State> = conn.new_event_queue();
        let qh = queue.handle();
        conn.display().get_registry(&qh, ());

        let mut state = State {
            proxy,
            seat: None,
            manager: None,
            gestures: None,
            pinch: None,
            tablets: HashMap::new(),
            pads: HashMap::new(),
            groups: HashMap::new(),
            rings: HashMap::new(),
            strips: HashMap::new(),
            dials: HashMap::new(),
            tools: HashMap::new(),
        };

        let spawned = std::thread::Builder::new()
            .name("wayland-tablet".into())
            .spawn(move || {
                let mut queue = queue;
                if let Err(e) = queue.roundtrip(&mut state) {
                    log::warn!("Tablet: registry roundtrip failed: {e}");
                    std::mem::forget(queue);
                    std::mem::forget(state);
                    return;
                }
                match (&state.manager, &state.seat) {
                    (Some(manager), Some(seat)) => {
                        manager.get_tablet_seat(seat, &queue.handle(), ());
                        log::info!("Tablet: listening for stylus input");
                    }
                    _ => log::info!("Tablet: compositor has no zwp_tablet_manager_v2"),
                }
                while queue.blocking_dispatch(&mut state).is_ok() {}
                // The connection is gone, usually because winit closed it on
                // exit. Destroying our proxies would touch the freed
                // wl_display, so leak them; the process is ending anyway.
                std::mem::forget(queue);
                std::mem::forget(state);
            });
        if let Err(e) = spawned {
            log::warn!("Tablet: failed to spawn thread: {e}");
        }
    }

    impl Dispatch<wl_registry::WlRegistry, ()> for State {
        fn event(
            state: &mut Self,
            registry: &wl_registry::WlRegistry,
            event: wl_registry::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            let wl_registry::Event::Global {
                name,
                interface,
                version,
            } = event
            else {
                return;
            };
            if interface == wl_seat::WlSeat::interface().name && state.seat.is_none() {
                state.seat = Some(registry.bind(name, 1, qh, ()));
            } else if interface == zwp_tablet_manager_v2::ZwpTabletManagerV2::interface().name {
                // Version 2 adds pad dials.
                let version = version.min(2);
                log::info!("Tablet: tablet protocol version {version}");
                state.manager = Some(registry.bind(name, version, qh, ()));
            } else if interface == zwp_pointer_gestures_v1::ZwpPointerGesturesV1::interface().name {
                state.gestures = Some(registry.bind(name, 1, qh, ()));
            }
        }
    }

    impl Dispatch<wl_seat::WlSeat, ()> for State {
        fn event(
            state: &mut Self,
            seat: &wl_seat::WlSeat,
            event: wl_seat::Event,
            _: &(),
            _: &Connection,
            qh: &QueueHandle<Self>,
        ) {
            // A pointer of our own to receive pinch gestures on.
            let wl_seat::Event::Capabilities {
                capabilities: WEnum::Value(caps),
            } = event
            else {
                return;
            };
            if caps.contains(wl_seat::Capability::Pointer) && state.pinch.is_none() {
                if let Some(gestures) = &state.gestures {
                    let pointer = seat.get_pointer(qh, ());
                    state.pinch = Some(gestures.get_pinch_gesture(&pointer, qh, ()));
                }
            }
        }
    }

    impl Dispatch<zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1, ()> for State {
        fn event(
            state: &mut Self,
            _: &zwp_pointer_gesture_pinch_v1::ZwpPointerGesturePinchV1,
            event: zwp_pointer_gesture_pinch_v1::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_pointer_gesture_pinch_v1::Event;
            let phase = match event {
                Event::Begin { .. } => PinchPhase::Begin,
                Event::Update { scale, .. } => PinchPhase::Update { scale },
                Event::End { .. } => PinchPhase::End,
                _ => return,
            };
            state.send(TabletEvent::Pinch(phase));
        }
    }

    impl Dispatch<zwp_tablet_seat_v2::ZwpTabletSeatV2, ()> for State {
        fn event(
            _: &mut Self,
            _: &zwp_tablet_seat_v2::ZwpTabletSeatV2,
            _: zwp_tablet_seat_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }

        event_created_child!(State, zwp_tablet_seat_v2::ZwpTabletSeatV2, [
            zwp_tablet_seat_v2::EVT_TABLET_ADDED_OPCODE => (zwp_tablet_v2::ZwpTabletV2, ()),
            zwp_tablet_seat_v2::EVT_TOOL_ADDED_OPCODE => (zwp_tablet_tool_v2::ZwpTabletToolV2, ()),
            zwp_tablet_seat_v2::EVT_PAD_ADDED_OPCODE => (zwp_tablet_pad_v2::ZwpTabletPadV2, ()),
        ]);
    }

    impl Dispatch<zwp_tablet_v2::ZwpTabletV2, ()> for State {
        fn event(
            state: &mut Self,
            tablet: &zwp_tablet_v2::ZwpTabletV2,
            event: zwp_tablet_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_v2::Event;
            let info = state.tablets.entry(tablet.id()).or_default();
            match event {
                Event::Name { name } => info.name = name,
                Event::Id { vid, pid } => {
                    info.vendor_id = vid;
                    info.product_id = pid;
                }
                Event::Done => {
                    log::info!(
                        "Tablet: {} ({:04x}:{:04x})",
                        info.name,
                        info.vendor_id,
                        info.product_id
                    );
                    state.send_devices();
                }
                Event::Removed => {
                    state.tablets.remove(&tablet.id());
                    tablet.destroy();
                    state.send_devices();
                }
                _ => {}
            }
        }
    }

    impl Dispatch<zwp_tablet_tool_v2::ZwpTabletToolV2, ()> for State {
        fn event(
            state: &mut Self,
            tool_proxy: &zwp_tablet_tool_v2::ZwpTabletToolV2,
            event: zwp_tablet_tool_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_tool_v2::{Capability, Event, Type};

            if let Event::Removed = event {
                state.tools.remove(&tool_proxy.id());
                tool_proxy.destroy();
                return;
            }

            let tool = state.tools.entry(tool_proxy.id()).or_default();
            match event {
                Event::Type {
                    tool_type: WEnum::Value(kind),
                } => {
                    tool.info.kind = match kind {
                        Type::Eraser => PenKind::Eraser,
                        Type::Brush => PenKind::Brush,
                        Type::Pencil => PenKind::Pencil,
                        Type::Airbrush => PenKind::Airbrush,
                        Type::Finger => PenKind::Finger,
                        Type::Mouse => PenKind::Mouse,
                        Type::Lens => PenKind::Lens,
                        _ => PenKind::Pen,
                    };
                }
                Event::HardwareSerial {
                    hardware_serial_hi,
                    hardware_serial_lo,
                } => {
                    tool.info.serial =
                        (u64::from(hardware_serial_hi) << 32) | u64::from(hardware_serial_lo);
                }
                Event::Capability {
                    capability: WEnum::Value(capability),
                } => match capability {
                    Capability::Tilt => tool.info.has_tilt = true,
                    Capability::Rotation => tool.info.has_rotation = true,
                    Capability::Distance => tool.info.has_distance = true,
                    _ => {}
                },
                Event::ProximityIn { .. } => tool.proximity_in = true,
                Event::Motion { x, y } => {
                    tool.sample.x = x;
                    tool.sample.y = y;
                    tool.moved = true;
                }
                Event::Pressure { pressure } => {
                    tool.sample.pressure = f64::from(pressure) / 65535.0;
                    tool.moved = true;
                }
                Event::Distance { distance } => {
                    tool.sample.distance = f64::from(distance) / 65535.0;
                    tool.moved = true;
                }
                Event::Tilt { tilt_x, tilt_y } => {
                    tool.sample.tilt = (tilt_x, tilt_y);
                    tool.moved = true;
                }
                Event::Rotation { degrees } => {
                    tool.sample.rotation = degrees;
                    tool.moved = true;
                }
                Event::Down { .. } => tool.down = Some(true),
                Event::Up => tool.down = Some(false),
                Event::Button {
                    button,
                    state: button_state,
                    ..
                } => {
                    let pressed = matches!(
                        button_state,
                        WEnum::Value(zwp_tablet_tool_v2::ButtonState::Pressed)
                    );
                    let button = match button {
                        BTN_STYLUS => StylusButton::First,
                        BTN_STYLUS2 => StylusButton::Second,
                        BTN_STYLUS3 => StylusButton::Third,
                        _ => {
                            log::debug!("Tablet: unhandled stylus button {button:#x}");
                            return;
                        }
                    };
                    tool.buttons.push((button, pressed));
                }
                Event::ProximityOut => tool.proximity_out = true,
                Event::Frame { time } => {
                    tool.sample.time = time;
                    // Axis values persist between frames; they're only re-sent
                    // when they change.
                    let info = tool.info;
                    let sample = tool.sample;
                    let moved = std::mem::take(&mut tool.moved);
                    let proximity_in = std::mem::take(&mut tool.proximity_in);
                    let down = tool.down.take();
                    let buttons = std::mem::take(&mut tool.buttons);
                    let proximity_out = std::mem::take(&mut tool.proximity_out);

                    if proximity_in {
                        state.send(TabletEvent::ProximityIn(info));
                    }
                    if moved || proximity_in {
                        state.send(TabletEvent::Motion(sample));
                    }
                    match down {
                        Some(true) => state.send(TabletEvent::Down),
                        Some(false) => state.send(TabletEvent::Up),
                        None => {}
                    }
                    for (button, pressed) in buttons {
                        state.send(TabletEvent::Button { button, pressed });
                    }
                    if proximity_out {
                        state.send(TabletEvent::ProximityOut);
                    }
                }
                _ => {}
            }
        }
    }

    impl Dispatch<zwp_tablet_pad_v2::ZwpTabletPadV2, ()> for State {
        fn event(
            state: &mut Self,
            pad: &zwp_tablet_pad_v2::ZwpTabletPadV2,
            event: zwp_tablet_pad_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_pad_v2::Event;
            match event {
                Event::Group { pad_group } => {
                    state.groups.insert(pad_group.id(), (pad.id(), 0));
                }
                Event::Buttons { buttons } => {
                    state.pads.entry(pad.id()).or_default().buttons = buttons;
                }
                Event::Done => {
                    if let Some(info) = state.pads.get(&pad.id()) {
                        log::info!(
                            "Tablet pad: {} buttons, {} rings, {} strips, {} dials, {} modes",
                            info.buttons,
                            info.rings,
                            info.strips,
                            info.dials,
                            info.modes
                        );
                    }
                    state.send_devices();
                }
                Event::Button {
                    button,
                    state: button_state,
                    ..
                } => state.send(TabletEvent::PadButton {
                    index: button,
                    pressed: matches!(
                        button_state,
                        WEnum::Value(zwp_tablet_pad_v2::ButtonState::Pressed)
                    ),
                }),
                Event::Removed => {
                    let id = pad.id();
                    state.pads.remove(&id);
                    state.groups.retain(|_, (owner, _)| *owner != id);
                    pad.destroy();
                    state.send_devices();
                }
                _ => {}
            }
        }

        event_created_child!(State, zwp_tablet_pad_v2::ZwpTabletPadV2, [
            zwp_tablet_pad_v2::EVT_GROUP_OPCODE => (zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, ()),
        ]);
    }

    impl Dispatch<zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, ()> for State {
        fn event(
            state: &mut Self,
            group: &zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2,
            event: zwp_tablet_pad_group_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_pad_group_v2::Event;
            let Some(pad) = state.groups.get(&group.id()).map(|(pad, _)| pad.clone()) else {
                return;
            };
            let control = |index| Control {
                index,
                group: group.id(),
                value: None,
                stopped: false,
            };
            match event {
                Event::Ring { ring } => {
                    let info = state.pads.entry(pad).or_default();
                    state.rings.insert(ring.id(), control(info.rings));
                    info.rings += 1;
                }
                Event::Strip { strip } => {
                    let info = state.pads.entry(pad).or_default();
                    state.strips.insert(strip.id(), control(info.strips));
                    info.strips += 1;
                }
                Event::Dial { dial } => {
                    let info = state.pads.entry(pad).or_default();
                    state.dials.insert(dial.id(), control(info.dials));
                    info.dials += 1;
                }
                Event::Modes { modes } => {
                    let info = state.pads.entry(pad).or_default();
                    info.modes = info.modes.max(modes);
                }
                Event::ModeSwitch { mode, .. } => {
                    if let Some(entry) = state.groups.get_mut(&group.id()) {
                        entry.1 = mode;
                    }
                }
                _ => {}
            }
        }

        event_created_child!(State, zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, [
            zwp_tablet_pad_group_v2::EVT_RING_OPCODE => (zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2, ()),
            zwp_tablet_pad_group_v2::EVT_STRIP_OPCODE => (zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2, ()),
            zwp_tablet_pad_group_v2::EVT_DIAL_OPCODE => (zwp_tablet_pad_dial_v2::ZwpTabletPadDialV2, ()),
        ]);
    }

    impl Dispatch<zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2, ()> for State {
        fn event(
            state: &mut Self,
            ring: &zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2,
            event: zwp_tablet_pad_ring_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_pad_ring_v2::Event;
            let Some(control) = state.rings.get_mut(&ring.id()) else {
                return;
            };
            match event {
                Event::Angle { degrees } => control.value = Some(degrees),
                Event::Stop => control.stopped = true,
                Event::Frame { .. } => {
                    let index = control.index;
                    let group = control.group.clone();
                    let angle = if std::mem::take(&mut control.stopped) {
                        control.value = None;
                        None
                    } else {
                        control.value.take()
                    };
                    let mode = state.group_mode(&group);
                    state.send(TabletEvent::Ring { index, mode, angle });
                }
                _ => {}
            }
        }
    }

    impl Dispatch<zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2, ()> for State {
        fn event(
            state: &mut Self,
            strip: &zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2,
            event: zwp_tablet_pad_strip_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_pad_strip_v2::Event;
            let Some(control) = state.strips.get_mut(&strip.id()) else {
                return;
            };
            match event {
                Event::Position { position } => {
                    control.value = Some(f64::from(position) / 65535.0);
                }
                Event::Stop => control.stopped = true,
                Event::Frame { .. } => {
                    let index = control.index;
                    let group = control.group.clone();
                    let position = if std::mem::take(&mut control.stopped) {
                        control.value = None;
                        None
                    } else {
                        control.value.take()
                    };
                    let mode = state.group_mode(&group);
                    state.send(TabletEvent::Strip {
                        index,
                        mode,
                        position,
                    });
                }
                _ => {}
            }
        }
    }

    impl Dispatch<zwp_tablet_pad_dial_v2::ZwpTabletPadDialV2, ()> for State {
        fn event(
            state: &mut Self,
            dial: &zwp_tablet_pad_dial_v2::ZwpTabletPadDialV2,
            event: zwp_tablet_pad_dial_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_pad_dial_v2::Event;
            let Some(control) = state.dials.get_mut(&dial.id()) else {
                return;
            };
            match event {
                // 120 per detent, like high-resolution scroll wheels.
                Event::Delta { value120 } => {
                    *control.value.get_or_insert(0.0) += f64::from(value120) / 120.0;
                }
                Event::Frame { .. } => {
                    let Some(detents) = control.value.take() else {
                        return;
                    };
                    let index = control.index;
                    let group = control.group.clone();
                    let mode = state.group_mode(&group);
                    state.send(TabletEvent::Dial {
                        index,
                        mode,
                        detents,
                    });
                }
                _ => {}
            }
        }
    }

    /// Objects whose events we don't care about.
    macro_rules! ignore_events {
        ($($iface:ty),* $(,)?) => {$(
            impl Dispatch<$iface, ()> for State {
                fn event(
                    _: &mut Self,
                    _: &$iface,
                    _: <$iface as Proxy>::Event,
                    _: &(),
                    _: &Connection,
                    _: &QueueHandle<Self>,
                ) {
                }
            }
        )*};
    }

    ignore_events!(
        wl_pointer::WlPointer,
        zwp_pointer_gestures_v1::ZwpPointerGesturesV1,
        zwp_tablet_manager_v2::ZwpTabletManagerV2,
    );
}

#[cfg(test)]
mod tests {
    use super::OneEuroFilter;

    /// Pen at 200 Hz: jitter of ±0.5 px while holding still is mostly removed.
    #[test]
    fn test_one_euro_steadies_slow_pen() {
        let mut filter = OneEuroFilter::new(3.0, 0.03, 1.0);
        let mut spread: f64 = 0.0;
        for i in 0..200 {
            let jitter = if i % 2 == 0 { 0.5 } else { -0.5 };
            let (x, _) = filter.filter((100.0 + jitter, 100.0), i * 5);
            if i > 50 {
                spread = spread.max((x - 100.0).abs());
            }
        }
        assert!(spread < 0.1, "residual jitter {spread}");
    }

    /// A fast stroke (1000 px/s) trails the pen by only a few pixels.
    #[test]
    fn test_one_euro_low_lag_when_fast() {
        let mut filter = OneEuroFilter::new(3.0, 0.03, 1.0);
        let mut lag = 0.0;
        for i in 0..100u32 {
            let x = i as f64 * 5.0;
            lag = x - filter.filter((x, 0.0), i * 5).0;
        }
        assert!(lag < 6.0, "lag {lag} px");
    }
}
