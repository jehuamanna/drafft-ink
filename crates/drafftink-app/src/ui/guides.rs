//! Handwriting and calligraphy guide lines: their editor and grid icon.

use super::*;

/// Controls for the handwriting and calligraphy guide lines.
pub(super) fn writing_guides_editor(
    ui: &mut egui::Ui,
    guides: &mut WritingGuides,
    stroke_width: f32,
) {
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

/// The grid button icon for ruled writing lines: the baseline darker;
/// handwriting has a dashed midline, calligraphy a fourth (descender) line.
pub(super) fn draw_ruled_icon(ui: &egui::Ui, style: GridStyle, center: Pos2, icon_color: Color32) {
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

/// The grid style menu, under the grid button: the styles, and the guide
/// lines editor for writing guides.
pub(super) fn grid_menu(response: &egui::Response, ui_state: &mut UiState) -> Option<UiAction> {
    let mut action = None;
    egui::Popup::menu(response)
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
                writing_guides_editor(ui, &mut ui_state.writing_guides, ui_state.stroke_width);
            }
        });
    action
}
