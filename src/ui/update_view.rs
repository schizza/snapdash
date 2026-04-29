use crate::theme::Palette;
use crate::ui::icon::Icon;
use crate::ui::theme::UiTheme;
use crate::update::UpdateState;

const UPDATE_ICON_SIZE: f32 = 12.0;

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
