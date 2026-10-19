//! User-configurable keyboard shortcuts.
//!
//! Every shortcut is a [`Command`] bound to zero or more [`KeyChord`]s. The
//! defaults are [`Keybindings::default`]; the user's changes are saved to
//! `~/.config/drafftink/keybindings.json`.

use drafftink_core::shapes::Brush;
use drafftink_core::tools::ToolKind;
use drafftink_render::GridStyle;
use serde::{Deserialize, Serialize};
use winit::event::KeyEvent;
use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};

/// Something a shortcut can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Command {
    Tool(ToolKind),
    /// Show the tools, colours, widths and properties next to the pointer;
    /// press again to hide them.
    QuickToolbars,
    // Edit
    Undo,
    Redo,
    SelectAll,
    Copy,
    Cut,
    Paste,
    Duplicate,
    Delete,
    Group,
    Ungroup,
    /// Close dialogs, or cancel and switch to Select.
    Cancel,
    NudgeUp,
    NudgeDown,
    NudgeLeft,
    NudgeRight,
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
    FlipHorizontal,
    FlipVertical,
    AlignLeft,
    AlignCenterH,
    AlignRight,
    AlignTop,
    AlignCenterV,
    AlignBottom,
    // Style
    /// Quick stroke colour by swatch position (0-5).
    StrokeColor(u8),
    NextColor,
    PreviousColor,
    /// Stroke width by position in [`STROKE_WIDTH_PRESETS`].
    StrokeWidth(u8),
    /// Switch to the pen with this brush.
    Brush(Brush),
    Thicker,
    Thinner,
    // View
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomToFit,
    CenterCanvas,
    /// Like the commands above, but pressing again (without moving the view
    /// in between) returns to the view before the first press.
    ZoomResetOrBack,
    ZoomToFitOrBack,
    CenterCanvasOrBack,
    ToggleGrid,
    /// Show this grid style.
    Grid(GridStyle),
    ToggleGridSnap,
    ToggleSmartSnap,
    ToggleAngleSnap,
    // File
    /// New empty document in its own tab.
    NewDocument,
    Save,
    Open,
    ExportPng,
    CopyPng,
    // App
    Settings,
}

/// Groups of commands, in the order shown in the settings.
pub const COMMAND_GROUPS: &[(&str, &[Command])] = &[
    (
        "Tools",
        &[
            Command::Tool(ToolKind::Select),
            Command::Tool(ToolKind::Pan),
            Command::Tool(ToolKind::Rectangle),
            Command::Tool(ToolKind::Ellipse),
            Command::Tool(ToolKind::Arrow),
            Command::Tool(ToolKind::Line),
            Command::Tool(ToolKind::Freehand),
            Command::Tool(ToolKind::Highlighter),
            Command::Tool(ToolKind::Eraser),
            Command::Tool(ToolKind::Text),
            Command::Tool(ToolKind::Math),
            Command::Tool(ToolKind::LaserPointer),
            Command::QuickToolbars,
        ],
    ),
    (
        "Edit",
        &[
            Command::Undo,
            Command::Redo,
            Command::SelectAll,
            Command::Copy,
            Command::Cut,
            Command::Paste,
            Command::Duplicate,
            Command::Delete,
            Command::Group,
            Command::Ungroup,
            Command::Cancel,
            Command::NudgeUp,
            Command::NudgeDown,
            Command::NudgeLeft,
            Command::NudgeRight,
        ],
    ),
    (
        "Arrange",
        &[
            Command::BringToFront,
            Command::BringForward,
            Command::SendBackward,
            Command::SendToBack,
            Command::FlipHorizontal,
            Command::FlipVertical,
            Command::AlignLeft,
            Command::AlignCenterH,
            Command::AlignRight,
            Command::AlignTop,
            Command::AlignCenterV,
            Command::AlignBottom,
        ],
    ),
    (
        "Style",
        &[
            Command::StrokeColor(0),
            Command::StrokeColor(1),
            Command::StrokeColor(2),
            Command::StrokeColor(3),
            Command::StrokeColor(4),
            Command::StrokeColor(5),
            Command::PreviousColor,
            Command::NextColor,
            Command::StrokeWidth(0),
            Command::StrokeWidth(1),
            Command::StrokeWidth(2),
            Command::StrokeWidth(3),
            Command::StrokeWidth(4),
            Command::StrokeWidth(5),
            Command::StrokeWidth(6),
            Command::Thinner,
            Command::Thicker,
        ],
    ),
    (
        "Brushes",
        &[
            Command::Brush(Brush::Pen),
            Command::Brush(Brush::PointedNib),
            Command::Brush(Brush::BroadNib),
            Command::Brush(Brush::BrushPen),
            Command::Brush(Brush::Marker),
            Command::Brush(Brush::Pencil),
            Command::Brush(Brush::Soft),
        ],
    ),
    (
        "View",
        &[
            Command::ZoomIn,
            Command::ZoomOut,
            Command::ZoomReset,
            Command::ZoomToFit,
            Command::CenterCanvas,
            Command::ZoomResetOrBack,
            Command::ZoomToFitOrBack,
            Command::CenterCanvasOrBack,
            Command::ToggleGrid,
            Command::Grid(GridStyle::None),
            Command::Grid(GridStyle::Lines),
            Command::Grid(GridStyle::HorizontalLines),
            Command::Grid(GridStyle::CrossPlus),
            Command::Grid(GridStyle::Dots),
            Command::Grid(GridStyle::Handwriting),
            Command::Grid(GridStyle::Calligraphy),
            Command::ToggleGridSnap,
            Command::ToggleSmartSnap,
            Command::ToggleAngleSnap,
        ],
    ),
    (
        "File",
        &[
            Command::NewDocument,
            Command::Save,
            Command::Open,
            Command::ExportPng,
            Command::CopyPng,
            Command::Settings,
        ],
    ),
];

/// Names of the quick colour swatches, by position.
pub const QUICK_COLOR_NAMES: [&str; 6] = ["Blue", "Red", "Emerald", "Amber", "Purple", "Slate"];
/// Stroke width presets (width in px, name), by position.
pub const STROKE_WIDTH_PRESETS: [(f32, &str); 7] = [
    (1.0, "Thin"),
    (1.25, "Fine"),
    (1.5, "Light"),
    (1.75, "Regular"),
    (2.0, "Normal"),
    (4.0, "Bold"),
    (8.0, "Extra bold"),
];

impl Command {
    /// For the "… / back" commands: the plain command they run, if any.
    pub fn view_toggle_base(self) -> Option<Self> {
        match self {
            Self::ZoomResetOrBack => Some(Self::ZoomReset),
            Self::ZoomToFitOrBack => Some(Self::ZoomToFit),
            Self::CenterCanvasOrBack => Some(Self::CenterCanvas),
            _ => None,
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::Tool(tool) => format!("{} tool", tool_label(tool)),
            Self::Undo => "Undo".into(),
            Self::Redo => "Redo".into(),
            Self::SelectAll => "Select all".into(),
            Self::Copy => "Copy".into(),
            Self::Cut => "Cut".into(),
            Self::Paste => "Paste".into(),
            Self::Duplicate => "Duplicate".into(),
            Self::Delete => "Delete selection".into(),
            Self::Group => "Group".into(),
            Self::Ungroup => "Ungroup".into(),
            Self::Cancel => "Cancel / close dialog".into(),
            Self::NudgeUp => "Nudge up".into(),
            Self::NudgeDown => "Nudge down".into(),
            Self::NudgeLeft => "Nudge left".into(),
            Self::NudgeRight => "Nudge right".into(),
            Self::BringToFront => "Bring to front".into(),
            Self::BringForward => "Bring forward".into(),
            Self::SendBackward => "Send backward".into(),
            Self::SendToBack => "Send to back".into(),
            Self::FlipHorizontal => "Flip horizontally".into(),
            Self::FlipVertical => "Flip vertically".into(),
            Self::AlignLeft => "Align left".into(),
            Self::AlignCenterH => "Align centre horizontally".into(),
            Self::AlignRight => "Align right".into(),
            Self::AlignTop => "Align top".into(),
            Self::AlignCenterV => "Align centre vertically".into(),
            Self::AlignBottom => "Align bottom".into(),
            Self::StrokeColor(i) => format!(
                "Stroke colour: {}",
                QUICK_COLOR_NAMES.get(i as usize).unwrap_or(&"?")
            ),
            Self::NextColor => "Next stroke colour".into(),
            Self::PreviousColor => "Previous stroke colour".into(),
            Self::StrokeWidth(i) => match STROKE_WIDTH_PRESETS.get(i as usize) {
                Some((width, name)) => format!("Stroke width: {name} ({width} px)"),
                None => "Stroke width: ?".into(),
            },
            Self::QuickToolbars => "Tools and colours at pointer".into(),
            Self::Brush(brush) => format!("Brush: {}", brush.name()),
            Self::Grid(style) => format!("Grid: {}", style.name()),
            Self::Thicker => "Thicker stroke".into(),
            Self::Thinner => "Thinner stroke".into(),
            Self::ZoomIn => "Zoom in".into(),
            Self::ZoomOut => "Zoom out".into(),
            Self::ZoomReset => "Zoom to 100%".into(),
            Self::ZoomToFit => "Zoom to fit".into(),
            Self::CenterCanvas => "Centre canvas".into(),
            Self::ZoomResetOrBack => "Zoom to 100% / back".into(),
            Self::ZoomToFitOrBack => "Zoom to fit / back".into(),
            Self::CenterCanvasOrBack => "Centre canvas / back".into(),
            Self::ToggleGrid => "Cycle grid style".into(),
            Self::ToggleGridSnap => "Toggle grid snap".into(),
            Self::ToggleSmartSnap => "Toggle smart guides".into(),
            Self::ToggleAngleSnap => "Toggle angle snap".into(),
            Self::NewDocument => "New document".into(),
            Self::Save => "Save".into(),
            Self::Open => "Open".into(),
            Self::ExportPng => "Export PNG".into(),
            Self::CopyPng => "Copy selection as PNG".into(),
            Self::Settings => "Settings".into(),
        }
    }
}

pub fn tool_label(tool: ToolKind) -> &'static str {
    match tool {
        ToolKind::Select => "Select",
        ToolKind::Pan => "Pan",
        ToolKind::Rectangle => "Rectangle",
        ToolKind::Ellipse => "Ellipse",
        ToolKind::Line => "Line",
        ToolKind::Arrow => "Arrow",
        ToolKind::Freehand => "Pen",
        ToolKind::Highlighter => "Highlighter",
        ToolKind::Eraser => "Eraser",
        ToolKind::Text => "Text",
        ToolKind::Math => "Math",
        ToolKind::LaserPointer => "Laser",
    }
}

/// A key with modifiers. `key` is layout-aware and ignores modifiers: a
/// lowercase character ("a", "1", "["), or a named key ("Delete", "Home").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyChord {
    pub key: String,
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
}

impl KeyChord {
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            ctrl: false,
            shift: false,
            alt: false,
        }
    }

    pub fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    /// The chord for a key press, or `None` for modifier keys on their own.
    pub fn from_event(event: &KeyEvent, ctrl: bool, shift: bool, alt: bool) -> Option<Self> {
        // Digits by position, so Shift/Alt+digit work on any layout.
        if let PhysicalKey::Code(code) = event.physical_key {
            if let Some(digit) = digit_of(code) {
                return Some(Self {
                    key: digit.to_string(),
                    ctrl,
                    shift,
                    alt,
                });
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        let key = {
            use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
            event.key_without_modifiers()
        };
        #[cfg(target_arch = "wasm32")]
        let key = event.logical_key.clone();

        let key = match key {
            Key::Character(c) => c.to_lowercase(),
            Key::Named(
                NamedKey::Control
                | NamedKey::Shift
                | NamedKey::Alt
                | NamedKey::Super
                | NamedKey::Meta
                | NamedKey::Hyper
                | NamedKey::AltGraph
                | NamedKey::CapsLock,
            ) => return None,
            Key::Named(NamedKey::Space) => "Space".into(),
            Key::Named(named) => format!("{named:?}"),
            _ => return None,
        };
        Some(Self {
            key,
            ctrl,
            shift,
            alt,
        })
    }

    /// Human-readable form, e.g. "Ctrl+Shift+Z".
    pub fn label(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".into());
        }
        if self.alt {
            parts.push("Alt".into());
        }
        if self.shift {
            parts.push("Shift".into());
        }
        let key = match self.key.as_str() {
            "ArrowUp" => "↑".to_string(),
            "ArrowDown" => "↓".to_string(),
            "ArrowLeft" => "←".to_string(),
            "ArrowRight" => "→".to_string(),
            "Escape" => "Esc".to_string(),
            k if k.chars().count() == 1 => k.to_uppercase(),
            k => k.to_string(),
        };
        parts.push(key);
        parts.join("+")
    }
}

fn digit_of(code: KeyCode) -> Option<u8> {
    Some(match code {
        KeyCode::Digit0 => 0,
        KeyCode::Digit1 => 1,
        KeyCode::Digit2 => 2,
        KeyCode::Digit3 => 3,
        KeyCode::Digit4 => 4,
        KeyCode::Digit5 => 5,
        KeyCode::Digit6 => 6,
        KeyCode::Digit7 => 7,
        KeyCode::Digit8 => 8,
        KeyCode::Digit9 => 9,
        _ => return None,
    })
}

/// A command and the chords bound to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub command: Command,
    pub keys: Vec<KeyChord>,
}

/// All keyboard shortcuts.
#[derive(Debug, Clone, PartialEq)]
pub struct Keybindings {
    pub bindings: Vec<Binding>,
}

impl Default for Keybindings {
    fn default() -> Self {
        use Command::*;
        let k = KeyChord::new;
        let tool = |t: ToolKind, keys: &[&str]| Binding {
            command: Tool(t),
            keys: keys.iter().map(|key| k(key)).collect(),
        };
        let bind = |command, keys: Vec<KeyChord>| Binding { command, keys };
        let mut bindings = vec![
            tool(ToolKind::Select, &["v", "1"]),
            tool(ToolKind::Pan, &["h"]),
            tool(ToolKind::Rectangle, &["r", "2"]),
            tool(ToolKind::Ellipse, &["o", "4"]),
            tool(ToolKind::Arrow, &["a", "5"]),
            tool(ToolKind::Line, &["l", "6"]),
            tool(ToolKind::Freehand, &["p", "7"]),
            tool(ToolKind::Highlighter, &[]),
            tool(ToolKind::Eraser, &["e"]),
            tool(ToolKind::Text, &["t", "8"]),
            tool(ToolKind::Math, &["m", "9"]),
            tool(ToolKind::LaserPointer, &["z"]),
            bind(Undo, vec![k("z").ctrl()]),
            bind(Redo, vec![k("z").ctrl().shift(), k("y").ctrl()]),
            bind(SelectAll, vec![k("a").ctrl()]),
            bind(Copy, vec![k("c").ctrl()]),
            bind(Cut, vec![k("x").ctrl()]),
            bind(Paste, vec![k("v").ctrl()]),
            bind(Duplicate, vec![k("d").ctrl()]),
            bind(Delete, vec![k("Delete"), k("Backspace")]),
            bind(Group, vec![k("g").ctrl()]),
            bind(Ungroup, vec![k("g").ctrl().shift()]),
            bind(Cancel, vec![k("Escape")]),
            bind(NudgeUp, vec![k("ArrowUp")]),
            bind(NudgeDown, vec![k("ArrowDown")]),
            bind(NudgeLeft, vec![k("ArrowLeft")]),
            bind(NudgeRight, vec![k("ArrowRight")]),
            bind(PreviousColor, vec![k("[")]),
            bind(NextColor, vec![k("]")]),
            bind(QuickToolbars, vec![k("q")]),
            bind(ZoomIn, vec![k("=").ctrl(), k("=").ctrl().shift()]),
            bind(ZoomOut, vec![k("-").ctrl()]),
            bind(ZoomReset, vec![k("0").ctrl()]),
            bind(ZoomToFit, vec![k("1").shift()]),
            bind(CenterCanvas, vec![k("Home")]),
            bind(NewDocument, vec![k("n").ctrl()]),
            bind(Save, vec![k("s").ctrl()]),
            bind(Open, vec![k("o").ctrl()]),
            bind(ExportPng, vec![k("e").ctrl()]),
            bind(CopyPng, vec![k("c").ctrl().shift(), k("e").ctrl().shift()]),
        ];
        for i in 0..6u8 {
            bindings.push(bind(StrokeColor(i), vec![k(&(i + 1).to_string()).alt()]));
        }
        // Thin, Normal, Bold and Extra bold.
        for (preset, key) in [(0, "7"), (4, "8"), (5, "9"), (6, "0")] {
            bindings.push(bind(StrokeWidth(preset), vec![k(key).alt()]));
        }
        // Commands without a default key, so they appear in the settings.
        for (_, commands) in COMMAND_GROUPS {
            for &command in *commands {
                if !bindings.iter().any(|b| b.command == command) {
                    bindings.push(bind(command, Vec::new()));
                }
            }
        }
        Self { bindings }
    }
}

impl Keybindings {
    /// The command bound to `chord`, if any.
    pub fn command_for(&self, chord: &KeyChord) -> Option<Command> {
        self.bindings
            .iter()
            .find(|b| b.keys.contains(chord))
            .map(|b| b.command)
    }

    pub fn keys(&self, command: Command) -> &[KeyChord] {
        self.bindings
            .iter()
            .find(|b| b.command == command)
            .map_or(&[], |b| &b.keys)
    }

    /// First key of `command` for tooltips, e.g. "Ctrl+Z", or "" if unbound.
    pub fn hint(&self, command: Command) -> String {
        self.keys(command)
            .first()
            .map(KeyChord::label)
            .unwrap_or_default()
    }

    /// `text` with the command's key appended, e.g. "Undo (Ctrl+Z)".
    pub fn with_hint(&self, text: &str, command: Command) -> String {
        match self.hint(command) {
            hint if hint.is_empty() => text.to_string(),
            hint => format!("{text} ({hint})"),
        }
    }

    /// Bind `chord` to `command`, taking it from any other command. Returns
    /// the command it was taken from.
    pub fn bind(&mut self, command: Command, chord: KeyChord) -> Option<Command> {
        let mut previous = None;
        for binding in &mut self.bindings {
            if binding.command != command && binding.keys.contains(&chord) {
                binding.keys.retain(|k| *k != chord);
                previous = Some(binding.command);
            }
        }
        if let Some(binding) = self.bindings.iter_mut().find(|b| b.command == command) {
            if !binding.keys.contains(&chord) {
                binding.keys.push(chord);
            }
        }
        previous
    }

    pub fn unbind(&mut self, command: Command, chord: &KeyChord) {
        if let Some(binding) = self.bindings.iter_mut().find(|b| b.command == command) {
            binding.keys.retain(|k| k != chord);
        }
    }

    /// Restore `command`'s default keys, taking them from other commands.
    pub fn reset(&mut self, command: Command) {
        let defaults = Self::default().keys(command).to_vec();
        for chord in &defaults {
            self.bind(command, chord.clone());
        }
        if let Some(binding) = self.bindings.iter_mut().find(|b| b.command == command) {
            binding.keys = defaults;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn path() -> Option<std::path::PathBuf> {
        Some(
            dirs::config_dir()?
                .join("drafftink")
                .join("keybindings.json"),
        )
    }

    /// Load saved bindings over the defaults. Commands missing from the file
    /// (e.g. added in a newer version) keep their default keys.
    pub fn load() -> Self {
        let mut bindings = Self::default();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(json) = Self::path().and_then(|p| std::fs::read_to_string(p).ok()) {
            match serde_json::from_str::<Vec<Binding>>(&json) {
                Ok(saved) => {
                    for saved in saved {
                        if let Some(b) = bindings
                            .bindings
                            .iter_mut()
                            .find(|b| b.command == saved.command)
                        {
                            b.keys = saved.keys;
                        }
                    }
                }
                Err(e) => log::warn!("Ignoring invalid keybindings: {e}"),
            }
        }
        bindings
    }

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
                    let json = serde_json::to_string_pretty(&self.bindings)
                        .map_err(std::io::Error::other)?;
                    std::fs::write(&path, json)
                });
            if let Err(e) = result {
                log::warn!("Failed to save keybindings: {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults_match_previous_shortcuts() {
        let kb = Keybindings::default();
        let k = KeyChord::new;
        assert_eq!(
            kb.command_for(&k("v")),
            Some(Command::Tool(ToolKind::Select))
        );
        assert_eq!(
            kb.command_for(&k("1")),
            Some(Command::Tool(ToolKind::Select))
        );
        assert_eq!(
            kb.command_for(&k("z")),
            Some(Command::Tool(ToolKind::LaserPointer))
        );
        assert_eq!(kb.command_for(&k("z").ctrl()), Some(Command::Undo));
        assert_eq!(kb.command_for(&k("z").ctrl().shift()), Some(Command::Redo));
        assert_eq!(kb.command_for(&k("1").alt()), Some(Command::StrokeColor(0)));
        assert_eq!(kb.command_for(&k("0").alt()), Some(Command::StrokeWidth(6)));
        assert_eq!(kb.command_for(&k("1").shift()), Some(Command::ZoomToFit));
        assert_eq!(kb.command_for(&k("q")), Some(Command::QuickToolbars));
        assert_eq!(kb.command_for(&k("j")), None);
    }

    #[test]
    fn test_no_default_chord_is_bound_twice() {
        let kb = Keybindings::default();
        let mut seen = std::collections::HashSet::new();
        for b in &kb.bindings {
            for key in &b.keys {
                assert!(seen.insert(key.clone()), "{key:?} bound twice");
            }
        }
    }

    #[test]
    fn test_every_command_is_listed() {
        let kb = Keybindings::default();
        let listed: Vec<Command> = COMMAND_GROUPS
            .iter()
            .flat_map(|(_, c)| c.iter().copied())
            .collect();
        for b in &kb.bindings {
            assert!(
                listed.contains(&b.command),
                "{:?} missing from COMMAND_GROUPS",
                b.command
            );
        }
        assert_eq!(listed.len(), kb.bindings.len());
    }

    #[test]
    fn test_rebind_takes_key_from_other_command() {
        let mut kb = Keybindings::default();
        let previous = kb.bind(Command::Tool(ToolKind::Highlighter), KeyChord::new("z"));
        assert_eq!(previous, Some(Command::Tool(ToolKind::LaserPointer)));
        assert_eq!(
            kb.command_for(&KeyChord::new("z")),
            Some(Command::Tool(ToolKind::Highlighter))
        );
        assert!(kb.keys(Command::Tool(ToolKind::LaserPointer)).is_empty());
        kb.reset(Command::Tool(ToolKind::LaserPointer));
        assert_eq!(
            kb.command_for(&KeyChord::new("z")),
            Some(Command::Tool(ToolKind::LaserPointer))
        );
        assert!(kb.keys(Command::Tool(ToolKind::Highlighter)).is_empty());
    }

    #[test]
    fn test_labels() {
        assert_eq!(KeyChord::new("z").ctrl().shift().label(), "Ctrl+Shift+Z");
        assert_eq!(KeyChord::new("ArrowUp").label(), "↑");
        assert_eq!(KeyChord::new("1").alt().label(), "Alt+1");
    }
}
