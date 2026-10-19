//! Quick toolbars: tools, brushes and properties shown at the pointer.

use super::*;

/// Tools and colours shown next to the pointer ([`Command::QuickToolbars`]).
#[derive(Debug, Clone, Copy)]
pub struct QuickBars {
    /// Pointer position when opened, in screen points (set on the first frame).
    pub anchor: Option<Pos2>,
    /// When it opened, in egui time (seconds).
    pub opened_at: f64,
}

impl QuickBars {
    pub fn new(now: f64) -> Self {
        Self {
            anchor: None,
            opened_at: now,
        }
    }
}

/// Something picked from the quick toolbars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuickItem {
    Tool(ToolKind),
    Color(Color32),
    Fill(Option<Color32>),
    Width(f32),
}

impl QuickItem {
    pub fn action(self) -> UiAction {
        match self {
            QuickItem::Tool(tool) => UiAction::SetTool(tool),
            QuickItem::Color(color) => UiAction::SetStrokeColor(color),
            QuickItem::Fill(color) => UiAction::SetFillColor(color),
            QuickItem::Width(width) => UiAction::SetStrokeWidth(width),
        }
    }
}

/// Render the quick toolbars around the pointer: tools in a column to its
/// left, fill, stroke width and stroke colours above it, and the properties
/// panel to its right, so everything is a short move away. The items most
/// used sit nearest the pointer (the pen tools and the quick colours).
/// Picking a tool closes them; style changes keep them open.
pub(super) fn render_quick_bars(
    ctx: &Context,
    ui_state: &mut UiState,
    props: &SelectedShapeProps,
) -> Option<UiAction> {
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
    let fills: Vec<(Color32, &str)> = QUICK_COLORS
        .iter()
        .map(|&i| (TAILWIND_COLORS[i].shades[1], TAILWIND_COLORS[i].name))
        .chain(
            QUICK_COLORS
                .iter()
                .map(|&i| (TAILWIND_COLORS[i].shades[6], TAILWIND_COLORS[i].name)),
        )
        .collect();
    let section = |ui: &mut egui::Ui, text: &str| {
        ui.label(
            egui::RichText::new(text)
                .size(10.0)
                .color(Color32::from_gray(120)),
        );
    };
    egui::Area::new(egui::Id::new("quick_colors"))
        .order(egui::Order::Foreground)
        .pivot(Align2::LEFT_BOTTOM)
        .fixed_pos(anchor + Vec2::new(-GAP + 2.0, -GAP + slide))
        .constrain(true)
        .show(ctx, |ui| {
            ui.set_opacity(ease);
            panel_frame().show(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                let mut item = |quick: QuickItem, clicked: bool| {
                    if clicked {
                        action = Some(quick);
                    }
                };

                section(ui, "Fill");
                ui.horizontal(|ui| {
                    let response = ui.scope(|ui| {
                        NoColorSwatch::new("No fill")
                            .selected(ui_state.fill_color.is_none())
                            .show(ui)
                    });
                    item(QuickItem::Fill(None), response.inner);
                    for &(color, name) in &fills {
                        let (clicked, _) = ColorSwatch::new(color, name)
                            .selected(ui_state.fill_color == Some(color))
                            .show(ui);
                        item(QuickItem::Fill(Some(color)), clicked);
                    }
                });

                section(ui, "Stroke width");
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                    for &(width, name) in STROKE_WIDTH_PRESETS.iter() {
                        let tooltip = format!("{name} ({width} px)");
                        let selected = (ui_state.stroke_width - width).abs() < 0.01;
                        let response = ui
                            .scope(|ui| StrokeWidthButton::new(width, &tooltip, selected).show(ui));
                        item(QuickItem::Width(width), response.inner);
                    }
                });

                section(ui, "Stroke");
                for row in &rows {
                    ui.horizontal(|ui| {
                        for &(color, name) in *row {
                            let (clicked, _) = ColorSwatch::new(color, name)
                                .style(drafftink_widgets::ColorSwatchStyle::large())
                                .selected(ui_state.stroke_color == color)
                                .show(ui);
                            item(QuickItem::Color(color), clicked);
                        }
                    });
                }
            });
        });

    // Properties of the selection or tool, to the right of the pointer.
    let mut props_action = None;
    if props.has_selection || props.is_drawing_tool {
        egui::Area::new(egui::Id::new("quick_properties"))
            .order(egui::Order::Foreground)
            .pivot(Align2::LEFT_TOP)
            .fixed_pos(anchor + Vec2::new(GAP - slide, -4.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.set_opacity(ease);
                panel_frame().show(ui, |ui| {
                    ui.set_width(176.0);
                    props_action = properties_contents(ui, props);
                });
            });
    }

    // Picking a tool closes the toolbars (you'll draw next); colours, widths
    // and properties keep them open so several can be set in one go.
    let picked_tool = matches!(action, Some(QuickItem::Tool(_)));
    ui_state.quick_bars = if picked_tool { None } else { Some(bars) };
    action.map(QuickItem::action).or(props_action)
}
