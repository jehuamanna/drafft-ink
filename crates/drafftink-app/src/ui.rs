//! UI components using egui.

use drafftink_core::practice::{PracticeSheet, SheetKind};
use drafftink_core::shapes::{
    Brush, FillPattern, FontFamily, FontWeight, Shape, ShapeId, ShapeStyle,
};
use drafftink_core::sync::ConnectionState;
use drafftink_core::tools::ToolKind;
use drafftink_render::{GridStyle, WritingGuides};
use egui::{
    Align2, Color32, Context, CornerRadius, Frame, ImageSource, Margin, Pos2, Rect, Stroke, Vec2,
    include_image,
};

#[cfg(target_arch = "wasm32")]
use crate::app::file_ops;
use crate::keybindings::{COMMAND_GROUPS, Command, Keybindings, STROKE_WIDTH_PRESETS};
use crate::tablet::{Devices, PenInfo};
use crate::tablet_settings::{
    Activation, ButtonAction, NibAngle, RingAction, TabletSettings, WACOM_VENDOR_ID,
};

// Re-export from widgets crate for consistent styling
use drafftink_widgets::{
    ColorGrid, ColorSwatch, ColorSwatchWithWheel, FontSizeButton, IconButton, NoColorSwatch,
    StrokeWidthButton, TAILWIND_COLORS, ToggleButton, default_btn, input_text,
    menu_item as widgets_menu_item, menu_item_enabled as widgets_menu_item_enabled,
    menu_separator as widgets_menu_separator, panel_frame as widgets_panel_frame, primary_btn,
    secondary_btn, section_label as widgets_section_label,
    vertical_separator as widgets_vertical_separator,
};

/// Properties of the currently selected shape(s) for the right panel.
#[derive(Debug, Clone, Default)]
pub struct SelectedShapeProps {
    /// Is a single shape selected?
    pub has_selection: bool,
    /// Number of selected shapes.
    pub selection_count: usize,
    /// Is the selected shape a text shape?
    pub is_text: bool,
    /// Is the selected shape a math shape?
    pub is_math: bool,
    /// Is the selected shape a rectangle?
    pub is_rectangle: bool,
    /// Is the selected shape a line?
    pub is_line: bool,
    /// Is the selected shape an arrow?
    pub is_arrow: bool,
    /// Is the selected shape a freehand?
    pub is_freehand: bool,
    /// Brush of the selected pen stroke, or of the pen tool.
    pub brush: Option<Brush>,
    /// Broad nib angle in degrees (with `brush`).
    pub nib_angle: f64,
    /// Font size (for text shapes).
    pub font_size: f32,
    /// Font family (for text shapes).
    pub font_family: FontFamily,
    /// Font weight (for text shapes).
    pub font_weight: FontWeight,
    /// Corner radius (for rectangle shapes).
    pub corner_radius: f32,
    /// Path style for lines/arrows (0 = Direct, 1 = Flowing, 2 = Angular).
    pub path_style: u8,
    /// Stroke style for lines/arrows (0 = Solid, 1 = Dashed, 2 = Dotted).
    pub stroke_style: u8,
    /// Sloppiness level (0 = Architect, 1 = Artist, 2 = Cartoonist).
    pub sloppiness: u8,
    /// Fill pattern (0 = Solid, 1 = Hachure, etc).
    pub fill_pattern: u8,
    /// Has fill color set.
    pub has_fill: bool,
    /// Is a drawing tool active (show panel for new shapes)?
    pub is_drawing_tool: bool,
    /// Is the active tool for rectangles?
    pub tool_is_rectangle: bool,
    /// Calligraphy mode for freehand.
    pub calligraphy_mode: bool,
    /// Pressure simulation mode for freehand.
    pub pressure_simulation: bool,
    /// Shape opacity (0.0-1.0).
    pub opacity: f32,
}

impl SelectedShapeProps {
    /// Create from a selected shape.
    pub fn from_shape(shape: &Shape) -> Self {
        Self::from_shape_with_count(shape, 1)
    }

    /// Create from a selected shape with a specific count.
    pub fn from_shape_with_count(shape: &Shape, count: usize) -> Self {
        let sloppiness = shape.style().sloppiness as u8;
        let fill_pattern = shape.style().fill_pattern as u8;
        let has_fill = shape.style().fill_color.is_some();
        let opacity = shape.style().opacity as f32;

        match shape {
            Shape::Text(text) => Self {
                has_selection: true,
                selection_count: count,
                is_text: true,
                font_size: text.font_size as f32,
                font_family: text.font_family,
                font_weight: text.font_weight,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Rectangle(rect) => Self {
                has_selection: true,
                selection_count: count,
                is_rectangle: true,
                corner_radius: rect.corner_radius as f32,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Line(line) => Self {
                has_selection: true,
                selection_count: count,
                is_line: true,
                path_style: line.path_style as u8,
                stroke_style: line.stroke_style as u8,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Arrow(arrow) => Self {
                has_selection: true,
                selection_count: count,
                is_arrow: true,
                path_style: arrow.path_style as u8,
                stroke_style: arrow.stroke_style as u8,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Freehand(freehand) => Self {
                has_selection: true,
                selection_count: count,
                is_freehand: true,
                brush: Some(freehand.brush),
                nib_angle: freehand.nib_angle,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            Shape::Math(math) => Self {
                has_selection: true,
                selection_count: count,
                is_math: true,
                font_size: math.font_size as f32,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
            _ => Self {
                has_selection: true,
                selection_count: count,
                sloppiness,
                fill_pattern,
                has_fill,
                opacity,
                ..Default::default()
            },
        }
    }

    /// Create props for drawing tool mode (no selection, configuring new shapes).
    pub fn for_tool(
        tool: drafftink_core::tools::ToolKind,
        ui_state: &UiState,
        calligraphy: bool,
        pressure_sim: bool,
        (brush, nib_angle): (Brush, f64),
    ) -> Self {
        use drafftink_core::tools::ToolKind;
        Self {
            is_drawing_tool: true,
            tool_is_rectangle: tool == ToolKind::Rectangle,
            is_line: tool == ToolKind::Line,
            is_arrow: tool == ToolKind::Arrow,
            is_freehand: tool == ToolKind::Freehand || tool == ToolKind::Highlighter,
            brush: (tool == ToolKind::Freehand).then_some(brush),
            nib_angle,
            calligraphy_mode: calligraphy,
            pressure_simulation: pressure_sim,
            sloppiness: ui_state.sloppiness as u8,
            fill_pattern: ui_state.fill_pattern as u8,
            has_fill: ui_state.fill_color.is_some(),
            path_style: ui_state.path_style,
            corner_radius: ui_state.corner_radius,
            ..Default::default()
        }
    }
}

// Tailwind colors are now imported from drafftink_widgets

/// Range of the stroke width slider, in px.
const STROKE_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 0.25..=64.0;

/// Widths that "thicker" and "thinner" step through: the presets, then
/// wider ones up to the end of the slider.
const STROKE_WIDTH_STEPS: [f32; 13] = [
    1.0, 1.25, 1.5, 1.75, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0,
];

/// Tabs of the settings window.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SettingsTab {
    #[default]
    Keyboard,
    Pen,
    Controls,
}

/// Which color popover is open
#[derive(Clone, Copy, PartialEq)]
pub enum ColorPopover {
    None,
    StrokeFull, // Full color grid for stroke
    FillFull,   // Full color grid for fill
    BgFull,     // Full color grid for background
}

/// Peer info for UI display
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer identifier
    pub peer_id: String,
    /// User name (if set)
    pub name: Option<String>,
    /// User color (CSS color string)
    pub color: String,
    /// Whether cursor is currently visible
    pub has_cursor: bool,
}

/// UI state and actions.
pub struct UiState {
    /// Currently selected tool (mirrored from canvas).
    pub current_tool: ToolKind,
    /// Current stroke color for new shapes.
    pub stroke_color: Color32,
    /// Current fill color for new shapes (None = no fill).
    pub fill_color: Option<Color32>,
    /// Current stroke width for new shapes.
    pub stroke_width: f32,
    /// Number of selected shapes.
    pub selection_count: usize,
    /// Whether the hamburger menu is open.
    pub menu_open: bool,
    /// Which color popover is currently open.
    pub color_popover: ColorPopover,
    /// Current grid style.
    pub grid_style: GridStyle,
    /// Proportions of the handwriting and calligraphy guides.
    pub writing_guides: WritingGuides,
    /// Tools and colours shown next to the pointer, when open.
    pub quick_bars: Option<QuickBars>,
    /// Practice sheet submenu open in the main menu.
    pub practice_submenu: Option<drafftink_core::practice::SheetKind>,
    /// Current zoom level (1.0 = 100%).
    pub zoom_level: f64,
    /// Whether grid snapping is enabled.
    pub grid_snap_enabled: bool,
    /// Whether smart guides are enabled.
    pub smart_snap_enabled: bool,
    /// Whether angle snapping is enabled (for lines/arrows only).
    pub angle_snap_enabled: bool,
    /// Export scale factor (1 = 1x, 2 = 2x, 3 = 3x).
    pub export_scale: u8,
    /// Current sloppiness level for new shapes.
    pub sloppiness: drafftink_core::shapes::Sloppiness,
    /// Current fill pattern for new shapes.
    pub fill_pattern: FillPattern,
    /// Current corner radius for new rectangles.
    pub corner_radius: f32,
    /// Current path style for new lines/arrows (0=Direct, 1=Flowing, 2=Angular).
    pub path_style: u8,
    // Collaboration state
    /// WebSocket connection state.
    pub connection_state: ConnectionState,
    /// Current room ID (if connected).
    pub current_room: Option<String>,
    /// Number of peers in the room.
    pub peer_count: usize,
    /// Server URL for collaboration.
    pub server_url: String,
    /// Room ID input for joining.
    pub room_input: String,
    /// Connected peers for presence display.
    pub peers: Vec<PeerInfo>,
    /// Local user's display name.
    pub user_name: String,
    /// Local user's color (hex string like "#6366f1").
    pub user_color: String,
    /// Canvas background color.
    pub bg_color: Color32,
    /// Whether the collaboration modal is open.
    pub collab_modal_open: bool,
    /// Whether the keyboard shortcuts modal is open.
    /// Whether the settings window is open, and its selected tab.
    pub settings_open: bool,
    pub settings_tab: SettingsTab,
    /// Keyboard shortcuts (loaded and saved by the app).
    pub keybindings: Keybindings,
    /// Command waiting for a key press to bind to it.
    pub key_capture: Option<Command>,
    /// Message about the last rebinding (e.g. a key moved from another command).
    pub key_capture_note: Option<String>,
    /// Filter text for the keyboard shortcuts list.
    pub key_filter: String,
    /// Pen tablet settings (loaded and saved by the app).
    pub tablet_settings: TabletSettings,
    /// Connected tablets and pads.
    pub tablet_devices: Devices,
    /// Whether the tablet settings currently apply.
    pub tablet_active: bool,
    /// The pen last seen near the tablet.
    pub tablet_pen: Option<PenInfo>,
    /// Last pressed tablet button, to help identify ExpressKeys.
    pub tablet_last_pad_button: Option<u32>,
    /// Pad dials read directly from the device (when the compositor doesn't
    /// deliver them).
    pub tablet_evdev_dials: usize,
    /// Reading a tablet pad's dials was refused for lack of permission.
    pub tablet_evdev_denied: bool,
    /// Whether the save dialog is open.
    pub save_dialog_open: bool,
    /// Whether the open dialog is open.
    pub open_dialog_open: bool,
    /// Whether the open recent dialog is open.
    pub open_recent_dialog_open: bool,
    /// Input for save document name.
    pub save_name_input: String,
    /// List of recent document names.
    pub recent_documents: Vec<String>,
    /// Selected document from recent list.
    pub selected_recent_document: Option<String>,
    /// Clipboard for copied/cut shapes (JSON serialized).
    pub clipboard_shapes: Option<String>,
    /// Last picked stroke color from color grid.
    pub last_picked_stroke: Option<Color32>,
    /// Last picked fill color from color grid.
    pub last_picked_fill: Option<Color32>,
    /// Math editor state: (shape_id, latex_input)
    pub math_editor: Option<(ShapeId, String)>,
    /// Names of the open tabs, in order (synced from the app each frame).
    pub tab_names: Vec<String>,
    /// Index of the active tab within `tab_names`.
    pub active_tab: usize,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            current_tool: ToolKind::Select,
            stroke_color: TAILWIND_COLORS[11].shades[6], // Indigo 500
            fill_color: None,
            stroke_width: 2.0,
            selection_count: 0,
            menu_open: false,
            color_popover: ColorPopover::None,
            grid_style: GridStyle::default(),
            writing_guides: WritingGuides::default(),
            quick_bars: None,
            practice_submenu: None,
            zoom_level: drafftink_core::camera::BASE_ZOOM,
            grid_snap_enabled: false,
            smart_snap_enabled: false,
            angle_snap_enabled: false,
            export_scale: 2, // Default to 2x for good quality
            sloppiness: drafftink_core::shapes::Sloppiness::Artist,
            fill_pattern: FillPattern::Solid,
            corner_radius: 0.0, // Sharp corners by default
            path_style: 0,      // Direct by default
            // Collaboration defaults
            connection_state: ConnectionState::Disconnected,
            current_room: None,
            peer_count: 0,
            server_url: "/ws".to_string(),
            room_input: String::new(),
            peers: Vec::new(),
            user_name: String::new(),
            user_color: "#6366f1".to_string(), // Indigo
            bg_color: Color32::WHITE,
            collab_modal_open: false,
            settings_open: false,
            settings_tab: SettingsTab::default(),
            keybindings: Keybindings::default(),
            key_capture: None,
            key_capture_note: None,
            key_filter: String::new(),
            tablet_settings: TabletSettings::default(),
            tablet_devices: Devices::default(),
            tablet_active: false,
            tablet_pen: None,
            tablet_last_pad_button: None,
            tablet_evdev_dials: 0,
            tablet_evdev_denied: false,
            save_dialog_open: false,
            open_dialog_open: false,
            open_recent_dialog_open: false,
            save_name_input: String::new(),
            recent_documents: Vec::new(),
            selected_recent_document: None,
            clipboard_shapes: None,
            last_picked_stroke: None,
            last_picked_fill: None,
            math_editor: None,
            tab_names: Vec::new(),
            active_tab: 0,
        }
    }
}

impl UiState {
    /// Update UI state from a shape's style.
    pub fn update_from_style(&mut self, style: &ShapeStyle) {
        let sc = style.stroke_color;
        self.stroke_color = Color32::from_rgba_unmultiplied(sc.r, sc.g, sc.b, sc.a);
        self.stroke_width = style.stroke_width as f32;
        self.fill_color = style
            .fill_color
            .map(|fc| Color32::from_rgba_unmultiplied(fc.r, fc.g, fc.b, fc.a));
        self.fill_pattern = style.fill_pattern;
        self.sloppiness = style.sloppiness;
    }

    /// Convert current UI style to ShapeStyle.
    /// Generates a new random seed for each shape created.
    pub fn to_shape_style(&self) -> ShapeStyle {
        use drafftink_core::shapes::SerializableColor;
        ShapeStyle {
            stroke_color: SerializableColor::new(
                self.stroke_color.r(),
                self.stroke_color.g(),
                self.stroke_color.b(),
                self.stroke_color.a(),
            ),
            stroke_width: self.stroke_width as f64,
            fill_color: self
                .fill_color
                .map(|c| SerializableColor::new(c.r(), c.g(), c.b(), c.a())),
            fill_pattern: self.fill_pattern,
            sloppiness: self.sloppiness,
            ..ShapeStyle::default() // Generates a new random seed
        }
    }
}

/// Actions that can be triggered by the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum UiAction {
    /// Change the current tool.
    SetTool(ToolKind),
    /// Change stroke color.
    SetStrokeColor(Color32),
    /// Change fill color.
    SetFillColor(Option<Color32>),
    /// Change stroke width.
    SetStrokeWidth(f32),
    /// Save document with current name to local storage.
    SaveLocal,
    /// Show save dialog to rename and save document locally.
    SaveLocalAs,
    /// Show open dialog with dropdown list.
    ShowOpenDialog,
    /// Show open recent dialog with all recent documents.
    ShowOpenRecentDialog,
    /// Save document with given name to local storage.
    SaveLocalWithName(String),
    /// Load document by name from local storage.
    LoadLocal(String),
    /// Save document (to local storage on WASM, file dialog on native).
    SaveDocument,
    /// Load document (from local storage on WASM, file dialog on native).
    LoadDocument,
    /// Download document as JSON file (WASM only, same as SaveDocument on native).
    DownloadDocument,
    /// Upload document from JSON file (WASM only, same as LoadDocument on native).
    UploadDocument,
    /// Import a Mermaid diagram from the clipboard as native, editable shapes.
    ImportMermaidFromClipboard,
    /// Switch to the tab at the given index.
    SwitchTab(usize),
    /// Close the tab at the given index.
    CloseTab(usize),
    /// Load an Excalidraw library (`.excalidrawlib`) into a new tab.
    LoadLibrary,
    /// Export document as PNG file.
    ExportPng,
    /// Copy selection to clipboard as PNG.
    CopyPng,
    /// Toggle grid style (cycles through styles).
    ToggleGrid,
    /// Show a specific grid style.
    SetGridStyle(GridStyle),
    /// Brush for the pen tool and selected pen strokes.
    SetBrush(Brush),
    /// Broad nib angle (degrees) for the pen tool and selected pen strokes.
    SetNibAngle(f64),
    /// Zoom in.
    ZoomIn,
    /// Zoom out.
    ZoomOut,
    /// Reset zoom to 100%.
    ZoomReset,
    /// Center canvas at origin (0,0).
    CenterCanvas,
    /// Toggle grid snap.
    ToggleGridSnap,
    /// Toggle smart guides.
    ToggleSmartSnap,
    /// Toggle angle snap (for lines/arrows).
    ToggleAngleSnap,
    /// Set font size for text shapes.
    SetFontSize(f32),
    /// Set font size for math shapes.
    SetMathFontSize(f32),
    /// Set font family for text shapes.
    SetFontFamily(u8), // 0 = GelPen, 1 = NotoSans, 2 = GelPenSerif, 3 = VanillaExtract
    /// Set font weight for text shapes.
    SetFontWeight(u8), // 0 = Light, 1 = Regular, 2 = Heavy
    /// Set corner radius for rectangle shapes.
    SetCornerRadius(f32),
    /// Set export scale (1, 2, or 3).
    SetExportScale(u8),
    /// Clear document (remove all shapes).
    ClearDocument,
    /// Show intro/welcome screen.
    ShowIntro,
    /// Open a handwriting or calligraphy practice sheet in its own tab.
    ShowPractice(drafftink_core::practice::PracticeSheet),
    /// Set sloppiness level for selected shapes.
    SetSloppiness(u8), // 0 = Architect, 1 = Artist, 2 = Cartoonist
    /// Set fill pattern for selected shapes.
    SetFillPattern(u8), // 0 = Solid, 1 = Hachure, 2 = ZigZag, 3 = CrossHatch, 4 = Dots, 5 = Dashed, 6 = ZigZagLine
    /// Set path style for selected lines/arrows.
    SetPathStyle(u8), // 0 = Direct, 1 = Flowing, 2 = Angular
    /// Set stroke style for selected lines/arrows.
    SetStrokeStyle(u8), // 0 = Solid, 1 = Dashed, 2 = Dotted
    /// Undo the last action.
    Undo,
    /// Redo the last undone action.
    Redo,
    /// Connect to collaboration server.
    Connect(String), // server URL
    /// Disconnect from collaboration server.
    Disconnect,
    /// Join a collaboration room.
    JoinRoom(String), // room ID
    /// Leave current collaboration room.
    LeaveRoom,
    /// Set user display name.
    SetUserName(String),
    /// Set user color (hex string).
    SetUserColor(String),
    /// Set canvas background color.
    SetBgColor(Color32),
    /// Bring selected shapes to front (topmost).
    BringToFront,
    /// Send selected shapes to back (bottommost).
    SendToBack,
    /// Move selected shapes one layer forward.
    BringForward,
    /// Move selected shapes one layer backward.
    SendBackward,
    /// Zoom to fit all elements (or selection if any).
    ZoomToFit,
    /// Duplicate selected shapes.
    Duplicate,
    /// Copy selected shapes to clipboard.
    CopyShapes,
    /// Cut selected shapes to clipboard.
    CutShapes,
    /// Paste shapes from clipboard.
    PasteShapes,
    /// Align selected shapes to top.
    AlignTop,
    /// Align selected shapes to bottom.
    AlignBottom,
    /// Align selected shapes to left.
    AlignLeft,
    /// Align selected shapes to right.
    AlignRight,
    /// Align selected shapes to horizontal center.
    AlignCenterH,
    /// Align selected shapes to vertical center.
    AlignCenterV,
    /// Show keyboard shortcuts help.
    ShowShortcuts,
    /// Toggle calligraphy mode for freehand tool.
    ToggleCalligraphy,
    /// Toggle pressure simulation for freehand tool.
    TogglePressureSimulation,
    /// Flip selected shapes horizontally.
    FlipHorizontal,
    /// Flip selected shapes vertically.
    FlipVertical,
    /// Set opacity for selected shapes.
    SetOpacity(f32),
    /// Update math shape LaTeX.
    UpdateMathLatex(ShapeId, String),
}

/// Tools and colours shown next to the pointer ([`Command::QuickToolbars`]).
#[derive(Debug, Clone, Copy)]
pub struct QuickBars {
    /// Pointer position when opened, in screen points (set on the first frame).
    pub anchor: Option<Pos2>,
    /// When it opened, in egui time (seconds).
    pub opened_at: f64,
    /// Item under the pointer: picked when a held hotkey is released.
    pub hovered: Option<QuickItem>,
}

impl QuickBars {
    /// Holding the hotkey at least this long (seconds) means "pick on
    /// release"; a shorter tap leaves the toolbars open for a click.
    pub const HOLD: f64 = 0.3;

    pub fn new(now: f64) -> Self {
        Self {
            anchor: None,
            opened_at: now,
            hovered: None,
        }
    }
}

/// Something picked from the quick toolbars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuickItem {
    Tool(ToolKind),
    Color(Color32),
}

impl QuickItem {
    pub fn action(self) -> UiAction {
        match self {
            QuickItem::Tool(tool) => UiAction::SetTool(tool),
            QuickItem::Color(color) => UiAction::SetStrokeColor(color),
        }
    }
}

/// Tool definitions with SVG icons
struct Tool {
    kind: ToolKind,
    label: &'static str,
    icon: ImageSource<'static>,
}

fn get_tools() -> Vec<Tool> {
    vec![
        Tool {
            kind: ToolKind::Select,
            label: "Select",
            icon: include_image!("../assets/select.svg"),
        },
        Tool {
            kind: ToolKind::Pan,
            label: "Pan",
            icon: include_image!("../assets/pan.svg"),
        },
        Tool {
            kind: ToolKind::Rectangle,
            label: "Rectangle",
            icon: include_image!("../assets/rectangle.svg"),
        },
        Tool {
            kind: ToolKind::Ellipse,
            label: "Ellipse",
            icon: include_image!("../assets/ellipse.svg"),
        },
        Tool {
            kind: ToolKind::Arrow,
            label: "Arrow",
            icon: include_image!("../assets/arrow.svg"),
        },
        Tool {
            kind: ToolKind::Line,
            label: "Line",
            icon: include_image!("../assets/line.svg"),
        },
        Tool {
            kind: ToolKind::Freehand,
            label: "Draw",
            icon: include_image!("../assets/freehand.svg"),
        },
        Tool {
            kind: ToolKind::Highlighter,
            label: "Highlighter",
            icon: include_image!("../assets/highlighter.svg"),
        },
        Tool {
            kind: ToolKind::Eraser,
            label: "Eraser",
            icon: include_image!("../assets/eraser.svg"),
        },
        Tool {
            kind: ToolKind::Text,
            label: "Text",
            icon: include_image!("../assets/text.svg"),
        },
        Tool {
            kind: ToolKind::Math,
            label: "Math",
            icon: include_image!("../assets/math.svg"),
        },
        Tool {
            kind: ToolKind::LaserPointer,
            label: "Laser",
            icon: include_image!("../assets/laser.svg"),
        },
    ]
}

/// Render all UI and return any triggered action.
pub fn render_ui(
    ctx: &Context,
    ui_state: &mut UiState,
    selected_props: &SelectedShapeProps,
) -> Option<UiAction> {
    egui_extras::install_image_loaders(ctx);

    let quick_action = render_quick_bars(ctx, ui_state);
    let toolbar_action = render_toolbar(ctx, ui_state);
    let properties_action = render_properties_panel(ctx, ui_state);
    let file_action = render_file_menu(ctx, ui_state);
    let bottom_action = render_bottom_toolbar(ctx, ui_state);
    let right_panel_action = render_right_panel(ctx, selected_props);
    let math_action = render_math_editor(ctx, ui_state);
    let tab_action = render_tab_bar(ctx, ui_state);

    // Render presence panel (no actions returned)
    render_presence_panel(ctx, ui_state);

    // Return the first action (toolbar takes precedence)
    quick_action
        .or(toolbar_action)
        .or(properties_action)
        .or(file_action)
        .or(bottom_action)
        .or(right_panel_action)
        .or(math_action)
        .or(tab_action)
}

/// Render the tab strip (top-center) with one button per open canvas plus a
/// control to load an Excalidraw library as a new tab. Icons in a library tab
/// are browsed on their own canvas and copied into a working tab, so no
/// separate thumbnail rendering is needed.
fn render_tab_bar(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let tabs = ui_state.tab_names.clone();
    let active = ui_state.active_tab;
    let closable = tabs.len() > 1;

    egui::Area::new(egui::Id::new("tab_bar"))
        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -12.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                    for (i, name) in tabs.iter().enumerate() {
                        let selected = i == active;
                        let label = egui::RichText::new(name).size(12.0).color(if selected {
                            Color32::WHITE
                        } else {
                            Color32::from_gray(90)
                        });
                        let btn = egui::Button::new(label)
                            .fill(if selected {
                                Color32::from_rgb(59, 130, 246)
                            } else {
                                Color32::TRANSPARENT
                            })
                            .corner_radius(egui::CornerRadius::same(4));
                        if ui.add(btn).clicked() {
                            action = Some(UiAction::SwitchTab(i));
                        }
                        if closable {
                            let close = egui::Button::new(
                                egui::RichText::new("×")
                                    .size(13.0)
                                    .color(Color32::from_gray(120)),
                            )
                            .fill(Color32::TRANSPARENT)
                            .corner_radius(egui::CornerRadius::same(4));
                            if ui.add(close).on_hover_text("Close tab").clicked() {
                                action = Some(UiAction::CloseTab(i));
                            }
                        }
                    }

                    ui.add_space(6.0);
                    let load = egui::Button::new(
                        egui::RichText::new("+ Library")
                            .size(12.0)
                            .color(Color32::from_gray(90)),
                    )
                    .fill(Color32::TRANSPARENT)
                    .corner_radius(egui::CornerRadius::same(4));
                    if ui
                        .add(load)
                        .on_hover_text("Load an Excalidraw library (.excalidrawlib) as a new tab")
                        .clicked()
                    {
                        action = Some(UiAction::LoadLibrary);
                    }
                });
            });
        });

    action
}

/// Render the toolbar and return any triggered action.
fn render_toolbar(ctx: &Context, ui_state: &UiState) -> Option<UiAction> {
    let mut action = None;
    let tools = get_tools();

    egui::Area::new(egui::Id::new("toolbar"))
        .anchor(Align2::LEFT_CENTER, Vec2::new(12.0, 0.0))
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                    for tool in &tools {
                        let is_selected = ui_state.current_tool == tool.kind;
                        let shortcut = ui_state
                            .keybindings
                            .keys(Command::Tool(tool.kind))
                            .iter()
                            .map(|k| k.label())
                            .collect::<Vec<_>>()
                            .join(" / ");
                        if IconButton::new(tool.icon.clone(), tool.label)
                            .shortcut(&shortcut)
                            .selected(is_selected)
                            .tool()
                            .show(ui)
                        {
                            action = Some(UiAction::SetTool(tool.kind));
                        }
                    }
                });
            });
        });

    action
}

/// Render the quick toolbars around the pointer: tools in a column to its
/// left and colours in rows above it, meeting at the pointer's upper-left so
/// everything is a short move away. The items most used sit nearest the
/// pointer (the pen tools and the quick colours).
fn render_quick_bars(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut bars = ui_state.quick_bars?;
    let now = ctx.input(|i| i.time);
    let anchor = *bars.anchor.get_or_insert_with(|| {
        ctx.input(|i| i.pointer.hover_pos())
            .unwrap_or_else(|| ctx.content_rect().center())
    });

    // Ease in over 120 ms: fade in while sliding out from the pointer.
    let t = ((now - bars.opened_at) / 0.12).clamp(0.0, 1.0) as f32;
    if t < 1.0 {
        ctx.request_repaint();
    }
    let ease = 1.0 - (1.0 - t).powi(3);
    let slide = (1.0 - ease) * 10.0;
    const GAP: f32 = 14.0;

    let pointer = ctx.input(|i| i.pointer.hover_pos());
    let mut hovered = None;
    let mut action = None;

    // Tools: two columns, the drawing tools in the column next to the pointer.
    let tools = get_tools();
    let near = [
        ToolKind::Freehand,
        ToolKind::Eraser,
        ToolKind::Select,
        ToolKind::Highlighter,
        ToolKind::Text,
        ToolKind::LaserPointer,
    ];
    let (inner, outer): (Vec<&Tool>, Vec<&Tool>) =
        tools.iter().partition(|tool| near.contains(&tool.kind));
    egui::Area::new(egui::Id::new("quick_tools"))
        .order(egui::Order::Foreground)
        .pivot(Align2::RIGHT_CENTER)
        .fixed_pos(anchor + Vec2::new(-GAP + slide, 0.0))
        .constrain(true)
        .show(ctx, |ui| {
            ui.set_opacity(ease);
            panel_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 2.0);
                    for column in [&outer, &inner] {
                        ui.vertical(|ui| {
                            for tool in column {
                                let tooltip = ui_state
                                    .keybindings
                                    .with_hint(tool.label, Command::Tool(tool.kind));
                                let response = ui.scope(|ui| {
                                    IconButton::new(tool.icon.clone(), &tooltip)
                                        .selected(ui_state.current_tool == tool.kind)
                                        .tool()
                                        .show(ui)
                                });
                                if response.inner {
                                    action = Some(QuickItem::Tool(tool.kind));
                                }
                                if pointer.is_some_and(|p| response.response.rect.contains(p)) {
                                    hovered = Some(QuickItem::Tool(tool.kind));
                                }
                            }
                        });
                    }
                });
            });
        });

    // Colours: the quick colours plus black and white in the row nearest
    // the pointer, the rest of the palette above.
    let quick: Vec<(Color32, &str)> = QUICK_COLORS
        .iter()
        .map(|&i| (TAILWIND_COLORS[i].shades[6], TAILWIND_COLORS[i].name))
        .chain([(Color32::BLACK, "Black"), (Color32::WHITE, "White")])
        .collect();
    let others: Vec<(Color32, &str)> = (0..TAILWIND_COLORS.len())
        .filter(|i| !QUICK_COLORS.contains(i))
        .map(|i| (TAILWIND_COLORS[i].shades[6], TAILWIND_COLORS[i].name))
        .collect();
    let mut rows: Vec<&[(Color32, &str)]> = others.chunks(quick.len()).collect();
    rows.push(&quick);
    egui::Area::new(egui::Id::new("quick_colors"))
        .order(egui::Order::Foreground)
        .pivot(Align2::LEFT_BOTTOM)
        .fixed_pos(anchor + Vec2::new(-GAP + 2.0, -GAP + slide))
        .constrain(true)
        .show(ctx, |ui| {
            ui.set_opacity(ease);
            panel_frame().show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                for row in &rows {
                    ui.horizontal(|ui| {
                        for &(color, name) in *row {
                            let (clicked, rect) = ColorSwatch::new(color, name)
                                .style(drafftink_widgets::ColorSwatchStyle::large())
                                .selected(ui_state.stroke_color == color)
                                .show(ui);
                            if clicked {
                                action = Some(QuickItem::Color(color));
                            }
                            if pointer.is_some_and(|p| rect.contains(p)) {
                                hovered = Some(QuickItem::Color(color));
                            }
                        }
                    });
                }
            });
        });

    bars.hovered = hovered;
    ui_state.quick_bars = if action.is_some() { None } else { Some(bars) };
    action.map(QuickItem::action)
}

/// Render the bottom-left toolbar with grid toggle and zoom controls.
fn render_bottom_toolbar(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let mut bg_color_rect = Rect::NOTHING;

    // Get screen rect to position at bottom
    #[allow(deprecated)]
    let screen_rect = ctx.input(|i| i.content_rect());
    let toolbar_height = 36.0;
    let margin = 12.0;
    let bottom_y = screen_rect.max.y - margin - toolbar_height;

    egui::Area::new(egui::Id::new("bottom_toolbar"))
        .fixed_pos(Pos2::new(margin, bottom_y.max(margin)))
        .interactable(true)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            // Light panel frame
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(250, 250, 252, 250))
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, Color32::from_gray(220)))
                .shadow(egui::epaint::Shadow {
                    spread: 0,
                    blur: 6,
                    offset: [0, 2],
                    color: Color32::from_black_alpha(10),
                })
                .inner_margin(Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                        let text_color = Color32::from_gray(80);

                        // Undo button
                        if IconButton::new(
                            include_image!("../assets/undo.svg"),
                            &ui_state.keybindings.with_hint("Undo", Command::Undo),
                        )
                        .small()
                        .show(ui)
                        {
                            action = Some(UiAction::Undo);
                        }

                        ui.add_space(2.0);

                        // Redo button
                        if IconButton::new(
                            include_image!("../assets/redo.svg"),
                            &ui_state.keybindings.with_hint("Redo", Command::Redo),
                        )
                        .small()
                        .show(ui)
                        {
                            action = Some(UiAction::Redo);
                        }

                        // Separator after undo/redo
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("|")
                                .size(14.0)
                                .color(Color32::from_gray(200)),
                        );
                        ui.add_space(8.0);

                        // Grid toggle button - draw grid pattern icon
                        // Grid style: click for a menu of styles (and the
                        // writing guide proportions).
                        let grid_tooltip =
                            format!("Grid: {} (click to choose)", ui_state.grid_style.name());
                        let grid_response =
                            grid_style_button(ui, ui_state.grid_style, &grid_tooltip);
                        egui::Popup::menu(&grid_response)
                            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                            .show(|ui| {
                                ui.set_min_width(240.0);
                                for style in GridStyle::ALL {
                                    let label = ui_state
                                        .keybindings
                                        .with_hint(style.name(), Command::Grid(style));
                                    if ui
                                        .selectable_label(ui_state.grid_style == style, label)
                                        .clicked()
                                    {
                                        action = Some(UiAction::SetGridStyle(style));
                                    }
                                }
                                if ui_state.grid_style.is_writing_guide() {
                                    ui.separator();
                                    writing_guides_editor(
                                        ui,
                                        &mut ui_state.writing_guides,
                                        ui_state.stroke_width,
                                    );
                                }
                            });

                        ui.add_space(4.0);

                        // Background color button - opens Tailwind color picker
                        let (clicked, rect) =
                            color_swatch_current(ui, ui_state.bg_color, "Background color");
                        bg_color_rect = rect;
                        if clicked {
                            ui_state.color_popover =
                                if ui_state.color_popover == ColorPopover::BgFull {
                                    ColorPopover::None
                                } else {
                                    ColorPopover::BgFull
                                };
                        }

                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("|")
                                .size(14.0)
                                .color(Color32::from_gray(200)),
                        );
                        ui.add_space(8.0);

                        // Zoom out button
                        let minus_response = ui.add(
                            egui::Label::new(
                                egui::RichText::new("\u{2212}") // − minus sign
                                    .size(16.0)
                                    .color(text_color),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if minus_response.clicked() {
                            action = Some(UiAction::ZoomOut);
                        }
                        minus_response.clone().on_hover_text(
                            ui_state.keybindings.with_hint("Zoom out", Command::ZoomOut),
                        );
                        minus_response.on_hover_cursor(egui::CursorIcon::PointingHand);

                        ui.add_space(12.0);

                        // Current zoom level (clickable to reset)
                        // Display zoom relative to BASE_ZOOM (so BASE_ZOOM = 100%)
                        let zoom_pct = (ui_state.zoom_level / drafftink_core::camera::BASE_ZOOM
                            * 100.0)
                            .round() as i32;
                        let zoom_response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("{}%", zoom_pct))
                                    .size(13.0)
                                    .color(text_color),
                            )
                            .sense(egui::Sense::click()),
                        );
                        if zoom_response.clicked() {
                            action = Some(UiAction::ZoomReset);
                        }
                        zoom_response.clone().on_hover_text(
                            ui_state
                                .keybindings
                                .with_hint("Reset to 100%", Command::ZoomReset),
                        );
                        zoom_response.on_hover_cursor(egui::CursorIcon::PointingHand);

                        ui.add_space(12.0);

                        // Zoom in button
                        let plus_response = ui.add(
                            egui::Label::new(egui::RichText::new("+").size(16.0).color(text_color))
                                .sense(egui::Sense::click()),
                        );
                        if plus_response.clicked() {
                            action = Some(UiAction::ZoomIn);
                        }
                        plus_response.clone().on_hover_text(
                            ui_state.keybindings.with_hint("Zoom in", Command::ZoomIn),
                        );
                        plus_response.on_hover_cursor(egui::CursorIcon::PointingHand);

                        ui.add_space(8.0);

                        // Center button - circle with dot icon
                        if IconButton::new(
                            include_image!("../assets/center.svg"),
                            &ui_state
                                .keybindings
                                .with_hint("Center canvas at origin", Command::CenterCanvas),
                        )
                        .small()
                        .show(ui)
                        {
                            action = Some(UiAction::CenterCanvas);
                        }

                        ui.add_space(4.0);

                        // Zoom to fit button
                        let fit_tooltip = if ui_state.selection_count > 0 {
                            "Zoom to fit selection"
                        } else {
                            "Zoom to fit all elements"
                        };
                        let fit_tooltip = ui_state
                            .keybindings
                            .with_hint(fit_tooltip, Command::ZoomToFit);
                        if IconButton::new(include_image!("../assets/zoom-fit.svg"), &fit_tooltip)
                            .small()
                            .show(ui)
                        {
                            action = Some(UiAction::ZoomToFit);
                        }

                        // Separator before snap buttons
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("|")
                                .size(14.0)
                                .color(Color32::from_gray(200)),
                        );
                        ui.add_space(8.0);

                        // Grid snap button
                        let grid_snap_tooltip = if ui_state.grid_snap_enabled {
                            "Grid Snap: On"
                        } else {
                            "Grid Snap: Off"
                        };
                        if IconButton::new(
                            include_image!("../assets/snap-grid.svg"),
                            grid_snap_tooltip,
                        )
                        .small()
                        .selected(ui_state.grid_snap_enabled)
                        .show(ui)
                        {
                            action = Some(UiAction::ToggleGridSnap);
                        }

                        ui.add_space(4.0);

                        // Smart guides button
                        let smart_snap_tooltip = if ui_state.smart_snap_enabled {
                            "Smart Guides: On"
                        } else {
                            "Smart Guides: Off"
                        };
                        if IconButton::new(
                            include_image!("../assets/snap-shapes.svg"),
                            smart_snap_tooltip,
                        )
                        .small()
                        .selected(ui_state.smart_snap_enabled)
                        .show(ui)
                        {
                            action = Some(UiAction::ToggleSmartSnap);
                        }

                        ui.add_space(4.0);

                        // Angle snap button
                        let angle_snap_tooltip = if ui_state.angle_snap_enabled {
                            "Angle Snap: On (15°)"
                        } else {
                            "Angle Snap: Off"
                        };
                        if IconButton::new(
                            include_image!("../assets/angle.svg"),
                            angle_snap_tooltip,
                        )
                        .small()
                        .selected(ui_state.angle_snap_enabled)
                        .show(ui)
                        {
                            action = Some(UiAction::ToggleAngleSnap);
                        }
                    });
                });
        });

    // Render background color popover if open
    if ui_state.color_popover == ColorPopover::BgFull {
        // Position popover above the button (since we're at the bottom of the screen)
        if let Some(selected_color) = ColorGrid::new(ui_state.bg_color, "Background Color")
            .above()
            .show(ctx, bg_color_rect)
        {
            action = Some(UiAction::SetBgColor(selected_color));
            ui_state.color_popover = ColorPopover::None;
        }
    }

    action
}

// Quick color indices into TAILWIND_COLORS: Blue, Red, Emerald, Amber, Purple, Slate
// Using 500-level for strokes (index 5), 100-level for fills (index 1)
const QUICK_COLORS: &[usize] = &[10, 0, 6, 2, 13, 17]; // Blue, Red, Emerald, Amber, Purple, Slate

/// The UI action for a command that is also a UI control, so shortcuts and
/// tablet buttons behave exactly like clicking it. `None` for commands the
/// app runs itself.
pub fn command_action(command: Command) -> Option<QueuedAction> {
    let ui = |action| Some(QueuedAction::Ui(action));
    match command {
        Command::StrokeColor(i) => QUICK_COLORS
            .get(i as usize)
            .and_then(|&family| ui(UiAction::SetStrokeColor(TAILWIND_COLORS[family].shades[6]))),
        Command::StrokeWidth(i) => STROKE_WIDTH_PRESETS
            .get(i as usize)
            .and_then(|&(width, _)| ui(UiAction::SetStrokeWidth(width))),
        Command::NextColor => Some(QueuedAction::CycleColor(1)),
        Command::PreviousColor => Some(QueuedAction::CycleColor(-1)),
        Command::Thicker => Some(QueuedAction::StepWidth(1)),
        Command::Thinner => Some(QueuedAction::StepWidth(-1)),
        Command::ZoomIn => ui(UiAction::ZoomIn),
        Command::ZoomOut => ui(UiAction::ZoomOut),
        Command::ZoomReset => ui(UiAction::ZoomReset),
        Command::ZoomToFit => ui(UiAction::ZoomToFit),
        Command::CenterCanvas => ui(UiAction::CenterCanvas),
        Command::ToggleGrid => ui(UiAction::ToggleGrid),
        Command::Grid(style) => ui(UiAction::SetGridStyle(style)),
        Command::ToggleGridSnap => ui(UiAction::ToggleGridSnap),
        Command::ToggleSmartSnap => ui(UiAction::ToggleSmartSnap),
        Command::ToggleAngleSnap => ui(UiAction::ToggleAngleSnap),
        Command::BringToFront => ui(UiAction::BringToFront),
        Command::BringForward => ui(UiAction::BringForward),
        Command::SendBackward => ui(UiAction::SendBackward),
        Command::SendToBack => ui(UiAction::SendToBack),
        Command::FlipHorizontal => ui(UiAction::FlipHorizontal),
        Command::FlipVertical => ui(UiAction::FlipVertical),
        Command::AlignLeft => ui(UiAction::AlignLeft),
        Command::AlignCenterH => ui(UiAction::AlignCenterH),
        Command::AlignRight => ui(UiAction::AlignRight),
        Command::AlignTop => ui(UiAction::AlignTop),
        Command::AlignCenterV => ui(UiAction::AlignCenterV),
        Command::AlignBottom => ui(UiAction::AlignBottom),
        _ => None,
    }
}

/// Tooltip for the colour wheel, mentioning the colour cycling keys.
fn pick_color_tooltip(keybindings: &Keybindings) -> String {
    let previous = keybindings.hint(Command::PreviousColor);
    let next = keybindings.hint(Command::NextColor);
    match (previous.is_empty(), next.is_empty()) {
        (false, false) => format!("Pick color ({previous} / {next} to cycle)"),
        _ => "Pick color".to_string(),
    }
}

/// An action queued for the next UI pass. Relative changes are resolved
/// against the current state when applied, so several in a row add up.
#[derive(Debug, Clone, PartialEq)]
pub enum QueuedAction {
    Ui(UiAction),
    /// Step through the stroke colour palette.
    CycleColor(isize),
    /// Step through the stroke widths.
    StepWidth(isize),
}

/// Turn a queued action into a concrete UI action.
pub fn resolve_queued_action(action: QueuedAction, ui_state: &UiState) -> UiAction {
    match action {
        QueuedAction::Ui(action) => action,
        QueuedAction::CycleColor(step) => {
            UiAction::SetStrokeColor(cycle_stroke_color(ui_state.stroke_color, step))
        }
        QueuedAction::StepWidth(step) => {
            // Nearest step to the current width (by ratio), then step and clamp.
            let ratio = |w: f32| (w / ui_state.stroke_width.max(0.01)).ln().abs();
            let current = STROKE_WIDTH_STEPS
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| ratio(**a).total_cmp(&ratio(**b)))
                .map_or(0, |(i, _)| i);
            let next = (current as isize + step).clamp(0, STROKE_WIDTH_STEPS.len() as isize - 1);
            UiAction::SetStrokeWidth(STROKE_WIDTH_STEPS[next as usize])
        }
    }
}

/// The palette colour `step` places from `current` (500 level, as the swatches
/// use), wrapping around. A custom colour starts from either end.
fn cycle_stroke_color(current: Color32, step: isize) -> Color32 {
    let shade = |family: usize| TAILWIND_COLORS[family].shades[6];
    let len = TAILWIND_COLORS.len() as isize;
    let next = match (0..TAILWIND_COLORS.len()).find(|&i| shade(i) == current) {
        Some(i) => (i as isize + step).rem_euclid(len),
        None if step > 0 => 0,
        None => len - 1,
    };
    shade(next as usize)
}

/// Render the properties panel at the top.
fn render_properties_panel(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Track current color swatch position for popover
    let mut stroke_current_rect = Rect::NOTHING;
    let mut fill_current_rect = Rect::NOTHING;

    egui::Area::new(egui::Id::new("properties"))
        .anchor(Align2::CENTER_TOP, Vec2::new(0.0, 12.0))
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);

                    // Stroke color section
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(2.0, 4.0);
                        widgets_section_label(ui, "Stroke");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                            // Quick colors (500-level for strokes)
                            for (i, &idx) in QUICK_COLORS.iter().enumerate() {
                                let color = TAILWIND_COLORS[idx].shades[6]; // 500-level
                                let is_selected = ui_state.stroke_color == color;
                                let tooltip = ui_state.keybindings.with_hint(
                                    TAILWIND_COLORS[idx].name,
                                    Command::StrokeColor(i as u8),
                                );
                                if color_swatch_selectable(ui, color, &tooltip, is_selected) {
                                    action = Some(UiAction::SetStrokeColor(color));
                                    ui_state.color_popover = ColorPopover::None;
                                }
                            }

                            // Separator
                            ui.add_space(4.0);
                            widgets_vertical_separator(ui);
                            ui.add_space(4.0);

                            // Last picked color as quick-select (only if different from quick colors)
                            if let Some(last) = ui_state.last_picked_stroke {
                                let is_selected = ui_state.stroke_color == last;
                                if color_swatch_selectable(ui, last, "Last picked", is_selected) {
                                    action = Some(UiAction::SetStrokeColor(last));
                                }
                            }

                            // Color wheel (opens full picker)
                            let (clicked, rect) = color_swatch_current(
                                ui,
                                ui_state.stroke_color,
                                &pick_color_tooltip(&ui_state.keybindings),
                            );
                            stroke_current_rect = rect;
                            if clicked {
                                ui_state.color_popover =
                                    if ui_state.color_popover == ColorPopover::StrokeFull {
                                        ColorPopover::None
                                    } else {
                                        ColorPopover::StrokeFull
                                    };
                            }
                        });
                    });

                    panel_separator(ui);
                    ui.add_space(8.0);

                    // Fill color section
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(2.0, 4.0);
                        widgets_section_label(ui, "Fill");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                            // "None" option
                            let is_none_selected = ui_state.fill_color.is_none();
                            if fill_swatch(ui, None, "None", is_none_selected) {
                                action = Some(UiAction::SetFillColor(None));
                                ui_state.color_popover = ColorPopover::None;
                            }

                            // Quick colors (100-level for fills)
                            for &idx in QUICK_COLORS {
                                let color = TAILWIND_COLORS[idx].shades[1]; // 100-level
                                let is_selected = ui_state.fill_color == Some(color);
                                if color_swatch_selectable(
                                    ui,
                                    color,
                                    TAILWIND_COLORS[idx].name,
                                    is_selected,
                                ) {
                                    action = Some(UiAction::SetFillColor(Some(color)));
                                    ui_state.color_popover = ColorPopover::None;
                                }
                            }

                            // Separator
                            ui.add_space(4.0);
                            widgets_vertical_separator(ui);
                            ui.add_space(4.0);

                            // Last picked color as quick-select
                            if let Some(last) = ui_state.last_picked_fill {
                                let is_selected = ui_state.fill_color == Some(last);
                                if color_swatch_selectable(ui, last, "Last picked", is_selected) {
                                    action = Some(UiAction::SetFillColor(Some(last)));
                                }
                            }

                            // Color wheel (opens full picker)
                            let current_fill = ui_state.fill_color.unwrap_or(Color32::TRANSPARENT);
                            let (clicked, rect) =
                                color_swatch_current(ui, current_fill, "Pick color");
                            fill_current_rect = rect;
                            if clicked {
                                ui_state.color_popover =
                                    if ui_state.color_popover == ColorPopover::FillFull {
                                        ColorPopover::None
                                    } else {
                                        ColorPopover::FillFull
                                    };
                            }
                        });
                    });

                    panel_separator(ui);
                    ui.add_space(8.0);

                    // Stroke width section
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(2.0, 4.0);
                        widgets_section_label(ui, "Stroke width");
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                            for (i, &(width, name)) in STROKE_WIDTH_PRESETS.iter().enumerate() {
                                let is_selected = (ui_state.stroke_width - width).abs() < 0.01;
                                let tooltip = ui_state.keybindings.with_hint(
                                    &format!("{name} ({width} px)"),
                                    Command::StrokeWidth(i as u8),
                                );
                                if StrokeWidthButton::new(width, &tooltip, is_selected).show(ui) {
                                    action = Some(UiAction::SetStrokeWidth(width));
                                }
                            }
                        });
                        ui.horizontal(|ui| {
                            let mut width = ui_state.stroke_width;
                            let slider = egui::Slider::new(&mut width, STROKE_WIDTH_RANGE)
                                .logarithmic(true)
                                .show_value(false);
                            let response = ui
                                .add(slider)
                                .on_hover_text("Any width: drag, or scroll over it");
                            let scroll = if response.hovered() {
                                ui.input(|i| i.raw_scroll_delta.y)
                            } else {
                                0.0
                            };
                            if scroll != 0.0 {
                                width = (width * 1.1_f32.powf(scroll.signum()))
                                    .clamp(*STROKE_WIDTH_RANGE.start(), *STROKE_WIDTH_RANGE.end());
                            }
                            if response.changed() || scroll != 0.0 {
                                // Two decimals is finer than anyone can see.
                                width = (width * 100.0).round() / 100.0;
                                action = Some(UiAction::SetStrokeWidth(width));
                            }
                            ui.label(
                                egui::RichText::new(format!("{width} px"))
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                        });
                    });
                });
            });
        });

    // Render full color grid popover if open (stroke/fill only - bg is handled in bottom toolbar)
    match ui_state.color_popover {
        ColorPopover::StrokeFull => {
            if let Some(selected_color) = ColorGrid::new(ui_state.stroke_color, "Stroke Color")
                .below()
                .show(ctx, stroke_current_rect)
            {
                ui_state.last_picked_stroke = Some(selected_color);
                action = Some(UiAction::SetStrokeColor(selected_color));
                ui_state.color_popover = ColorPopover::None;
            }
        }
        ColorPopover::FillFull => {
            if let Some(selected_color) = ColorGrid::new(
                ui_state.fill_color.unwrap_or(Color32::TRANSPARENT),
                "Fill Color",
            )
            .below()
            .show(ctx, fill_current_rect)
            {
                ui_state.last_picked_fill = Some(selected_color);
                action = Some(UiAction::SetFillColor(Some(selected_color)));
                ui_state.color_popover = ColorPopover::None;
            }
        }
        ColorPopover::None | ColorPopover::BgFull => {}
    }

    action
}

// colors_match is now imported from drafftink_widgets

/// Color swatch circle that can show selection state with black inner offset - uses drafftink_widgets.
fn color_swatch_selectable(ui: &mut egui::Ui, color: Color32, name: &str, selected: bool) -> bool {
    let (clicked, _) = ColorSwatch::new(color, name).selected(selected).show(ui);
    clicked
}

/// Current color swatch with hue wheel background (color picker button) - uses drafftink_widgets.
fn color_swatch_current(ui: &mut egui::Ui, color: Color32, tooltip: &str) -> (bool, Rect) {
    ColorSwatchWithWheel::new(color, tooltip).show(ui)
}

// hue_to_rgb is now imported from drafftink_widgets

/// Grid style button - draws icon representing current grid style.
/// This is kept local because it draws a custom procedural icon.
fn grid_style_button(ui: &mut egui::Ui, style: GridStyle, tooltip: &str) -> egui::Response {
    let size = Vec2::new(24.0, 24.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let icon_color = if response.hovered() {
            Color32::from_gray(40)
        } else {
            Color32::from_gray(80)
        };

        let center = rect.center();
        let grid_size = 5.0; // Size of grid cell in icon

        match style {
            GridStyle::None => {
                // Draw empty square with slash
                let sq_size = 12.0;
                let sq = Rect::from_center_size(center, Vec2::splat(sq_size));
                ui.painter().rect_stroke(
                    sq,
                    CornerRadius::same(2),
                    Stroke::new(1.5, icon_color),
                    egui::StrokeKind::Inside,
                );
                ui.painter().line_segment(
                    [sq.left_bottom(), sq.right_top()],
                    Stroke::new(1.0, Color32::from_gray(160)),
                );
            }
            GridStyle::Lines => {
                // Draw 3x3 grid lines
                for i in 0..3 {
                    let offset = (i as f32 - 1.0) * grid_size;
                    // Vertical line
                    ui.painter().line_segment(
                        [
                            Pos2::new(center.x + offset, center.y - grid_size * 1.5),
                            Pos2::new(center.x + offset, center.y + grid_size * 1.5),
                        ],
                        Stroke::new(1.0, icon_color),
                    );
                    // Horizontal line
                    ui.painter().line_segment(
                        [
                            Pos2::new(center.x - grid_size * 1.5, center.y + offset),
                            Pos2::new(center.x + grid_size * 1.5, center.y + offset),
                        ],
                        Stroke::new(1.0, icon_color),
                    );
                }
            }
            GridStyle::HorizontalLines => {
                // Draw 3 horizontal lines
                for i in 0..3 {
                    let offset = (i as f32 - 1.0) * grid_size;
                    ui.painter().line_segment(
                        [
                            Pos2::new(center.x - grid_size * 1.5, center.y + offset),
                            Pos2::new(center.x + grid_size * 1.5, center.y + offset),
                        ],
                        Stroke::new(1.0, icon_color),
                    );
                }
            }
            GridStyle::CrossPlus => {
                // Draw small crosses at grid intersections
                let cross_size = 2.0;
                for i in -1..=1 {
                    for j in -1..=1 {
                        let cx = center.x + (i as f32) * grid_size;
                        let cy = center.y + (j as f32) * grid_size;
                        // Horizontal arm
                        ui.painter().line_segment(
                            [
                                Pos2::new(cx - cross_size, cy),
                                Pos2::new(cx + cross_size, cy),
                            ],
                            Stroke::new(1.0, icon_color),
                        );
                        // Vertical arm
                        ui.painter().line_segment(
                            [
                                Pos2::new(cx, cy - cross_size),
                                Pos2::new(cx, cy + cross_size),
                            ],
                            Stroke::new(1.0, icon_color),
                        );
                    }
                }
            }
            GridStyle::Handwriting | GridStyle::Calligraphy => {
                // Ruled writing lines: baseline darker; handwriting has a
                // dashed midline, calligraphy a fourth (descender) line.
                let four = style == GridStyle::Calligraphy;
                let ys: &[f32] = if four {
                    &[-6.0, -2.0, 2.0, 6.0]
                } else {
                    &[-5.0, 0.0, 5.0]
                };
                let base_index = 2;
                for (i, &dy) in ys.iter().enumerate() {
                    let y = center.y + dy;
                    let (x0, x1) = (center.x - 8.0, center.x + 8.0);
                    let width = if i == base_index { 1.5_f32 } else { 1.0 };
                    if !four && i == 1 {
                        let mut x = x0;
                        while x < x1 {
                            ui.painter().line_segment(
                                [Pos2::new(x, y), Pos2::new((x + 2.0).min(x1), y)],
                                Stroke::new(1.0_f32, icon_color),
                            );
                            x += 4.0;
                        }
                    } else {
                        ui.painter().line_segment(
                            [Pos2::new(x0, y), Pos2::new(x1, y)],
                            Stroke::new(width, icon_color),
                        );
                    }
                }
            }
            GridStyle::Dots => {
                // Draw dots at grid intersections
                let dot_radius = 1.5;
                for i in -1..=1 {
                    for j in -1..=1 {
                        let cx = center.x + (i as f32) * grid_size;
                        let cy = center.y + (j as f32) * grid_size;
                        ui.painter()
                            .circle_filled(Pos2::new(cx, cy), dot_radius, icon_color);
                    }
                }
            }
        }
    }

    response.on_hover_text(tooltip)
}

/// Controls for the handwriting and calligraphy guide lines.
fn writing_guides_editor(ui: &mut egui::Ui, guides: &mut WritingGuides, stroke_width: f32) {
    ui.label(
        egui::RichText::new("Guide lines")
            .size(11.0)
            .color(Color32::from_gray(100)),
    );
    ui.add(
        egui::Slider::new(&mut guides.x_height, 10.0..=300.0)
            .logarithmic(true)
            .suffix(" px")
            .text("x-height"),
    )
    .on_hover_text("Height of small letters such as a, x, o");
    ui.add(egui::Slider::new(&mut guides.ascender, 0.0..=2.5).text("Ascender ×"))
        .on_hover_text("Space above the x-height for b, d, h, k, l, as a multiple of it");
    ui.add(egui::Slider::new(&mut guides.descender, 0.0..=2.5).text("Descender ×"))
        .on_hover_text("Space below the baseline for g, j, p, q, y, as a multiple of it");
    ui.add(egui::Slider::new(&mut guides.row_gap, 0.0..=2.0).text("Row gap ×"));
    ui.add(
        egui::Slider::new(&mut guides.slant, 0.0..=45.0)
            .suffix("°")
            .text("Slant"),
    )
    .on_hover_text("Slant lines, in degrees from upright (0 hides them)");

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Proportions (ascender : x-height : descender)")
            .size(11.0)
            .color(Color32::from_gray(100)),
    );
    for (name, ascender, descender, slant) in WritingGuides::PRESETS {
        if ui.button(name).clicked() {
            guides.ascender = ascender;
            guides.descender = descender;
            guides.slant = slant;
        }
    }
    // A broad nib at full pressure is four stroke widths wide.
    let nib = f64::from(stroke_width) * 4.0;
    if ui
        .button("x-height = 5 nib widths")
        .on_hover_text(format!(
            "Size the guides to the Broad nib brush at the current stroke width ({nib:.1} px nib)"
        ))
        .clicked()
    {
        guides.x_height = (nib * 5.0).clamp(10.0, 300.0);
    }
    if ui.button("Reset").clicked() {
        *guides = WritingGuides::default();
    }
}

/// Render the right-side properties panel for selected shapes.
fn render_right_panel(ctx: &Context, props: &SelectedShapeProps) -> Option<UiAction> {
    // Show panel if shape is selected OR if a drawing tool is active
    if !props.has_selection && !props.is_drawing_tool {
        return None;
    }

    let mut action = None;
    let panel_width = 200.0;
    let margin = 12.0;

    egui::Area::new(egui::Id::new("right_panel"))
        .anchor(Align2::RIGHT_CENTER, Vec2::new(-margin, 0.0))
        .interactable(true)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::from_rgba_unmultiplied(250, 250, 252, 250))
                .corner_radius(CornerRadius::same(8))
                .stroke(Stroke::new(1.0, Color32::from_gray(220)))
                .shadow(egui::epaint::Shadow {
                    spread: 0,
                    blur: 6,
                    offset: [0, 2],
                    color: Color32::from_black_alpha(10),
                })
                .inner_margin(Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(panel_width - 24.0);

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 8.0);

                        // Panel title
                        ui.label(
                            egui::RichText::new("Properties")
                                .size(14.0)
                                .strong()
                                .color(Color32::from_gray(60)),
                        );
                        ui.add_space(4.0);

                        // Text-specific properties
                        if props.is_text {
                            // Font Family
                            ui.label(
                                egui::RichText::new("Font Family")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                let is_gelpen = props.font_family == FontFamily::GelPen;
                                if ToggleButton::new("GelPen", is_gelpen).show(ui) && !is_gelpen {
                                    action = Some(UiAction::SetFontFamily(0));
                                }

                                let is_gelpen_serif = props.font_family == FontFamily::GelPenSerif;
                                if ToggleButton::new("GelPen Serif", is_gelpen_serif).show(ui)
                                    && !is_gelpen_serif
                                {
                                    action = Some(UiAction::SetFontFamily(2));
                                }

                                // TODO: Re-enable Noto Sans once font rendering is finalized
                                // let is_noto = props.font_family == FontFamily::NotoSans;
                                // if ToggleButton::new("Noto", is_noto).show(ui) && !is_noto
                                // {
                                //     action = Some(UiAction::SetFontFamily(1));
                                // }

                                let is_vanilla = props.font_family == FontFamily::VanillaExtract;
                                if ToggleButton::new("Vanilla", is_vanilla).show(ui) && !is_vanilla
                                {
                                    action = Some(UiAction::SetFontFamily(3));
                                }
                            });

                            ui.add_space(4.0);

                            // Font Weight
                            ui.label(
                                egui::RichText::new("Font Weight")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                let is_light = props.font_weight == FontWeight::Light;
                                if ToggleButton::new("Light", is_light).show(ui) && !is_light {
                                    action = Some(UiAction::SetFontWeight(0));
                                }

                                let is_regular = props.font_weight == FontWeight::Regular;
                                if ToggleButton::new("Regular", is_regular).show(ui) && !is_regular
                                {
                                    action = Some(UiAction::SetFontWeight(1));
                                }

                                let is_heavy = props.font_weight == FontWeight::Heavy;
                                if ToggleButton::new("Heavy", is_heavy).show(ui) && !is_heavy {
                                    action = Some(UiAction::SetFontWeight(2));
                                }
                            });

                            ui.add_space(4.0);

                            // Font Size - S/M/L/XL buttons
                            ui.label(
                                egui::RichText::new("Font Size")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                // S = 16px
                                let is_small = (props.font_size - 16.0).abs() < 1.0;
                                if FontSizeButton::new("S", 16.0, is_small).show(ui) {
                                    action = Some(UiAction::SetFontSize(16.0));
                                }

                                // M = 20px (default)
                                let is_medium = (props.font_size - 20.0).abs() < 1.0;
                                if FontSizeButton::new("M", 20.0, is_medium).show(ui) {
                                    action = Some(UiAction::SetFontSize(20.0));
                                }

                                // L = 28px
                                let is_large = (props.font_size - 28.0).abs() < 1.0;
                                if FontSizeButton::new("L", 28.0, is_large).show(ui) {
                                    action = Some(UiAction::SetFontSize(28.0));
                                }

                                // XL = 36px
                                let is_xlarge = (props.font_size - 36.0).abs() < 1.0;
                                if FontSizeButton::new("XL", 36.0, is_xlarge).show(ui) {
                                    action = Some(UiAction::SetFontSize(36.0));
                                }
                            });
                        }

                        // Math-specific properties (font size only)
                        if props.is_math {
                            ui.label(
                                egui::RichText::new("Font Size")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                let is_small = (props.font_size - 16.0).abs() < 1.0;
                                if FontSizeButton::new("S", 16.0, is_small).show(ui) {
                                    action = Some(UiAction::SetMathFontSize(16.0));
                                }

                                let is_medium = (props.font_size - 20.0).abs() < 1.0;
                                if FontSizeButton::new("M", 20.0, is_medium).show(ui) {
                                    action = Some(UiAction::SetMathFontSize(20.0));
                                }

                                let is_large = (props.font_size - 28.0).abs() < 1.0;
                                if FontSizeButton::new("L", 28.0, is_large).show(ui) {
                                    action = Some(UiAction::SetMathFontSize(28.0));
                                }

                                let is_xlarge = (props.font_size - 36.0).abs() < 1.0;
                                if FontSizeButton::new("XL", 36.0, is_xlarge).show(ui) {
                                    action = Some(UiAction::SetMathFontSize(36.0));
                                }
                            });
                        }

                        // Rectangle-specific properties (for selected rect OR rectangle tool)
                        if props.is_rectangle || props.tool_is_rectangle {
                            // Corner Radius - On/Off toggle
                            ui.label(
                                egui::RichText::new("Rounded Corners")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                // Off = 0px (sharp corners)
                                let is_off = props.corner_radius < 1.0;
                                if ToggleButton::new("Off", is_off).show(ui) && !is_off {
                                    action = Some(UiAction::SetCornerRadius(0.0));
                                }

                                // On = 32px (adaptive radius)
                                let is_on = props.corner_radius >= 1.0;
                                if ToggleButton::new("On", is_on).show(ui) && !is_on {
                                    action = Some(UiAction::SetCornerRadius(32.0));
                                }
                            });
                        }

                        // Sloppiness (for all shapes except text and freehand/highlighter)
                        if !props.is_text && !props.is_freehand {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Sloppiness")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                // Architect = 0 (clean lines)
                                let is_architect = props.sloppiness == 0;
                                if ToggleButton::new("Architect", is_architect).show(ui)
                                    && !is_architect
                                {
                                    action = Some(UiAction::SetSloppiness(0));
                                }

                                // Artist = 1 (slight wobble)
                                let is_artist = props.sloppiness == 1;
                                if ToggleButton::new("Artist", is_artist).show(ui) && !is_artist {
                                    action = Some(UiAction::SetSloppiness(1));
                                }

                                // Cartoonist = 2 (very sketchy)
                                let is_cartoonist = props.sloppiness == 2;
                                if ToggleButton::new("Cartoonist", is_cartoonist).show(ui)
                                    && !is_cartoonist
                                {
                                    action = Some(UiAction::SetSloppiness(2));
                                }

                                // Drunk = 3 (chaotic)
                                let is_drunk = props.sloppiness == 3;
                                if ToggleButton::new("Drunk", is_drunk).show(ui) && !is_drunk {
                                    action = Some(UiAction::SetSloppiness(3));
                                }
                            });
                        }

                        // Fill pattern (only for shapes with fill, not lines/arrows/freehand)
                        if props.has_fill && !props.is_line && !props.is_arrow && !props.is_freehand
                        {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Fill Pattern")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                                let patterns =
                                    [(0u8, "Solid"), (1, "Hatch"), (3, "Cross"), (4, "Dots")];
                                for (idx, name) in patterns {
                                    let is_selected = props.fill_pattern == idx;
                                    if ToggleButton::new(name, is_selected).show(ui) && !is_selected
                                    {
                                        action = Some(UiAction::SetFillPattern(idx));
                                    }
                                }
                            });
                        }

                        // Path style (for lines and arrows only)
                        if props.is_line || props.is_arrow {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Path")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                let is_direct = props.path_style == 0;
                                if ToggleButton::new("Direct", is_direct).show(ui) && !is_direct {
                                    action = Some(UiAction::SetPathStyle(0));
                                }

                                let is_flowing = props.path_style == 1;
                                if ToggleButton::new("Flowing", is_flowing).show(ui) && !is_flowing
                                {
                                    action = Some(UiAction::SetPathStyle(1));
                                }

                                let is_angular = props.path_style == 2;
                                if ToggleButton::new("Angular", is_angular).show(ui) && !is_angular
                                {
                                    action = Some(UiAction::SetPathStyle(2));
                                }
                            });

                            // Stroke style (solid/dashed/dotted)
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Stroke")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                let is_solid = props.stroke_style == 0;
                                if ToggleButton::new("Solid", is_solid).show(ui) && !is_solid {
                                    action = Some(UiAction::SetStrokeStyle(0));
                                }

                                let is_dashed = props.stroke_style == 1;
                                if ToggleButton::new("Dashed", is_dashed).show(ui) && !is_dashed {
                                    action = Some(UiAction::SetStrokeStyle(1));
                                }

                                let is_dotted = props.stroke_style == 2;
                                if ToggleButton::new("Dotted", is_dotted).show(ui) && !is_dotted {
                                    action = Some(UiAction::SetStrokeStyle(2));
                                }
                            });
                        }

                        // Brush (pen tool and pen strokes)
                        if let Some(current) = props.brush {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Brush")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                                for brush in Brush::ALL {
                                    if ToggleButton::new(brush.name(), current == brush).show(ui)
                                        && current != brush
                                    {
                                        action = Some(UiAction::SetBrush(brush));
                                    }
                                }
                            });
                            ui.label(
                                egui::RichText::new(current.description())
                                    .size(10.0)
                                    .color(Color32::from_gray(120)),
                            );
                            if current == Brush::BroadNib {
                                let mut angle = props.nib_angle;
                                let slider = egui::Slider::new(&mut angle, 0.0..=90.0)
                                    .suffix("°")
                                    .text("Nib angle");
                                if ui
                                    .add(slider)
                                    .on_hover_text(
                                        "Angle of the nib edge from horizontal. \
                                         Foundational 30°, italic 40-45°, blackletter 35-40°",
                                    )
                                    .changed()
                                {
                                    action = Some(UiAction::SetNibAngle(angle.round()));
                                }
                            }
                        }

                        // Motion smoothing (for freehand tool only)
                        if props.is_freehand {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Motion")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                                if ToggleButton::new("Direct", !props.calligraphy_mode).show(ui)
                                    && props.calligraphy_mode
                                {
                                    action = Some(UiAction::ToggleCalligraphy);
                                }
                                if ToggleButton::new("Smoothed", props.calligraphy_mode).show(ui)
                                    && !props.calligraphy_mode
                                {
                                    action = Some(UiAction::ToggleCalligraphy);
                                }
                            });

                            // Pressure simulation toggle
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                                if ToggleButton::new("Uniform", !props.pressure_simulation).show(ui)
                                    && props.pressure_simulation
                                {
                                    action = Some(UiAction::TogglePressureSimulation);
                                }
                                if ToggleButton::new("Pressure", props.pressure_simulation).show(ui)
                                    && !props.pressure_simulation
                                {
                                    action = Some(UiAction::TogglePressureSimulation);
                                }
                            });
                        }

                        // Z-Order controls (only when shapes are selected, not for drawing tools)
                        if props.has_selection && !props.is_drawing_tool {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Layer")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                // Back (send to bottommost)
                                if IconButton::new(
                                    include_image!("../assets/layer-back.svg"),
                                    "Send to Back",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::SendToBack);
                                }

                                // Backward (one layer down)
                                if IconButton::new(
                                    include_image!("../assets/layer-backward.svg"),
                                    "Send Backward",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::SendBackward);
                                }

                                // Forward (one layer up)
                                if IconButton::new(
                                    include_image!("../assets/layer-forward.svg"),
                                    "Bring Forward",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::BringForward);
                                }

                                // Front (bring to topmost)
                                if IconButton::new(
                                    include_image!("../assets/layer-front.svg"),
                                    "Bring to Front",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::BringToFront);
                                }
                            });

                            // Flip/Mirror controls
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Transform")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                if IconButton::new(
                                    include_image!("../assets/flip-h.svg"),
                                    "Flip Horizontal",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::FlipHorizontal);
                                }
                                if IconButton::new(
                                    include_image!("../assets/flip-v.svg"),
                                    "Flip Vertical",
                                )
                                .show(ui)
                                {
                                    action = Some(UiAction::FlipVertical);
                                }
                            });

                            // Opacity control
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new("Opacity")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                let mut opacity = props.opacity;
                                let slider = egui::Slider::new(&mut opacity, 0.0..=1.0)
                                    .show_value(false)
                                    .custom_formatter(|v, _| format!("{}%", (v * 100.0) as i32));
                                if ui.add(slider).changed() {
                                    action = Some(UiAction::SetOpacity(opacity));
                                }
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{}%",
                                        (props.opacity * 100.0) as i32
                                    ))
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                                );
                            });

                            // Alignment controls (only when 2+ shapes are selected)
                            if props.selection_count >= 2 {
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("Align")
                                        .size(11.0)
                                        .color(Color32::from_gray(100)),
                                );
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                    if IconButton::new(
                                        include_image!("../assets/align-left.svg"),
                                        "Align Left",
                                    )
                                    .show(ui)
                                    {
                                        action = Some(UiAction::AlignLeft);
                                    }
                                    if IconButton::new(
                                        include_image!("../assets/align-center-v.svg"),
                                        "Align Center (Vertical)",
                                    )
                                    .show(ui)
                                    {
                                        action = Some(UiAction::AlignCenterV);
                                    }
                                    if IconButton::new(
                                        include_image!("../assets/align-right.svg"),
                                        "Align Right",
                                    )
                                    .show(ui)
                                    {
                                        action = Some(UiAction::AlignRight);
                                    }
                                    if IconButton::new(
                                        include_image!("../assets/align-top.svg"),
                                        "Align Top",
                                    )
                                    .show(ui)
                                    {
                                        action = Some(UiAction::AlignTop);
                                    }
                                    if IconButton::new(
                                        include_image!("../assets/align-center-h.svg"),
                                        "Align Center (Horizontal)",
                                    )
                                    .show(ui)
                                    {
                                        action = Some(UiAction::AlignCenterH);
                                    }
                                    if IconButton::new(
                                        include_image!("../assets/align-bottom.svg"),
                                        "Align Bottom",
                                    )
                                    .show(ui)
                                    {
                                        action = Some(UiAction::AlignBottom);
                                    }
                                });
                            }
                        }
                    });
                });
        });

    action
}

/// Render the hamburger menu and collab button at top-left.
fn render_file_menu(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    let has_selection = ui_state.selection_count > 0;

    // Hamburger button and collab button in a panel
    egui::Area::new(egui::Id::new("hamburger_button"))
        .anchor(Align2::LEFT_TOP, Vec2::new(12.0, 12.0))
        .show(ctx, |ui| {
            panel_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                    if hamburger_button(ui, ui_state.menu_open) {
                        ui_state.menu_open = !ui_state.menu_open;
                        ui_state.collab_modal_open = false; // Close modal when opening menu
                    }

                    if collab_button(ui, ui_state.connection_state, ui_state.collab_modal_open) {
                        ui_state.collab_modal_open = !ui_state.collab_modal_open;
                        ui_state.menu_open = false; // Close menu when opening modal
                    }
                });
            });
        });

    // Practice submenu: the parent item and its rect, set while drawing the menu.
    let mut practice_anchor: Option<(SheetKind, Rect)> = None;

    // Dropdown menu (only shown when open)
    if ui_state.menu_open {
        egui::Area::new(egui::Id::new("file_menu_dropdown"))
            .anchor(Align2::LEFT_TOP, Vec2::new(12.0, 56.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                panel_frame().show(ui, |ui| {
                    ui.set_width(180.0); // Fixed menu width
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

                        if menu_item(ui, "Save Local", &ui_state.keybindings.hint(Command::Save)) {
                            action = Some(UiAction::SaveLocal);
                            ui_state.menu_open = false;
                        }
                        if menu_item(ui, "Save Local As...", "") {
                            action = Some(UiAction::SaveLocalAs);
                            ui_state.menu_open = false;
                        }

                        widgets_menu_separator(ui);

                        if menu_item(ui, "Open", &ui_state.keybindings.hint(Command::Open)) {
                            action = Some(UiAction::ShowOpenDialog);
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "Open Recent...", "") {
                            action = Some(UiAction::ShowOpenRecentDialog);
                            ui_state.menu_open = false;
                        }

                        widgets_menu_separator(ui);

                        if menu_item(ui, "Clear", "") {
                            action = Some(UiAction::ClearDocument);
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "Show Intro", "") {
                            action = Some(UiAction::ShowIntro);
                            ui_state.menu_open = false;
                        }

                        // Practice sheet submenus open on hover (or click).
                        let mut over_parent = false;
                        for kind in [SheetKind::Handwriting, SheetKind::Calligraphy] {
                            let item =
                                ui.scope(|ui| menu_item(ui, &format!("{}  ›", kind.name()), ""));
                            let hovered = ui.rect_contains_pointer(item.response.rect);
                            over_parent |= hovered;
                            if item.inner || hovered {
                                ui_state.practice_submenu = Some(kind);
                            }
                            if ui_state.practice_submenu == Some(kind) {
                                practice_anchor = Some((kind, item.response.rect));
                            }
                        }
                        // Hovering another item closes the submenu.
                        if !over_parent && ui.ui_contains_pointer() {
                            ui_state.practice_submenu = None;
                            practice_anchor = None;
                        }

                        widgets_menu_separator(ui);

                        // Download/Upload for file export/import (WASM shows both, native just uses Save/Open)
                        #[cfg(target_arch = "wasm32")]
                        {
                            if menu_item(ui, "Download JSON", "") {
                                action = Some(UiAction::DownloadDocument);
                                ui_state.menu_open = false;
                            }
                            if menu_item(ui, "Import JSON/Excalidraw", "") {
                                action = Some(UiAction::UploadDocument);
                                ui_state.menu_open = false;
                            }
                            widgets_menu_separator(ui);
                        }

                        if menu_item(ui, "Import Mermaid (Clipboard)", "") {
                            action = Some(UiAction::ImportMermaidFromClipboard);
                            ui_state.menu_open = false;
                        }

                        widgets_menu_separator(ui);

                        if menu_item(
                            ui,
                            "Export PNG",
                            &ui_state.keybindings.hint(Command::ExportPng),
                        ) {
                            action = Some(UiAction::ExportPng);
                            ui_state.menu_open = false;
                        }
                        // Copy as PNG (show disabled state if no selection)
                        if menu_item_enabled(
                            ui,
                            "Copy as PNG",
                            &ui_state.keybindings.hint(Command::CopyPng),
                            has_selection,
                        ) {
                            action = Some(UiAction::CopyPng);
                            ui_state.menu_open = false;
                        }

                        // Export scale selector
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0); // Align with menu item text
                            ui.label(
                                egui::RichText::new("Scale:")
                                    .size(11.0)
                                    .color(Color32::from_rgb(100, 116, 139)),
                            );
                            ui.add_space(4.0);
                            for scale in [1u8, 2, 3] {
                                let label = format!("{}x", scale);
                                let selected = ui_state.export_scale == scale;
                                let btn = egui::Button::new(
                                    egui::RichText::new(&label).size(11.0).color(if selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_gray(80)
                                    }),
                                )
                                .fill(if selected {
                                    Color32::from_rgb(59, 130, 246)
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .stroke(egui::Stroke::NONE)
                                .corner_radius(egui::CornerRadius::same(4))
                                .min_size(Vec2::new(24.0, 20.0));
                                if ui.add(btn).clicked() {
                                    action = Some(UiAction::SetExportScale(scale));
                                }
                            }
                        });

                        widgets_menu_separator(ui);

                        if menu_item(
                            ui,
                            "Settings...",
                            &ui_state.keybindings.hint(Command::Settings),
                        ) {
                            ui_state.settings_open = true;
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "Keyboard Shortcuts", "") {
                            ui_state.settings_open = true;
                            ui_state.settings_tab = SettingsTab::Keyboard;
                            ui_state.menu_open = false;
                        }

                        if menu_item(ui, "Pen & Tablet", "") {
                            ui_state.settings_open = true;
                            ui_state.settings_tab = SettingsTab::Controls;
                            ui_state.menu_open = false;
                        }
                    });
                });
            });

        // Close menu when clicking outside
        if ctx.input(|i| i.pointer.any_click()) {
            let buttons_rect = Rect::from_min_size(
                Pos2::new(12.0, 12.0),
                Vec2::new(80.0, 48.0), // Wider to include collab button
            );
            let menu_rect = Rect::from_min_size(
                Pos2::new(12.0, 56.0),
                Vec2::new(180.0, 250.0), // Shorter now without collaboration section
            );
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if !buttons_rect.contains(pos) && !menu_rect.contains(pos) {
                    ui_state.menu_open = false;
                }
            }
        }

        if let Some((kind, rect)) = practice_anchor {
            egui::Area::new(egui::Id::new("practice_submenu"))
                .order(egui::Order::Foreground)
                .fixed_pos(rect.right_top() + Vec2::new(10.0, -8.0))
                .constrain(true)
                .show(ctx, |ui| {
                    panel_frame().show(ui, |ui| {
                        ui.set_width(220.0);
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                        for sheet in PracticeSheet::all().filter(|s| s.kind() == kind) {
                            if menu_item(ui, sheet.name(), "") {
                                action = Some(UiAction::ShowPractice(sheet));
                                ui_state.menu_open = false;
                                ui_state.practice_submenu = None;
                            }
                        }
                    });
                });
        }
    }

    // Render collaboration modal if open
    if ui_state.collab_modal_open {
        if let Some(modal_action) = render_collaboration_modal(ctx, ui_state) {
            action = Some(modal_action);
        }
    }

    if ui_state.settings_open {
        render_settings(ctx, ui_state);
    }

    // Render save dialog if open
    if ui_state.save_dialog_open {
        if let Some(save_action) = render_save_dialog(ctx, ui_state) {
            action = Some(save_action);
        }
    }

    // Render open dialog if open
    if ui_state.open_dialog_open {
        if let Some(open_action) = render_open_dialog(ctx, ui_state) {
            action = Some(open_action);
        }
    }

    // Render open recent dialog if open
    if ui_state.open_recent_dialog_open {
        if let Some(open_action) = render_open_recent_dialog(ctx, ui_state) {
            action = Some(open_action);
        }
    }

    action
}

/// Hamburger menu button (three horizontal lines).
fn hamburger_button(ui: &mut egui::Ui, is_open: bool) -> bool {
    let size = Vec2::new(32.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        let bg_color = if is_open {
            Color32::from_rgb(59, 130, 246)
        } else if response.hovered() {
            Color32::from_gray(235)
        } else {
            Color32::TRANSPARENT
        };

        let line_color = if is_open {
            Color32::WHITE
        } else {
            Color32::from_gray(80)
        };

        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), bg_color);

        // Draw three horizontal lines
        let line_width = 14.0;
        let line_height = 2.0;
        let spacing = 4.0;
        let start_x = rect.center().x - line_width / 2.0;
        let center_y = rect.center().y;

        for i in -1..=1 {
            let y = center_y + (i as f32) * spacing;
            let line_rect = Rect::from_min_size(
                Pos2::new(start_x, y - line_height / 2.0),
                Vec2::new(line_width, line_height),
            );
            ui.painter()
                .rect_filled(line_rect, CornerRadius::same(1), line_color);
        }
    }

    response.on_hover_text("Menu").clicked()
}

/// Collaboration button with connection status indicator.
fn collab_button(ui: &mut egui::Ui, connection_state: ConnectionState, is_open: bool) -> bool {
    let size = Vec2::new(32.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        // Background
        let bg_color = if is_open {
            Color32::from_rgb(59, 130, 246)
        } else if response.hovered() {
            Color32::from_gray(235)
        } else {
            Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), bg_color);

        // Status indicator color
        let status_color = match connection_state {
            ConnectionState::Connected => Color32::from_rgb(34, 197, 94), // Green
            ConnectionState::Connecting => Color32::from_rgb(250, 204, 21), // Yellow
            ConnectionState::Disconnected => Color32::from_gray(160),     // Gray
            ConnectionState::Error => Color32::from_rgb(239, 68, 68),     // Red
        };

        let icon_color = if is_open {
            Color32::WHITE
        } else {
            Color32::from_gray(80)
        };

        // Draw two person silhouettes (simplified, smaller)
        let center = rect.center();

        // Left person (head + body arc)
        let left_x = center.x - 4.0;
        ui.painter()
            .circle_filled(Pos2::new(left_x, center.y - 3.0), 2.5, icon_color);
        ui.painter()
            .circle_filled(Pos2::new(left_x, center.y + 4.0), 3.5, icon_color);

        // Right person (head + body arc)
        let right_x = center.x + 4.0;
        ui.painter()
            .circle_filled(Pos2::new(right_x, center.y - 3.0), 2.5, icon_color);
        ui.painter()
            .circle_filled(Pos2::new(right_x, center.y + 4.0), 3.5, icon_color);

        // Status dot in top-right corner
        let dot_pos = Pos2::new(rect.right() - 5.0, rect.top() + 5.0);
        ui.painter().circle_filled(dot_pos, 3.5, status_color);
        ui.painter()
            .circle_stroke(dot_pos, 3.5, Stroke::new(1.5, Color32::WHITE));
    }

    let tooltip = match connection_state {
        ConnectionState::Connected => "Connected - Click to manage collaboration",
        ConnectionState::Connecting => "Connecting...",
        ConnectionState::Disconnected => "Not connected - Click to collaborate",
        ConnectionState::Error => "Connection error - Click to retry",
    };
    response.on_hover_text(tooltip).clicked()
}

/// Render the collaboration modal dialog.
fn render_collaboration_modal(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Semi-transparent backdrop
    #[allow(deprecated)]
    let screen_rect = ctx.input(|i| i.content_rect());
    egui::Area::new(egui::Id::new("collab_modal_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Middle)
        .interactable(true)
        .show(ctx, |ui| {
            let (rect, response) = ui.allocate_exact_size(screen_rect.size(), egui::Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(80));
            // Click on backdrop closes modal
            if response.clicked() {
                ui_state.collab_modal_open = false;
            }
        });

    // Modal dialog
    let modal_width = 320.0;

    egui::Area::new(egui::Id::new("collab_modal"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .interactable(true)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .shadow(egui::epaint::Shadow {
                    spread: 2,
                    blur: 20,
                    offset: [0, 4],
                    color: Color32::from_black_alpha(40),
                })
                .inner_margin(Margin::same(24))
                .show(ui, |ui| {
                    // Apply light theme visuals for this modal
                    ui.visuals_mut().widgets.inactive.bg_fill = Color32::from_gray(245);
                    ui.visuals_mut().widgets.inactive.bg_stroke =
                        Stroke::new(1.0, Color32::from_gray(200));
                    ui.visuals_mut().widgets.hovered.bg_fill = Color32::from_gray(235);
                    ui.visuals_mut().widgets.hovered.bg_stroke =
                        Stroke::new(1.0, Color32::from_gray(180));
                    ui.visuals_mut().widgets.active.bg_fill = Color32::from_gray(225);
                    ui.visuals_mut().widgets.active.bg_stroke =
                        Stroke::new(1.0, Color32::from_rgb(59, 130, 246));
                    ui.visuals_mut().extreme_bg_color = Color32::WHITE;
                    ui.visuals_mut().override_text_color = Some(Color32::from_gray(30));
                    ui.visuals_mut().selection.bg_fill =
                        Color32::from_rgb(59, 130, 246).gamma_multiply(0.3);
                    ui.visuals_mut().selection.stroke =
                        Stroke::new(1.0, Color32::from_rgb(59, 130, 246));

                    ui.set_width(modal_width);

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(0.0, 12.0);

                        // Header with close button
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Collaborate")
                                    .size(18.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.collab_modal_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);

                        // Connection status
                        let status_color = match ui_state.connection_state {
                            ConnectionState::Connected => Color32::from_rgb(34, 197, 94),
                            ConnectionState::Connecting => Color32::from_rgb(250, 204, 21),
                            ConnectionState::Disconnected => Color32::from_gray(148),
                            ConnectionState::Error => Color32::from_rgb(239, 68, 68),
                        };
                        let status_text = match ui_state.connection_state {
                            ConnectionState::Connected => {
                                if let Some(ref room) = ui_state.current_room {
                                    format!("Connected to room: {}", room)
                                } else {
                                    "Connected (no room)".to_string()
                                }
                            }
                            ConnectionState::Connecting => "Connecting...".to_string(),
                            ConnectionState::Disconnected => "Not connected".to_string(),
                            ConnectionState::Error => "Connection error".to_string(),
                        };

                        ui.horizontal(|ui| {
                            let (dot_rect, _) =
                                ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                            ui.painter()
                                .circle_filled(dot_rect.center(), 5.0, status_color);
                            ui.label(
                                egui::RichText::new(&status_text)
                                    .size(13.0)
                                    .color(Color32::from_gray(60)),
                            );
                        });

                        if ui_state.current_room.is_some() {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} peer(s) online",
                                    ui_state.peer_count
                                ))
                                .size(11.0)
                                .color(Color32::from_gray(100)),
                            );
                        }

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        // Server URL
                        ui.label(
                            egui::RichText::new("Server URL")
                                .size(12.0)
                                .strong()
                                .color(Color32::from_gray(60)),
                        );
                        input_text(ui, &mut ui_state.server_url, modal_width, "/ws");

                        ui.add_space(4.0);

                        // Connect/Disconnect button (styled)
                        ui.horizontal(|ui| {
                            let button_text = match ui_state.connection_state {
                                ConnectionState::Disconnected | ConnectionState::Error => "Connect",
                                ConnectionState::Connected | ConnectionState::Connecting => {
                                    "Disconnect"
                                }
                            };
                            if primary_btn(ui, button_text) {
                                match ui_state.connection_state {
                                    ConnectionState::Disconnected | ConnectionState::Error => {
                                        action =
                                            Some(UiAction::Connect(ui_state.server_url.clone()));
                                    }
                                    ConnectionState::Connected | ConnectionState::Connecting => {
                                        action = Some(UiAction::Disconnect);
                                    }
                                }
                            }
                        });

                        // Room controls (only when connected)
                        if ui_state.connection_state == ConnectionState::Connected {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            ui.label(
                                egui::RichText::new("Room")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_gray(60)),
                            );
                            input_text(
                                ui,
                                &mut ui_state.room_input,
                                modal_width,
                                "Enter room name",
                            );

                            ui.add_space(4.0);

                            if ui_state.current_room.is_some() {
                                let leave_btn = egui::Button::new(
                                    egui::RichText::new("Leave Room").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(239, 68, 68))
                                .min_size(Vec2::new(modal_width, 36.0))
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(leave_btn).clicked() {
                                    action = Some(UiAction::LeaveRoom);
                                }
                            } else if !ui_state.room_input.is_empty() {
                                let join_btn = egui::Button::new(
                                    egui::RichText::new("Join Room").color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(34, 197, 94))
                                .min_size(Vec2::new(modal_width, 36.0))
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(join_btn).clicked() {
                                    action = Some(UiAction::JoinRoom(ui_state.room_input.clone()));
                                }
                            }

                            ui.separator();
                            ui.add_space(4.0);

                            // Your Profile section
                            ui.label(
                                egui::RichText::new("Your Profile")
                                    .size(12.0)
                                    .strong()
                                    .color(Color32::from_gray(60)),
                            );
                            ui.add_space(4.0);

                            // Name input
                            ui.label(
                                egui::RichText::new("Display Name")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            let name_response =
                                input_text(ui, &mut ui_state.user_name, modal_width, "Anonymous");
                            if name_response.lost_focus() {
                                action = Some(UiAction::SetUserName(ui_state.user_name.clone()));
                            }

                            ui.add_space(8.0);

                            // Color picker
                            ui.label(
                                egui::RichText::new("Cursor Color")
                                    .size(11.0)
                                    .color(Color32::from_gray(100)),
                            );
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                                let colors = [
                                    ("#ef4444", "Red"),
                                    ("#f97316", "Orange"),
                                    ("#eab308", "Yellow"),
                                    ("#22c55e", "Green"),
                                    ("#06b6d4", "Cyan"),
                                    ("#3b82f6", "Blue"),
                                    ("#6366f1", "Indigo"),
                                    ("#a855f7", "Purple"),
                                    ("#ec4899", "Pink"),
                                ];
                                for (hex, name) in colors {
                                    let color = parse_color(hex);
                                    let is_selected = ui_state.user_color == hex;
                                    let size = 24.0;
                                    let (rect, response) = ui.allocate_exact_size(
                                        Vec2::new(size, size),
                                        egui::Sense::click(),
                                    );

                                    // Draw color circle
                                    ui.painter().circle_filled(rect.center(), size / 2.0, color);

                                    // Selection indicator (checkmark or ring)
                                    if is_selected {
                                        ui.painter().circle_stroke(
                                            rect.center(),
                                            size / 2.0 + 2.0,
                                            Stroke::new(2.0, Color32::from_gray(60)),
                                        );
                                    }

                                    // Hover effect
                                    if response.hovered() && !is_selected {
                                        ui.painter().circle_stroke(
                                            rect.center(),
                                            size / 2.0 + 1.0,
                                            Stroke::new(1.0, Color32::from_gray(150)),
                                        );
                                    }

                                    if response.clicked() {
                                        ui_state.user_color = hex.to_string();
                                        action = Some(UiAction::SetUserColor(hex.to_string()));
                                    }
                                    response.on_hover_text(name);
                                }
                            });
                        }
                    });
                });
        });

    action
}

/// Menu item with label and shortcut - uses drafftink_widgets.
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: &str) -> bool {
    widgets_menu_item(ui, label, shortcut)
}

/// Menu item with label, shortcut, and enabled state - uses drafftink_widgets.
fn menu_item_enabled(ui: &mut egui::Ui, label: &str, shortcut: &str, enabled: bool) -> bool {
    widgets_menu_item_enabled(ui, label, shortcut, enabled)
}

/// Common panel frame style - uses drafftink_widgets.
fn panel_frame() -> Frame {
    widgets_panel_frame()
}

/// Vertical separator for panels.
fn panel_separator(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let line_rect = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + 4.0),
        Vec2::new(1.0, rect.height() - 8.0),
    );
    ui.painter()
        .rect_filled(line_rect, 0.0, Color32::from_gray(220));
    ui.add_space(1.0);
}

/// Color swatch circle for fill colors (supports "none") - uses drafftink_widgets.
fn fill_swatch(ui: &mut egui::Ui, color: Option<Color32>, name: &str, selected: bool) -> bool {
    match color {
        Some(c) => {
            let (clicked, _) = ColorSwatch::new(c, name).selected(selected).show(ui);
            clicked
        }
        None => NoColorSwatch::new(name).selected(selected).show(ui),
    }
}

/// Parse a CSS color string to Color32.
fn parse_color(color: &str) -> Color32 {
    // Handle hex colors
    if let Some(hex) = color.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
            return Color32::from_rgb(r, g, b);
        }
    }
    // Default color for unparseable strings
    Color32::from_rgb(100, 116, 139) // slate-500
}

/// Render the presence panel showing connected users.
fn render_presence_panel(ctx: &Context, ui_state: &UiState) {
    // Only show if in a room with peers
    if ui_state.current_room.is_none() || ui_state.peers.is_empty() {
        return;
    }

    let margin = 12.0;

    egui::Area::new(egui::Id::new("presence_panel"))
        .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-margin, -margin))
        .interactable(false)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(0.0, 0.0);

                // List of peers
                for peer in ui_state.peers.iter().take(8) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                        // Color dot
                        let color = parse_color(&peer.color);
                        let (dot_rect, _) =
                            ui.allocate_exact_size(Vec2::new(8.0, 8.0), egui::Sense::hover());
                        ui.painter().circle_filled(dot_rect.center(), 4.0, color);

                        // Name
                        let display_name = peer.name.as_deref().unwrap_or("Anonymous");
                        ui.label(
                            egui::RichText::new(display_name)
                                .size(10.0)
                                .color(Color32::from_gray(100)),
                        );
                    });
                }
            });
        });
}

/// Render the save dialog modal.
fn render_save_dialog(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Backdrop
    egui::Area::new(egui::Id::new("save_dialog_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                ui_state.save_dialog_open = false;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("save_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Save Document")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.save_dialog_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("Document name:")
                                .size(12.0)
                                .color(Color32::from_gray(60)),
                        );
                        let response = input_text(ui, &mut ui_state.save_name_input, 300.0, "");

                        if response.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            && !ui_state.save_name_input.trim().is_empty()
                        {
                            action = Some(UiAction::SaveLocalWithName(
                                ui_state.save_name_input.trim().to_string(),
                            ));
                            ui_state.save_dialog_open = false;
                        }

                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            if secondary_btn(ui, "Cancel") {
                                ui_state.save_dialog_open = false;
                            }
                            if primary_btn(ui, "Save")
                                && !ui_state.save_name_input.trim().is_empty()
                            {
                                action = Some(UiAction::SaveLocalWithName(
                                    ui_state.save_name_input.trim().to_string(),
                                ));
                                ui_state.save_dialog_open = false;
                            }
                        });
                    });
                });
        });

    action
}

/// Render the open dialog modal with dropdown.
fn render_open_dialog(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Backdrop
    egui::Area::new(egui::Id::new("open_dialog_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                ui_state.open_dialog_open = false;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("open_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Open Document")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.open_dialog_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("Select document:")
                                .size(12.0)
                                .color(Color32::from_gray(60)),
                        );

                        if ui_state.recent_documents.is_empty() {
                            ui.label(
                                egui::RichText::new("No saved documents")
                                    .color(Color32::from_gray(150)),
                            );
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    for doc_name in &ui_state.recent_documents.clone() {
                                        if ui.button(doc_name).clicked() {
                                            action = Some(UiAction::LoadLocal(doc_name.clone()));
                                            ui_state.open_dialog_open = false;
                                        }
                                    }
                                });
                        }
                    });
                });
        });

    action
}

/// Render the open recent dialog modal.
fn render_open_recent_dialog(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;

    // Backdrop
    egui::Area::new(egui::Id::new("open_recent_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                ui_state.open_recent_dialog_open = false;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("open_recent_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(300.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Open Recent")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        ui_state.open_recent_dialog_open = false;
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("Select document:")
                                .size(12.0)
                                .color(Color32::from_gray(60)),
                        );

                        if ui_state.recent_documents.is_empty() {
                            ui.label(
                                egui::RichText::new("No recent documents")
                                    .color(Color32::from_gray(150)),
                            );
                        } else {
                            ui.add_space(4.0);

                            let selected_text = ui_state
                                .selected_recent_document
                                .as_deref()
                                .unwrap_or("Select a document")
                                .to_string();
                            ui.scope(|ui| {
                                ui.visuals_mut().widgets.inactive.bg_stroke =
                                    Stroke::new(1.0, Color32::from_gray(220));
                                ui.visuals_mut().widgets.hovered.bg_stroke =
                                    Stroke::new(1.0, Color32::from_gray(180));
                                ui.visuals_mut().widgets.active.bg_stroke =
                                    Stroke::new(1.0, Color32::from_rgb(59, 130, 246));
                                ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::WHITE;
                                ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::WHITE;

                                egui::ComboBox::from_id_salt("recent_docs_dropdown")
                                    .selected_text(selected_text)
                                    .width(260.0)
                                    .show_ui(ui, |ui| {
                                        for doc_name in &ui_state.recent_documents.clone() {
                                            ui.selectable_value(
                                                &mut ui_state.selected_recent_document,
                                                Some(doc_name.clone()),
                                                doc_name,
                                            );
                                        }
                                    });
                            });

                            ui.add_space(12.0);

                            ui.horizontal(|ui| {
                                if primary_btn(ui, "Open") {
                                    if let Some(doc_name) = &ui_state.selected_recent_document {
                                        action = Some(UiAction::LoadLocal(doc_name.clone()));
                                        ui_state.open_recent_dialog_open = false;
                                    }
                                }
                            });
                        }
                    });
                });
        });

    action
}

/// Render the math equation editor dialog.
fn render_math_editor(ctx: &Context, ui_state: &mut UiState) -> Option<UiAction> {
    let (shape_id, latex_input) = ui_state.math_editor.as_mut()?;
    let shape_id = *shape_id;
    let mut action = None;
    let mut close = false;

    // Backdrop
    egui::Area::new(egui::Id::new("math_editor_backdrop"))
        .fixed_pos(Pos2::ZERO)
        .order(egui::Order::Background)
        .show(ctx, |ui| {
            let screen_rect = ctx.input(|i| i.content_rect());
            let response = ui.allocate_rect(screen_rect, egui::Sense::click());
            ui.painter()
                .rect_filled(screen_rect, 0.0, Color32::from_black_alpha(80));
            if response.clicked() {
                close = true;
            }
        });

    // Modal window
    egui::Area::new(egui::Id::new("math_editor_dialog"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            Frame::new()
                .fill(Color32::WHITE)
                .corner_radius(CornerRadius::same(12))
                .stroke(Stroke::new(1.0, Color32::from_gray(200)))
                .inner_margin(Margin::same(20))
                .show(ui, |ui| {
                    ui.set_width(400.0);
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Edit Equation")
                                    .size(16.0)
                                    .strong()
                                    .color(Color32::from_gray(30)),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if default_btn(ui, "X") {
                                        close = true;
                                    }
                                },
                            );
                        });

                        ui.add_space(12.0);

                        ui.label(
                            egui::RichText::new("LaTeX:")
                                .size(12.0)
                                .color(Color32::from_gray(60)),
                        );
                        ui.add_space(4.0);

                        let text_edit = egui::TextEdit::multiline(latex_input)
                            .desired_width(f32::INFINITY)
                            .desired_rows(3)
                            .font(egui::TextStyle::Monospace);
                        let response = ui.add(text_edit);

                        // Request focus on first frame
                        response.request_focus();

                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.small_button("Copy").clicked() {
                                #[cfg(not(target_arch = "wasm32"))]
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    let _ = cb.set_text(latex_input.as_str());
                                }
                                #[cfg(target_arch = "wasm32")]
                                file_ops::copy_text_to_clipboard(latex_input);
                            }
                            if ui.small_button("Paste").clicked() {
                                #[cfg(not(target_arch = "wasm32"))]
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    if let Ok(text) = cb.get_text() {
                                        *latex_input = text;
                                    }
                                }
                                #[cfg(target_arch = "wasm32")]
                                file_ops::request_clipboard_text_for_math();
                            }
                        });

                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Examples: x^2, \\frac{a}{b}, \\sqrt{x}, \\sum_{i=1}^n",
                            )
                            .size(11.0)
                            .color(Color32::from_gray(120)),
                        );

                        ui.add_space(16.0);

                        ui.horizontal(|ui| {
                            if primary_btn(ui, "Apply") {
                                let latex: String = latex_input.clone();
                                action = Some(UiAction::UpdateMathLatex(shape_id, latex));
                                close = true;
                            }
                            ui.add_space(8.0);
                            if default_btn(ui, "Cancel") {
                                close = true;
                            }
                        });
                    });
                });
        });

    if close {
        ui_state.math_editor = None;
    }

    action
}

/// Settings window: keyboard shortcuts, pen feel, and tablet buttons and dials.
fn render_settings(ctx: &Context, ui_state: &mut UiState) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_stroke_color() {
        let shade = |i: usize| TAILWIND_COLORS[i].shades[6];
        let last = TAILWIND_COLORS.len() - 1;
        assert_eq!(cycle_stroke_color(shade(11), 1), shade(12));
        assert_eq!(cycle_stroke_color(shade(11), -1), shade(10));
        assert_eq!(cycle_stroke_color(shade(last), 1), shade(0));
        assert_eq!(cycle_stroke_color(shade(0), -1), shade(last));
        let custom = Color32::from_rgb(1, 2, 3);
        assert_eq!(cycle_stroke_color(custom, 1), shade(0));
        assert_eq!(cycle_stroke_color(custom, -1), shade(last));
    }
}
