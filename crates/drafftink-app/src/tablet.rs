//! Graphics-tablet (stylus) input.
//!
//! winit doesn't bind the Wayland tablet protocol, and compositors such as
//! Hyprland don't emulate pointer buttons for a pen tip, so without this a pen
//! can move the cursor but never draw. On Wayland we share winit's
//! `wl_display`, listen for `zwp_tablet_tool_v2` events on a background thread
//! and forward them to the event loop as user events.

/// Event loop user event.
#[derive(Debug, Clone, Copy)]
pub enum UserEvent {
    Tablet(TabletEvent),
}

/// A stylus event. Positions are surface-local logical coordinates.
#[derive(Debug, Clone, Copy)]
pub enum TabletEvent {
    /// Tool entered the window; `eraser` is true for the pen's eraser end.
    ProximityIn { eraser: bool },
    /// Tool moved; `pressure` is normalized to 0.0-1.0.
    Motion { x: f64, y: f64, pressure: f64 },
    /// Tip touched the surface.
    Down,
    /// Tip lifted off the surface.
    Up,
    /// Barrel button: `secondary` is the second (lower) stylus button.
    Button { secondary: bool, pressed: bool },
    /// Tool left the window.
    ProximityOut,
}

#[cfg(target_os = "linux")]
pub use wayland::spawn;

#[cfg(target_os = "linux")]
mod wayland {
    use super::{TabletEvent, UserEvent};
    use std::collections::HashMap;
    use wayland_backend::client::{Backend, ObjectId};
    use wayland_client::protocol::{wl_registry, wl_seat};
    use wayland_client::{
        Connection, Dispatch, EventQueue, Proxy, QueueHandle, event_created_child,
    };
    use wayland_protocols::wp::tablet::zv2::client::{
        zwp_tablet_manager_v2, zwp_tablet_pad_group_v2, zwp_tablet_pad_ring_v2,
        zwp_tablet_pad_strip_v2, zwp_tablet_pad_v2, zwp_tablet_seat_v2, zwp_tablet_tool_v2,
        zwp_tablet_v2,
    };
    use winit::event_loop::EventLoopProxy;

    /// linux/input-event-codes.h
    const BTN_STYLUS: u32 = 0x14b;
    const BTN_STYLUS2: u32 = 0x14c;

    /// Events of one tool accumulated until its `frame` event.
    #[derive(Default)]
    struct ToolFrame {
        position: Option<(f64, f64)>,
        pressure: f64,
        eraser: bool,
        proximity_in: bool,
        down: Option<bool>,
        buttons: Vec<(bool, bool)>,
        proximity_out: bool,
    }

    struct State {
        proxy: EventLoopProxy<UserEvent>,
        seat: Option<wl_seat::WlSeat>,
        manager: Option<zwp_tablet_manager_v2::ZwpTabletManagerV2>,
        tools: HashMap<ObjectId, ToolFrame>,
    }

    impl State {
        fn send(&self, event: TabletEvent) {
            let _ = self.proxy.send_event(UserEvent::Tablet(event));
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
            tools: HashMap::new(),
        };

        let spawned = std::thread::Builder::new()
            .name("wayland-tablet".into())
            .spawn(move || {
                let mut queue = queue;
                if let Err(e) = queue.roundtrip(&mut state) {
                    log::warn!("Tablet: registry roundtrip failed: {e}");
                    return;
                }
                match (&state.manager, &state.seat) {
                    (Some(manager), Some(seat)) => {
                        manager.get_tablet_seat(seat, &queue.handle(), ());
                        log::info!("Tablet: listening for stylus input");
                    }
                    _ => {
                        log::info!("Tablet: compositor has no zwp_tablet_manager_v2");
                        return;
                    }
                }
                while queue.blocking_dispatch(&mut state).is_ok() {}
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
            if let wl_registry::Event::Global {
                name, interface, ..
            } = event
            {
                if interface == wl_seat::WlSeat::interface().name && state.seat.is_none() {
                    state.seat = Some(registry.bind(name, 1, qh, ()));
                } else if interface == zwp_tablet_manager_v2::ZwpTabletManagerV2::interface().name {
                    // Version 1: newer versions add pad dials we'd have to handle.
                    state.manager = Some(registry.bind(name, 1, qh, ()));
                }
            }
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

    impl Dispatch<zwp_tablet_tool_v2::ZwpTabletToolV2, ()> for State {
        fn event(
            state: &mut Self,
            tool: &zwp_tablet_tool_v2::ZwpTabletToolV2,
            event: zwp_tablet_tool_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
            use zwp_tablet_tool_v2::Event;

            if let Event::Removed = event {
                state.tools.remove(&tool.id());
                tool.destroy();
                return;
            }

            let frame = state.tools.entry(tool.id()).or_default();
            match event {
                Event::Type { tool_type } => {
                    frame.eraser = matches!(
                        tool_type.into_result(),
                        Ok(zwp_tablet_tool_v2::Type::Eraser)
                    );
                }
                Event::ProximityIn { .. } => frame.proximity_in = true,
                Event::Motion { x, y } => frame.position = Some((x, y)),
                Event::Pressure { pressure } => frame.pressure = pressure as f64 / 65535.0,
                Event::Down { .. } => frame.down = Some(true),
                Event::Up => frame.down = Some(false),
                Event::Button {
                    button,
                    state: button_state,
                    ..
                } => {
                    let pressed = matches!(
                        button_state.into_result(),
                        Ok(zwp_tablet_tool_v2::ButtonState::Pressed)
                    );
                    match button {
                        BTN_STYLUS => frame.buttons.push((false, pressed)),
                        BTN_STYLUS2 => frame.buttons.push((true, pressed)),
                        _ => {}
                    }
                }
                Event::ProximityOut => frame.proximity_out = true,
                Event::Frame { .. } => {
                    let frame = std::mem::take(frame);
                    // Keep per-tool state: type is sent once, pressure only on change.
                    let next = state.tools.entry(tool.id()).or_default();
                    next.pressure = frame.pressure;
                    next.eraser = frame.eraser;

                    if frame.proximity_in {
                        state.send(TabletEvent::ProximityIn {
                            eraser: frame.eraser,
                        });
                    }
                    if let Some((x, y)) = frame.position {
                        state.send(TabletEvent::Motion {
                            x,
                            y,
                            pressure: frame.pressure,
                        });
                    }
                    match frame.down {
                        Some(true) => state.send(TabletEvent::Down),
                        Some(false) => state.send(TabletEvent::Up),
                        None => {}
                    }
                    for (secondary, pressed) in frame.buttons {
                        state.send(TabletEvent::Button { secondary, pressed });
                    }
                    if frame.proximity_out {
                        state.send(TabletEvent::ProximityOut);
                    }
                }
                _ => {}
            }
        }
    }

    impl Dispatch<zwp_tablet_pad_v2::ZwpTabletPadV2, ()> for State {
        fn event(
            _: &mut Self,
            _: &zwp_tablet_pad_v2::ZwpTabletPadV2,
            _: zwp_tablet_pad_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }

        event_created_child!(State, zwp_tablet_pad_v2::ZwpTabletPadV2, [
            zwp_tablet_pad_v2::EVT_GROUP_OPCODE => (zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, ()),
        ]);
    }

    impl Dispatch<zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, ()> for State {
        fn event(
            _: &mut Self,
            _: &zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2,
            _: zwp_tablet_pad_group_v2::Event,
            _: &(),
            _: &Connection,
            _: &QueueHandle<Self>,
        ) {
        }

        event_created_child!(State, zwp_tablet_pad_group_v2::ZwpTabletPadGroupV2, [
            zwp_tablet_pad_group_v2::EVT_RING_OPCODE => (zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2, ()),
            zwp_tablet_pad_group_v2::EVT_STRIP_OPCODE => (zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2, ()),
        ]);
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
        wl_seat::WlSeat,
        zwp_tablet_manager_v2::ZwpTabletManagerV2,
        zwp_tablet_v2::ZwpTabletV2,
        zwp_tablet_pad_ring_v2::ZwpTabletPadRingV2,
        zwp_tablet_pad_strip_v2::ZwpTabletPadStripV2,
    );
}
