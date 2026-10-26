use iced::{Color, Font, Theme, font, theme::Palette};
use vrx::config::{Appearance, Config, FontFamily};
pub fn hex(value: &str) -> Color {
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
            warning: hex("F2CB75"),
            danger: hex("F48383"),
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
