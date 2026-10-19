//! Tablet pad dials read directly from the kernel (Linux evdev).
//!
//! Compositors that only offer version 1 of the Wayland tablet protocol (such
//! as Hyprland 0.56) don't forward pad dials to applications. Pads report
//! their dials as scroll wheels, so we read those from the device itself.
//! This needs read access to the pad's `/dev/input/event*` node, which
//! `packaging/linux/70-drafftink-tablet.rules` grants to the logged-in user.

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use winit::event_loop::EventLoopProxy;

use crate::tablet::{TabletEvent, UserEvent};

// linux/input-event-codes.h
const EV_REL: u16 = 0x02;
const REL_HWHEEL: u16 = 0x06;
const REL_WHEEL: u16 = 0x08;
const REL_WHEEL_HI_RES: u16 = 0x0b;
const REL_HWHEEL_HI_RES: u16 = 0x0c;

/// Size of `struct input_event` (a `timeval` then type, code and value).
const EVENT_SIZE: usize = if cfg!(target_pointer_width = "64") {
    24
} else {
    16
};

/// How often to look for newly plugged-in pads.
const RESCAN_INTERVAL: Duration = Duration::from_secs(3);

/// A tablet pad with dials.
struct Pad {
    node: PathBuf,
    /// Dial axes in index order: (low-res code, high-res code if supported).
    dials: Vec<(u16, Option<u16>)>,
}

/// Start reading pad dials in the background. Reports what it finds with
/// [`TabletEvent::EvdevStatus`] and dial turns with [`TabletEvent::EvdevDial`].
pub fn spawn(proxy: EventLoopProxy<UserEvent>) {
    let spawned = std::thread::Builder::new()
        .name("tablet-evdev".into())
        .spawn(move || scan_loop(proxy));
    if let Err(e) = spawned {
        log::warn!("Tablet: failed to spawn dial reader: {e}");
    }
}

fn scan_loop(proxy: EventLoopProxy<UserEvent>) {
    let open: std::sync::Arc<std::sync::Mutex<HashSet<PathBuf>>> = Default::default();
    let mut last_status = None;
    loop {
        let pads = find_pads();
        let mut dials = 0;
        let mut denied = false;
        for pad in pads {
            let first_index = dials;
            dials += pad.dials.len();
            if open.lock().unwrap().contains(&pad.node) {
                continue;
            }
            match File::open(&pad.node) {
                Ok(file) => {
                    log::info!(
                        "Tablet: reading {} dial(s) from {}",
                        pad.dials.len(),
                        pad.node.display()
                    );
                    open.lock().unwrap().insert(pad.node.clone());
                    let proxy = proxy.clone();
                    let open = open.clone();
                    std::thread::spawn(move || {
                        read_dials(file, &pad.dials, first_index, &proxy);
                        // Unplugged or failed: allow reopening on a later scan.
                        open.lock().unwrap().remove(&pad.node);
                    });
                }
                Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => denied = true,
                Err(e) => log::warn!("Tablet: can't open {}: {e}", pad.node.display()),
            }
        }
        let status = (dials, denied);
        if last_status != Some(status) {
            if denied {
                log::info!("Tablet: no permission to read tablet pad dials");
            }
            last_status = Some(status);
            let event = TabletEvent::EvdevStatus {
                dials,
                permission_denied: denied,
            };
            if proxy.send_event(UserEvent::Tablet(event)).is_err() {
                return;
            }
        }
        std::thread::sleep(RESCAN_INTERVAL);
    }
}

/// Read dial events until the device goes away.
fn read_dials(
    mut file: File,
    dials: &[(u16, Option<u16>)],
    first_index: usize,
    proxy: &EventLoopProxy<UserEvent>,
) {
    let mut buf = [0u8; EVENT_SIZE];
    loop {
        if file.read_exact(&mut buf).is_err() {
            return;
        }
        let kind = u16::from_ne_bytes([buf[EVENT_SIZE - 8], buf[EVENT_SIZE - 7]]);
        let code = u16::from_ne_bytes([buf[EVENT_SIZE - 6], buf[EVENT_SIZE - 5]]);
        let value = i32::from_ne_bytes(buf[EVENT_SIZE - 4..].try_into().unwrap());
        if kind != EV_REL {
            continue;
        }
        // Prefer high-resolution events (120 per detent) when the dial has them.
        let detents = dials
            .iter()
            .enumerate()
            .find_map(|(i, &(low, high))| match high {
                Some(high) if code == high => Some((i, f64::from(value) / 120.0)),
                None if code == low => Some((i, f64::from(value))),
                _ => None,
            });
        if let Some((i, detents)) = detents {
            let event = TabletEvent::EvdevDial {
                index: first_index + i,
                detents,
            };
            if proxy.send_event(UserEvent::Tablet(event)).is_err() {
                return;
            }
        }
    }
}

/// Input devices that udev marks as tablet pads and that have dials.
fn find_pads() -> Vec<Pad> {
    let Ok(entries) = std::fs::read_dir("/sys/class/input") else {
        return Vec::new();
    };
    let mut pads: Vec<Pad> = entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("event"))
        .filter_map(|e| {
            let sys = e.path();
            if !is_tablet_pad(&sys) {
                return None;
            }
            let rel = read_bitmask(&sys.join("device/capabilities/rel"));
            let has = |code: u16| rel.contains(&code);
            let dials: Vec<_> = [
                (REL_WHEEL, REL_WHEEL_HI_RES),
                (REL_HWHEEL, REL_HWHEEL_HI_RES),
            ]
            .into_iter()
            .filter(|&(low, _)| has(low))
            .map(|(low, high)| (low, has(high).then_some(high)))
            .collect();
            (!dials.is_empty()).then(|| Pad {
                node: Path::new("/dev/input").join(e.file_name()),
                dials,
            })
        })
        .collect();
    pads.sort_by(|a, b| a.node.cmp(&b.node));
    pads
}

/// Whether udev classified the device as a tablet pad (from its database
/// entry, `/run/udev/data/c<major>:<minor>`).
fn is_tablet_pad(sys: &Path) -> bool {
    let Ok(dev) = std::fs::read_to_string(sys.join("dev")) else {
        return false;
    };
    std::fs::read_to_string(format!("/run/udev/data/c{}", dev.trim()))
        .is_ok_and(|data| data.lines().any(|l| l == "E:ID_INPUT_TABLET_PAD=1"))
}

/// Parse a sysfs capability bitmask (space-separated hex words, most
/// significant first) into the set bit numbers.
fn read_bitmask(path: &Path) -> HashSet<u16> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    parse_bitmask(&text)
}

fn parse_bitmask(text: &str) -> HashSet<u16> {
    let bits_per_word = usize::BITS as usize;
    text.split_whitespace()
        .rev()
        .enumerate()
        .flat_map(|(word_index, word)| {
            let word = u64::from_str_radix(word, 16).unwrap_or(0);
            (0..bits_per_word)
                .filter(move |bit| word >> bit & 1 == 1)
                .map(move |bit| (word_index * bits_per_word + bit) as u16)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bitmask() {
        // The Intuos Pro L pad: REL_HWHEEL, REL_WHEEL and both high-res axes.
        let bits = parse_bitmask("1940\n");
        assert_eq!(
            bits,
            HashSet::from([REL_HWHEEL, REL_WHEEL, REL_WHEEL_HI_RES, REL_HWHEEL_HI_RES])
        );
        let bits = parse_bitmask("1 0");
        assert!(bits.contains(&(usize::BITS as u16)));
    }
}
