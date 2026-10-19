//! Actions queued by keyboard shortcuts and tablet controls, resolved
//! against the UI state like clicking the matching control.

use super::*;

/// Widths that "thicker" and "thinner" step through: the presets, then
/// wider ones up to the end of the slider.
const STROKE_WIDTH_STEPS: [f32; 13] = [
    1.0, 1.25, 1.5, 1.75, 2.0, 4.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0,
];

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
        Command::NewDocument => ui(UiAction::NewDocument),
        Command::Save => ui(UiAction::Save),
        Command::SaveAs => ui(UiAction::SaveAs),
        Command::CloseTab => ui(UiAction::CloseActiveTab),
        Command::NextTab => ui(UiAction::NextTab),
        Command::PreviousTab => ui(UiAction::PreviousTab),
        #[cfg(not(target_arch = "wasm32"))]
        Command::Open => ui(UiAction::OpenFile),
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
