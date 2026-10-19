//! Keyboard shortcut registry and documentation.

/// A keyboard shortcut definition.
#[derive(Debug, Clone)]
pub struct Shortcut {
    pub key: &'static str,
    pub ctrl: bool,
    pub shift: bool,
    pub description: &'static str,
}

impl Shortcut {
    pub const fn new(
        key: &'static str,
        ctrl: bool,
        shift: bool,
        description: &'static str,
    ) -> Self {
        Self {
            key,
            ctrl,
            shift,
            description,
        }
    }

    /// Format the shortcut for display (e.g., "Ctrl+S").
    pub fn format(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(self.key);
        parts.join("+")
    }
}

/// Registry of all keyboard shortcuts.
pub struct ShortcutRegistry;

impl ShortcutRegistry {
    /// Get all registered shortcuts.
    pub fn all() -> Vec<Shortcut> {
        vec![
            Shortcut::new("A", true, false, "Select all shapes"),
            Shortcut::new("S", true, false, "Save Local"),
            Shortcut::new("O", true, false, "Open..."),
            Shortcut::new("E", true, false, "Export to PNG"),
            Shortcut::new("E", true, true, "Copy selection as PNG"),
            Shortcut::new("Z", true, false, "Undo"),
            Shortcut::new("Z", true, true, "Redo"),
            Shortcut::new("Y", true, false, "Redo"),
            Shortcut::new("G", true, false, "Group selected shapes"),
            Shortcut::new("G", true, true, "Ungroup selected shapes"),
            Shortcut::new("C", true, true, "Copy selection as PNG"),
            Shortcut::new("C", true, false, "Copy shapes"),
            Shortcut::new("X", true, false, "Cut shapes"),
            Shortcut::new("V", true, false, "Paste shapes or image"),
            Shortcut::new("Delete", false, false, "Delete selected shapes"),
            Shortcut::new("Backspace", false, false, "Delete selected shapes"),
            Shortcut::new("Escape", false, false, "Cancel current action"),
            Shortcut::new(
                "Alt+1-6",
                false,
                false,
                "Stroke colour: Blue, Red, Emerald, Amber, Purple, Slate",
            ),
            Shortcut::new("[ / ]", false, false, "Previous / next stroke colour"),
            Shortcut::new(
                "Alt+7/8/9/0",
                false,
                false,
                "Stroke width: Thin, Normal, Bold, Extra Bold",
            ),
            Shortcut::new("= / -", true, false, "Zoom in / out"),
            Shortcut::new("0", true, false, "Reset zoom to 100%"),
            Shortcut::new("1", false, true, "Zoom to fit selection or all"),
            Shortcut::new("Home", false, false, "Center canvas"),
            Shortcut::new(
                "Space",
                false,
                false,
                "Draw / click at cursor (hold like the left mouse button)",
            ),
            Shortcut::new(
                "Shift+Drag",
                false,
                false,
                "Maintain aspect ratio while resizing",
            ),
        ]
    }

    /// Print all shortcuts to console.
    pub fn print_all() {
        println!("\n=== Keyboard Shortcuts ===");
        for shortcut in Self::all() {
            println!("  {:20} {}", shortcut.format(), shortcut.description);
        }
        println!();
    }
}
