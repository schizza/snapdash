use crate::theme::ThemeKind;
use iced::{Color, Theme};

#[derive(Clone, Copy)]
pub struct UiTheme {
    pub palette: crate::theme::Palette,
}

impl From<&ThemeKind> for UiTheme {
    fn from(theme: &ThemeKind) -> Self {
        Self {
            palette: theme.palette(),
        }
    }
}

/// Styl pro "ikonové" tlačítko v pravém horním rohu (gear apod.)
pub fn icon_button<'a>(
    ui_theme: UiTheme,
    alpha: f32,
) -> impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'a {
    let p = ui_theme.palette;
    move |_theme: &Theme, _status: iced::widget::button::Status| iced::widget::button::Style {
        background: None,
        border: iced::Border {
            radius: 12.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: iced::Shadow::default(),
        text_color: Color {
            a: alpha,
            ..p.text_primary
        },
        ..Default::default()
    }
}

/// Styl pro text uvnitř ikonového tlačítka (pokud ho chceš zdůraznit jinak než default text)
pub fn icon_text<'a>(
    ui_theme: UiTheme,
    alpha: f32,
) -> impl Fn(&Theme) -> iced::widget::text::Style + 'a {
    let p = ui_theme.palette;
    move |_theme: &Theme| iced::widget::text::Style {
        color: Some(Color {
            a: alpha,
            ..p.text_primary
        }),
    }
}
