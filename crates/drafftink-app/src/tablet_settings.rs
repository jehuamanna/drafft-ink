//! User-configurable pen tablet behaviour.
//!
//! Button, ExpressKey, dial, ring, strip and pinch mappings apply to any
//! device. The pen-feel settings (pressure curve, stabilizer, tilt, eraser,
//! hover) apply while a Wacom tablet is connected, or for any tablet when
//! forced; otherwise the pen feels plain ([`TabletSettings::plain`]): raw
//! pressure and position. See [`TabletSettings::effective`].

use drafftink_core::tools::ToolKind;

use crate::keybindings::{COMMAND_GROUPS, Command, tool_label};
use serde::{Deserialize, Serialize};

/// USB vendor id of Wacom.
pub const WACOM_VENDOR_ID: u32 = 0x056a;

/// When the tablet settings are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Activation {
    /// Only while a Wacom tablet is connected.
    #[default]
    WacomOnly,
    /// For any connected tablet.
    AnyTablet,
    /// Never; tablets behave as a plain pen.
    Off,
}

/// What a pen or tablet button does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ButtonAction {
    #[default]
    None,
    /// Pan the canvas while held (like the middle mouse button).
    Pan,
    /// Right mouse button.
    RightClick,
    /// Use a tool while held, then return to the previous one.
    HoldTool(ToolKind),
    /// Show the tools and colours at the pointer while held.
    HoldQuickToolbars,
    /// Switch to a tool.
    SwitchTool(ToolKind),
    /// Switch to a tool, or back to the previous one if already active.
    ToggleTool(ToolKind),
    /// Run any command available to keyboard shortcuts.
    Command(Command),
}

impl ButtonAction {
    /// Every choice offered in the settings, grouped for display.
    pub fn choice_groups() -> Vec<(&'static str, Vec<Self>)> {
        let tools = all_tools();
        let mut groups = vec![
            ("Mouse", vec![Self::None, Self::Pan, Self::RightClick]),
            ("While held", vec![Self::HoldQuickToolbars]),
            (
                "Tool while held",
                tools.iter().map(|&t| Self::HoldTool(t)).collect(),
            ),
            (
                "Switch tool",
                tools.iter().map(|&t| Self::SwitchTool(t)).collect(),
            ),
            (
                "Toggle tool",
                tools.iter().map(|&t| Self::ToggleTool(t)).collect(),
            ),
        ];
        for &(group, commands) in COMMAND_GROUPS {
            let commands: Vec<Self> = commands
                .iter()
                .filter(|c| !matches!(c, Command::Tool(_)))
                .map(|&c| Self::Command(c))
                .collect();
            if !commands.is_empty() {
                groups.push((group, commands));
            }
        }
        groups
    }

    pub fn label(self) -> String {
        match self {
            Self::None => "Nothing".into(),
            Self::Pan => "Pan (hold)".into(),
            Self::RightClick => "Right click".into(),
            Self::HoldTool(tool) => format!("{} (hold)", tool_label(tool)),
            Self::HoldQuickToolbars => format!("{} (hold)", Command::QuickToolbars.label()),
            Self::SwitchTool(tool) => format!("{} (switch)", tool_label(tool)),
            Self::ToggleTool(tool) => format!("{} (toggle)", tool_label(tool)),
            Self::Command(command) => command.label(),
        }
    }
}

/// Every tool, in toolbar order.
fn all_tools() -> Vec<ToolKind> {
    COMMAND_GROUPS
        .iter()
        .flat_map(|(_, commands)| commands.iter())
        .filter_map(|c| match c {
            Command::Tool(tool) => Some(*tool),
            _ => None,
        })
        .collect()
}

/// What turning a dial or touch ring (or sliding a touch strip) does, once
/// per step. Positive steps are clockwise (or down a strip).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RingAction {
    #[default]
    None,
    Zoom,
    StrokeWidth,
    StrokeColor,
    ScrollVertical,
    ScrollHorizontal,
    /// Step through undo history.
    UndoRedo,
    /// Run one command per step clockwise and another anticlockwise, e.g.
    /// nudge right / left.
    Commands {
        clockwise: Command,
        anticlockwise: Command,
    },
}

impl RingAction {
    pub const ALL: [Self; 8] = [
        Self::None,
        Self::Zoom,
        Self::StrokeWidth,
        Self::StrokeColor,
        Self::ScrollVertical,
        Self::ScrollHorizontal,
        Self::UndoRedo,
        Self::Commands {
            clockwise: Command::NudgeRight,
            anticlockwise: Command::NudgeLeft,
        },
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "Nothing",
            Self::Zoom => "Zoom",
            Self::StrokeWidth => "Stroke width",
            Self::StrokeColor => "Stroke colour",
            Self::ScrollVertical => "Scroll vertically",
            Self::ScrollHorizontal => "Scroll horizontally",
            Self::UndoRedo => "Undo / redo",
            Self::Commands { .. } => "Commands (per direction)",
        }
    }

    /// Whether `other` is the same kind of action, ignoring its commands.
    pub fn same_kind(self, other: Self) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
    }
}

/// Where the calligraphy nib angle comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NibAngle {
    /// The fixed angle in the settings.
    #[default]
    Fixed,
    /// The direction the pen is tilted.
    TiltDirection,
    /// Barrel rotation (Art Pen).
    PenRotation,
}

impl NibAngle {
    pub const ALL: [Self; 3] = [Self::Fixed, Self::TiltDirection, Self::PenRotation];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fixed => "Fixed angle",
            Self::TiltDirection => "Tilt direction",
            Self::PenRotation => "Pen rotation (Art Pen)",
        }
    }
}

/// Pen tablet settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TabletSettings {
    pub activation: Activation,

    // Pressure
    /// Pressure curve exponent: below 1 is softer (less force for thick
    /// lines), above 1 is firmer.
    pub pressure_curve: f64,
    /// Raw pressure treated as zero (ignores a resting pen's weight).
    pub pressure_min: f64,
    /// Raw pressure treated as full (reach full width without pressing hard).
    pub pressure_max: f64,
    /// 0 = raw pressure, towards 1 = steadier but slower to respond.
    pub pressure_smoothing: f64,

    // Stabilizer (one-euro filter)
    pub stabilizer: bool,
    /// Smoothing cutoff (Hz) while the pen moves slowly; lower is steadier.
    pub stabilizer_min_cutoff: f64,
    /// How quickly smoothing backs off with speed; higher means less lag.
    pub stabilizer_speed: f64,

    // Tilt and rotation
    /// 0 = off; towards 1, tilting the pen widens the stroke (and an upright
    /// pen draws thinner).
    pub tilt_width: f64,
    /// Calligraphy: stroke width depends on direction relative to the nib.
    pub calligraphy: bool,
    pub nib_angle_source: NibAngle,
    /// Nib angle in degrees for [`NibAngle::Fixed`].
    pub nib_angle: f64,
    /// Width across the nib's thin edge, as a fraction of full width.
    pub nib_thinness: f64,

    // Hover
    /// Fade the brush outline as the pen is lifted away from the tablet.
    pub hover_fade: bool,

    // Eraser end
    pub eraser_end_switches_tool: bool,
    /// 0 = fixed eraser size; towards 1, light pressure erases a smaller area.
    pub eraser_pressure: f64,

    // Pens
    /// Each physical pen (by serial number) remembers the tool it last used.
    pub remember_tool_per_pen: bool,
    /// Barrel buttons, in kernel order (BTN_STYLUS, BTN_STYLUS2, BTN_STYLUS3).
    pub stylus_buttons: [ButtonAction; 3],

    // Tablet controls
    /// ExpressKeys, by button number as reported by the tablet.
    pub pad_buttons: Vec<ButtonAction>,
    /// Touch ring action for each ring mode (switched by its centre button).
    pub ring_modes: Vec<RingAction>,
    /// Degrees of ring rotation per step.
    pub ring_step_degrees: f64,
    /// Action for each dial, by dial number.
    pub dial_actions: Vec<RingAction>,
    /// Dial detents (clicks) per step.
    pub dial_detents_per_step: f64,
    /// Swap the dials' clockwise and anticlockwise actions.
    pub dial_reverse: bool,
    /// Touch strip action for each strip mode.
    pub strip_modes: Vec<RingAction>,
    /// Pinch on the tablet's touch surface (or a touchpad) to zoom.
    pub pinch_zoom: bool,
}

impl Default for TabletSettings {
    fn default() -> Self {
        Self {
            activation: Activation::default(),
            pressure_curve: 1.0,
            pressure_min: 0.0,
            pressure_max: 1.0,
            // Tuned for fast handwriting: little lag, jitter still removed
            // when slow.
            pressure_smoothing: 0.3,
            stabilizer: true,
            stabilizer_min_cutoff: 6.0,
            stabilizer_speed: 0.1,
            tilt_width: 0.0,
            calligraphy: false,
            nib_angle_source: NibAngle::default(),
            nib_angle: 45.0,
            nib_thinness: 0.2,
            hover_fade: true,
            eraser_end_switches_tool: true,
            eraser_pressure: 0.0,
            remember_tool_per_pen: false,
            stylus_buttons: [
                ButtonAction::HoldTool(ToolKind::Eraser),
                ButtonAction::Pan,
                ButtonAction::HoldTool(ToolKind::Select),
            ],
            pad_buttons: vec![
                ButtonAction::Command(Command::Undo),
                ButtonAction::Command(Command::Redo),
                ButtonAction::Pan,
                ButtonAction::HoldTool(ToolKind::Eraser),
                ButtonAction::ToggleTool(ToolKind::Select),
                ButtonAction::Command(Command::ZoomToFit),
                ButtonAction::Command(Command::PreviousColor),
                ButtonAction::Command(Command::NextColor),
            ],
            ring_modes: vec![
                RingAction::Zoom,
                RingAction::StrokeWidth,
                RingAction::StrokeColor,
                RingAction::UndoRedo,
            ],
            ring_step_degrees: 15.0,
            dial_actions: vec![RingAction::Zoom, RingAction::StrokeWidth],
            dial_detents_per_step: 1.0,
            dial_reverse: false,
            strip_modes: vec![RingAction::ScrollVertical],
            pinch_zoom: true,
        }
    }
}

impl TabletSettings {
    /// A plain pressure-sensitive pen whose lower barrel button pans, and
    /// nothing else.
    pub fn plain() -> Self {
        Self {
            activation: Activation::Off,
            pressure_curve: 1.0,
            pressure_min: 0.0,
            pressure_max: 1.0,
            pressure_smoothing: 0.0,
            stabilizer: false,
            tilt_width: 0.0,
            calligraphy: false,
            hover_fade: false,
            eraser_end_switches_tool: false,
            eraser_pressure: 0.0,
            remember_tool_per_pen: false,
            stylus_buttons: [
                ButtonAction::RightClick,
                ButtonAction::Pan,
                ButtonAction::None,
            ],
            pad_buttons: Vec::new(),
            ring_modes: Vec::new(),
            dial_actions: Vec::new(),
            strip_modes: Vec::new(),
            pinch_zoom: false,
            ..Self::default()
        }
    }

    /// The settings in effect. Button, ExpressKey, dial, ring, strip and
    /// pinch mappings always apply; the pen-feel settings (pressure,
    /// stabilizer, tilt, eraser, hover) only when `pen_feel_applies`.
    pub fn effective(&self, pen_feel_applies: bool) -> Self {
        if pen_feel_applies {
            return self.clone();
        }
        Self {
            stylus_buttons: self.stylus_buttons,
            pad_buttons: self.pad_buttons.clone(),
            ring_modes: self.ring_modes.clone(),
            ring_step_degrees: self.ring_step_degrees,
            dial_actions: self.dial_actions.clone(),
            dial_detents_per_step: self.dial_detents_per_step,
            dial_reverse: self.dial_reverse,
            strip_modes: self.strip_modes.clone(),
            pinch_zoom: self.pinch_zoom,
            ..Self::plain()
        }
    }

    /// Whether these settings apply, given the vendor ids of connected tablets.
    pub fn applies(&self, tablet_vendors: &[u32]) -> bool {
        match self.activation {
            Activation::WacomOnly => tablet_vendors.contains(&WACOM_VENDOR_ID),
            Activation::AnyTablet => !tablet_vendors.is_empty(),
            Activation::Off => false,
        }
    }

    /// Map raw tablet pressure (0.0-1.0) through the range and curve.
    pub fn map_pressure(&self, raw: f64) -> f64 {
        let range = (self.pressure_max - self.pressure_min).max(0.01);
        let p = ((raw - self.pressure_min) / range).clamp(0.0, 1.0);
        p.powf(self.pressure_curve.max(0.05))
    }

    /// Width factor (0.0-1.0) from tilt (degrees from vertical, x and y).
    pub fn tilt_factor(&self, (tilt_x, tilt_y): (f64, f64)) -> f64 {
        if self.tilt_width <= 0.0 {
            return 1.0;
        }
        // Pens report up to about 60° of tilt.
        let amount = (tilt_x.hypot(tilt_y) / 60.0).clamp(0.0, 1.0);
        (1.0 - self.tilt_width) + self.tilt_width * amount
    }

    /// Width factor (0.0-1.0) for drawing in direction `stroke_angle`
    /// (radians) with the nib at `nib_angle` (radians).
    pub fn nib_factor(&self, stroke_angle: f64, nib_angle: f64) -> f64 {
        if !self.calligraphy {
            return 1.0;
        }
        let across = (stroke_angle - nib_angle).sin().abs();
        self.nib_thinness + (1.0 - self.nib_thinness) * across
    }

    /// Settings file location (native only).
    #[cfg(not(target_arch = "wasm32"))]
    fn path() -> Option<std::path::PathBuf> {
        Some(dirs::config_dir()?.join("drafftink").join("tablet.json"))
    }

    /// Load saved settings, or the defaults.
    pub fn load() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(json) = Self::path().and_then(|p| std::fs::read_to_string(p).ok()) {
            match serde_json::from_str(&json) {
                Ok(settings) => return settings,
                Err(e) => log::warn!("Ignoring invalid tablet settings: {e}"),
            }
        }
        Self::default()
    }

    /// Save settings (native only).
    pub fn save(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(path) = Self::path() else {
                return;
            };
            let result = path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| {
                    let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
                    std::fs::write(&path, json)
                });
            if let Err(e) = result {
                log::warn!("Failed to save tablet settings: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_applies() {
        let wacom = [WACOM_VENDOR_ID];
        let other = [0x28bd];
        let mut s = TabletSettings::default();
        assert!(s.applies(&wacom));
        assert!(!s.applies(&other));
        assert!(!s.applies(&[]));
        s.activation = Activation::AnyTablet;
        assert!(s.applies(&other));
        assert!(!s.applies(&[]));
        s.activation = Activation::Off;
        assert!(!s.applies(&wacom));
    }

    #[test]
    fn test_map_pressure() {
        let mut s = TabletSettings::default();
        assert_eq!(s.map_pressure(0.5), 0.5);
        s.pressure_min = 0.1;
        s.pressure_max = 0.6;
        assert_eq!(s.map_pressure(0.05), 0.0);
        assert!((s.map_pressure(0.35) - 0.5).abs() < 1e-9);
        assert_eq!(s.map_pressure(0.9), 1.0);
        s.pressure_curve = 2.0;
        assert!((s.map_pressure(0.35) - 0.25).abs() < 1e-9);
    }

    #[test]
    fn test_nib_and_tilt() {
        let mut s = TabletSettings::default();
        assert_eq!(s.nib_factor(0.0, 0.0), 1.0);
        assert_eq!(s.tilt_factor((30.0, 0.0)), 1.0);
        s.calligraphy = true;
        assert!((s.nib_factor(0.0, 0.0) - s.nib_thinness).abs() < 1e-9);
        assert!((s.nib_factor(std::f64::consts::FRAC_PI_2, 0.0) - 1.0).abs() < 1e-9);
        s.tilt_width = 0.5;
        assert_eq!(s.tilt_factor((0.0, 0.0)), 0.5);
        assert_eq!(s.tilt_factor((0.0, 60.0)), 1.0);
    }

    #[test]
    fn test_partial_json_uses_defaults() {
        let s: TabletSettings = serde_json::from_str(r#"{"pressure_curve": 1.5}"#).unwrap();
        assert_eq!(s.pressure_curve, 1.5);
        assert_eq!(s.ring_modes, TabletSettings::default().ring_modes);
    }
}
