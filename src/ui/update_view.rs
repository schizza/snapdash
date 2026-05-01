//! UI helpers for rendering update-related widgets. Lives in `ui/` so the
//! update domain stays free of `Icon`/`Palette` dependencies — this is
//! the bridge that maps a domain `UpdateState` to a styled `Text`.

use crate::theme::Palette;
use crate::ui::icon::Icon;
use crate::ui::theme::UiTheme;
use crate::update::UpdateState;

const UPDATE_ICON_SIZE: f32 = 12.0;

/// Glyph + color combo for the version-status indicator shown in the
/// settings footer. Returned as `Text` so callers can still chain
/// `.size()`, `.color()`, etc. to override the defaults.
pub fn status_icon<'a>(
    state: UpdateState,
    ui_theme: UiTheme,
    p: Palette,
) -> iced::widget::Text<'a> {
    let (icon, color) = match state {
        UpdateState::Unknown => (Icon::Unknown, p.text_dim),
        UpdateState::UptoDate => (Icon::Check, p.success),
        UpdateState::UpdateAvailable => (Icon::Download, p.danger),
    };

    icon.text(ui_theme).size(UPDATE_ICON_SIZE).color(color)
}
