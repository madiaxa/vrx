use iced::{Color, Font, Theme, font, theme::Palette};
use vrx::config::{Appearance, Config, FontFamily};
pub fn hex(value: &str) -> Color {
    if !vrx::config::valid_rgb(value) {
        return Color::WHITE;
    }
    u32::from_str_radix(value, 16)
        .map(|n| Color::from_rgb8((n >> 16) as u8, (n >> 8) as u8, n as u8))
        .unwrap_or(Color::WHITE)
}
pub fn palette(config: &Config) -> (Color, Color, Color) {
    match config.appearance {
        Appearance::Dark => (hex("10151D"), hex("E9EDF4"), hex("FFAD7D")),
        Appearance::Light => (hex("F6F4EF"), hex("20252D"), hex("A43D19")),
        Appearance::Contrast => (Color::BLACK, Color::WHITE, hex("FFFF00")),
        Appearance::Custom => (
            hex(&config.background),
            hex(&config.foreground),
            hex(&config.accent),
        ),
    }
}
pub fn focal_color(config: &Config) -> Color {
    config
        .focal_color_rgb()
        .map(hex)
        .unwrap_or_else(|| palette(config).2)
}
pub fn theme(config: &Config) -> Theme {
    let (background, text, primary) = palette(config);
    Theme::custom(
        "VRX",
        Palette {
            background,
            text,
            primary,
            success: hex("6FCEAC"),
            warning: hex(&config.warning),
            danger: hex(&config.warning),
        },
    )
}
pub fn font(config: &Config) -> Font {
    Font {
        family: match config.font {
            FontFamily::Sans => font::Family::SansSerif,
            FontFamily::Serif => font::Family::Serif,
            FontFamily::Mono => font::Family::Monospace,
            FontFamily::Georgia => font::Family::Name("Georgia"),
            FontFamily::Arial => font::Family::Name("Arial"),
        },
        weight: if config.bold {
            font::Weight::Bold
        } else {
            font::Weight::Normal
        },
        ..Font::DEFAULT
    }
}

pub fn warning_button(
    config: &Config,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let color = hex(&config.warning);
    let brightness = 0.2126 * color.r + 0.7152 * color.g + 0.0722 * color.b;
    iced::widget::button::Style {
        background: Some(
            color
                .scale_alpha(if status == iced::widget::button::Status::Hovered {
                    0.8
                } else {
                    1.0
                })
                .into(),
        ),
        text_color: if brightness > 0.5 {
            Color::BLACK
        } else {
            Color::WHITE
        },
        ..Default::default()
    }
}

/// A dedicated override remains effective in preset themes as well as Custom.
pub fn dropdown_accent(config: &Config) -> Color {
    if vrx::config::valid_rgb(&config.dropdown_color) {
        hex(&config.dropdown_color)
    } else {
        palette(config).2
    }
}
pub fn dropdown_button(
    config: &Config,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let accent = dropdown_accent(config);
    let (background, foreground, _) = palette(config);
    let luma = |c: Color| 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
    let text = if (luma(accent) - luma(background)).abs() < 0.3 {
        foreground
    } else {
        accent
    };
    let alpha = match status {
        iced::widget::button::Status::Hovered => 0.20,
        iced::widget::button::Status::Pressed => 0.28,
        _ => 0.12,
    };
    iced::widget::button::Style {
        background: Some(accent.scale_alpha(alpha).into()),
        text_color: text,
        border: iced::Border {
            color: accent.scale_alpha(0.5),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
pub fn dropdown_panel(config: &Config) -> iced::widget::container::Style {
    let accent = dropdown_accent(config);
    iced::widget::container::Style {
        background: Some(accent.scale_alpha(0.035).into()),
        border: iced::Border {
            color: accent.scale_alpha(0.22),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
