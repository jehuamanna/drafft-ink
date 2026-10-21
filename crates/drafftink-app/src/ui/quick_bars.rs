//! Icon-only radial menu at the pointer.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Selection,
    Tools,
    Stroke,
    Fill,
    Width,
}

#[derive(Debug, Clone, Copy)]
pub struct QuickBars {
    pub anchor: Option<Pos2>,
    pub opened_at: f64,
    page: Page,
}
impl QuickBars {
    pub fn new(now: f64) -> Self {
        Self {
            anchor: None,
            opened_at: now,
            page: Page::Tools,
        }
    }
}

struct Item {
    icon: Option<ImageSource<'static>>,
    color: Option<Color32>,
    width: Option<f32>,
    tooltip: String,
    selected: bool,
    action: UiAction,
}
impl Item {
    fn icon(icon: ImageSource<'static>, tooltip: &str, selected: bool, action: UiAction) -> Self {
        Self {
            icon: Some(icon),
            color: None,
            width: None,
            tooltip: tooltip.into(),
            selected,
            action,
        }
    }
}

fn ring_position(center: Pos2, radius: f32, index: usize, count: usize) -> Pos2 {
    let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * index as f32 / count as f32;
    center + Vec2::angled(angle) * radius
}

fn button(ui: &mut egui::Ui, id: egui::Id, center: Pos2, size: f32, item: &Item) -> bool {
    let rect = egui::Rect::from_center_size(center, Vec2::splat(size));
    let response = ui.interact(rect, id, egui::Sense::click());
    let fill = if item.selected {
        Color32::from_rgb(219, 234, 254)
    } else if response.hovered() {
        Color32::from_gray(233)
    } else {
        Color32::from_rgb(253, 253, 255)
    };
    ui.painter().circle_filled(center, size / 2.0, fill);
    ui.painter().circle_stroke(
        center,
        size / 2.0,
        Stroke::new(
            1.0_f32,
            if item.selected {
                Color32::from_rgb(59, 130, 246)
            } else {
                Color32::from_gray(212)
            },
        ),
    );
    let ink = if matches!(item.action, UiAction::DeleteSelection) {
        Color32::from_rgb(190, 45, 55)
    } else {
        Color32::from_gray(55)
    };
    if let Some(icon) = &item.icon {
        egui::Image::new(icon.clone()).tint(ink).paint_at(
            ui,
            egui::Rect::from_center_size(center, Vec2::splat(size * 0.52)),
        );
    } else if let Some(color) = item.color {
        ui.painter().circle_filled(center, size * 0.27, color);
        ui.painter().circle_stroke(
            center,
            size * 0.27,
            Stroke::new(1.0_f32, Color32::from_gray(180)),
        );
    } else if let Some(width) = item.width {
        ui.painter().line_segment(
            [center - Vec2::new(9.0, 0.0), center + Vec2::new(9.0, 0.0)],
            Stroke::new(width.min(8.0), ink),
        );
    }
    let clicked = response.clicked();
    response.on_hover_text(&item.tooltip);
    clicked
}

pub(super) fn render_quick_bars(
    ctx: &Context,
    state: &mut UiState,
    props: &SelectedShapeProps,
) -> Option<UiAction> {
    let mut bars = state.quick_bars?;
    if bars.anchor.is_none() && props.has_selection {
        bars.page = Page::Selection;
    }
    let anchor = *bars.anchor.get_or_insert_with(|| {
        ctx.input(|i| i.pointer.hover_pos())
            .unwrap_or_else(|| ctx.content_rect().center())
    });
    let mut items = Vec::new();
    match bars.page {
        Page::Selection => {
            items = vec![
                Item::icon(
                    include_image!("../../assets/pan.svg"),
                    "Free movement",
                    props.movement_axis == 0,
                    UiAction::SetMovementAxis(0),
                ),
                Item::icon(
                    include_image!("../../assets/move-x.svg"),
                    "Horizontal movement",
                    props.movement_axis == 1,
                    UiAction::SetMovementAxis(1),
                ),
                Item::icon(
                    include_image!("../../assets/move-y.svg"),
                    "Vertical movement",
                    props.movement_axis == 2,
                    UiAction::SetMovementAxis(2),
                ),
                Item::icon(
                    include_image!("../../assets/scale.svg"),
                    "Proportional scale (Shift)",
                    props.uniform_scale,
                    UiAction::ToggleUniformScale,
                ),
                Item::icon(
                    include_image!("../../assets/flip-h.svg"),
                    "Flip horizontally",
                    false,
                    UiAction::FlipHorizontal,
                ),
                Item::icon(
                    include_image!("../../assets/flip-v.svg"),
                    "Flip vertically",
                    false,
                    UiAction::FlipVertical,
                ),
                Item::icon(
                    include_image!("../../assets/duplicate.svg"),
                    "Duplicate selection",
                    false,
                    UiAction::Duplicate,
                ),
                Item::icon(
                    include_image!("../../assets/delete.svg"),
                    "Delete selection (Delete)",
                    false,
                    UiAction::DeleteSelection,
                ),
            ];
        }
        Page::Tools => {
            for tool in get_tools() {
                items.push(Item::icon(
                    tool.icon,
                    &state
                        .keybindings
                        .with_hint(tool.label, Command::Tool(tool.kind)),
                    state.current_tool == tool.kind,
                    UiAction::SetTool(tool.kind),
                ));
            }
        }
        Page::Stroke | Page::Fill => {
            if bars.page == Page::Fill {
                items.push(Item::icon(
                    include_image!("../../assets/no-fill.svg"),
                    "No fill",
                    state.fill_color.is_none(),
                    UiAction::SetFillColor(None),
                ));
            }
            let colors = QUICK_COLORS
                .iter()
                .map(|&i| {
                    (
                        TAILWIND_COLORS[i].shades[if bars.page == Page::Fill { 1 } else { 6 }],
                        TAILWIND_COLORS[i].name,
                    )
                })
                .chain([(Color32::BLACK, "Black"), (Color32::WHITE, "White")]);
            for (color, name) in colors {
                let fill = bars.page == Page::Fill;
                items.push(Item {
                    icon: None,
                    color: Some(color),
                    width: None,
                    tooltip: name.into(),
                    selected: if fill {
                        state.fill_color == Some(color)
                    } else {
                        state.stroke_color == color
                    },
                    action: if fill {
                        UiAction::SetFillColor(Some(color))
                    } else {
                        UiAction::SetStrokeColor(color)
                    },
                });
            }
        }
        Page::Width => {
            for (width, name) in STROKE_WIDTH_PRESETS {
                items.push(Item {
                    icon: None,
                    color: None,
                    width: Some(width),
                    tooltip: format!("{name} ({width} px)"),
                    selected: (state.stroke_width - width).abs() < 0.01,
                    action: UiAction::SetStrokeWidth(width),
                });
            }
        }
    }
    let diameter = 272.0;
    let screen = ctx.content_rect();
    let top_left = Pos2::new(
        (anchor.x - diameter / 2.0)
            .clamp(screen.min.x, (screen.max.x - diameter).max(screen.min.x)),
        (anchor.y - diameter / 2.0)
            .clamp(screen.min.y, (screen.max.y - diameter).max(screen.min.y)),
    );
    let mut action = None;
    let mut close = false;
    egui::Area::new(egui::Id::new("quick_radial"))
        .order(egui::Order::Foreground)
        .fixed_pos(top_left)
        .movable(false)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(diameter), egui::Sense::hover());
            let center = rect.center();
            let t = ((ctx.input(|i| i.time) - bars.opened_at) / 0.12).clamp(0.0, 1.0) as f32;
            ui.set_opacity(t);
            if t < 1.0 {
                ctx.request_repaint();
            }
            ui.painter().circle_filled(
                center,
                132.0,
                Color32::from_rgba_unmultiplied(248, 249, 252, 242),
            );
            ui.painter().circle_stroke(
                center,
                132.0,
                Stroke::new(1.0_f32, Color32::from_gray(220)),
            );
            for (index, item) in items.iter().enumerate() {
                if button(
                    ui,
                    ui.id().with(("item", index)),
                    ring_position(center, 105.0, index, items.len()),
                    36.0,
                    item,
                ) {
                    action = Some(item.action.clone());
                }
            }
            let pages = [
                (
                    Page::Selection,
                    include_image!("../../assets/select.svg"),
                    "Selection",
                ),
                (
                    Page::Tools,
                    include_image!("../../assets/freehand.svg"),
                    "Tools",
                ),
                (
                    Page::Stroke,
                    include_image!("../../assets/palette.svg"),
                    "Stroke color",
                ),
                (
                    Page::Fill,
                    include_image!("../../assets/fill.svg"),
                    "Fill color",
                ),
                (
                    Page::Width,
                    include_image!("../../assets/width.svg"),
                    "Stroke width",
                ),
            ];
            for (index, (page, icon, label)) in pages.into_iter().enumerate() {
                if page == Page::Selection && !props.has_selection {
                    continue;
                }
                let item = Item::icon(
                    icon,
                    label,
                    bars.page == page,
                    UiAction::SetTool(state.current_tool),
                );
                if button(
                    ui,
                    ui.id().with(("page", index)),
                    ring_position(center, 47.0, index, 5),
                    32.0,
                    &item,
                ) {
                    bars.page = page;
                }
            }
            let item = Item::icon(
                include_image!("../../assets/close.svg"),
                "Close (Esc)",
                false,
                UiAction::SetTool(state.current_tool),
            );
            close = button(ui, ui.id().with("close"), center, 24.0, &item);
        });
    let dismiss = close
        || matches!(
            action,
            Some(UiAction::SetTool(_) | UiAction::DeleteSelection | UiAction::Duplicate)
        );
    state.quick_bars = if dismiss { None } else { Some(bars) };
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radial_delete_click_dispatches_and_closes() {
        let ctx = Context::default();
        let mut state = UiState::default();
        state.quick_bars = Some(QuickBars {
            anchor: Some(Pos2::new(300.0, 300.0)),
            opened_at: 0.0,
            page: Page::Selection,
        });
        let props = SelectedShapeProps {
            has_selection: true,
            selection_count: 2,
            ..Default::default()
        };
        let position = ring_position(Pos2::new(300.0, 300.0), 105.0, 7, 8);
        let mut action = None;
        for (time, events) in [
            (1.0, vec![]),
            (1.1, vec![]),
            (
                1.2,
                vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
            ),
            (
                1.3,
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            ),
        ] {
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(Pos2::ZERO, Vec2::splat(600.0))),
                time: Some(time),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input, |ctx| {
                action = render_quick_bars(ctx, &mut state, &props);
            });
        }
        assert_eq!(action, Some(UiAction::DeleteSelection));
        assert!(state.quick_bars.is_none());
    }
}
