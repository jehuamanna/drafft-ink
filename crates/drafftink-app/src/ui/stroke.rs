//! Brush and stroke width controls in the properties panel.

use super::*;

/// Range of the stroke width slider, in px.
const STROKE_WIDTH_RANGE: std::ops::RangeInclusive<f32> = 0.25..=64.0;

/// Brush choice (and nib angle for the broad nib), for the pen tool and
/// selected pen strokes.
pub(super) fn brush_section(ui: &mut egui::Ui, props: &SelectedShapeProps) -> Option<UiAction> {
    let mut action = None;
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
                if ToggleButton::new(brush.name(), current == brush).show(ui) && current != brush {
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
    action
}

/// A slider for any stroke width, also changed by scrolling over it.
pub(super) fn width_slider(ui: &mut egui::Ui, stroke_width: f32) -> Option<UiAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        let mut width = stroke_width;
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
    action
}
