use crate::{engine::Timing, text::orp::Strategy};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Appearance {
    #[default]
    Dark,
    Light,
    Contrast,
    Custom,
}
impl Appearance {
    pub const ALL: [Self; 4] = [Self::Dark, Self::Light, Self::Contrast, Self::Custom];
}
impl std::fmt::Display for Appearance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum FontFamily {
    #[default]
    Sans,
    Serif,
    Mono,
    Georgia,
    Arial,
}
impl FontFamily {
    pub const ALL: [Self; 5] = [
        Self::Sans,
        Self::Serif,
        Self::Mono,
        Self::Georgia,
        Self::Arial,
    ];
}
impl std::fmt::Display for FontFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Color-independent guide marks placed above and below the focal grapheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GuideShape {
    #[default]
    Ticks,
    Dots,
    Lines,
}
impl GuideShape {
    pub const ALL: [Self; 3] = [Self::Ticks, Self::Dots, Self::Lines];
}
impl std::fmt::Display for GuideShape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub timing: Timing,
    pub orp: Strategy,
    pub font: FontFamily,
    pub font_size: f32,
    pub bold: bool,
    pub appearance: Appearance,
    pub foreground: String,
    pub background: String,
    /// Opaque export canvas, independent of the reader theme.
    pub export_background: String,
    pub accent: String,
    pub warning: String,
    pub guide: bool,
    pub focal_enabled: bool,
    /// Empty follows the theme accent; otherwise six RGB hex digits.
    pub focal_color: String,
    pub guide_shape: GuideShape,
    pub guide_size_percent: u32,
    /// Gap from the shaped line box, as a percentage of reading font size.
    pub guide_gap_percent: u32,
    pub gui_text_size: f32,
    pub radix: u8,
    pub numeral_alphabet: String,
    pub ternary_alphabet: String,
    pub countdown_short: String,
    pub countdown_long: String,
    pub side_opacity_percent: u32,
    pub keys: crate::keybinds::Keymap,
    pub vertical: f32,
    pub focal_x: f32,
    pub jump_words: u32,
    pub speed_step: u32,
    pub diagnostics: bool,
    pub minimal_playback: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            timing: Timing::default(),
            orp: Strategy::Classic,
            font: FontFamily::Sans,
            font_size: 76.0,
            bold: false,
            appearance: Appearance::Dark,
            foreground: "E9EDF4".into(),
            background: "10151D".into(),
            export_background: "00FF00".into(),
            accent: "FFAD7D".into(),
            warning: "F2CB75".into(),
            guide: true,
            focal_enabled: true,
            focal_color: String::new(),
            guide_shape: GuideShape::Ticks,
            guide_size_percent: 100,
            guide_gap_percent: 12,
            gui_text_size: 16.0,
            radix: 10,
            numeral_alphabet: crate::numerals::DIGITS.into(),
            ternary_alphabet: "T01".into(),
            countdown_short: "{seconds}s".into(),
            countdown_long: "{minutes}:{seconds:02}".into(),
            side_opacity_percent: 100,
            keys: crate::keybinds::Keymap::default(),
            vertical: 0.46,
            focal_x: 0.45,
            jump_words: 10,
            speed_step: 25,
            diagnostics: false,
            minimal_playback: false,
        }
    }
}
impl Config {
    pub fn sanitize(&mut self) {
        self.timing.sanitize();
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(20.0, 360.0)
        } else {
            76.0
        };
        self.vertical = if self.vertical.is_finite() {
            self.vertical.clamp(0.02, 0.98)
        } else {
            0.46
        };
        self.focal_x = if self.focal_x.is_finite() {
            self.focal_x.clamp(0.02, 0.98)
        } else {
            0.45
        };
        self.guide_size_percent = self.guide_size_percent.clamp(25, 400);
        self.guide_gap_percent = self.guide_gap_percent.min(100);
        if !self.focal_color.is_empty() && self.focal_color_rgb().is_none() {
            self.focal_color.clear();
        }
        self.jump_words = self.jump_words.clamp(1, 100_000);
        self.speed_step = self.speed_step.clamp(1, 1000);
        self.radix = self.radix.clamp(2, 60);
        self.side_opacity_percent = self.side_opacity_percent.min(100);
        if crate::numerals::validate_alphabet(&self.numeral_alphabet, usize::from(self.radix), 60)
            .is_err()
        {
            self.numeral_alphabet = crate::numerals::DIGITS.into();
        }
        if crate::numerals::validate_alphabet(&self.ternary_alphabet, 3, 3).is_err() {
            self.ternary_alphabet = "T01".into();
        }
        if crate::countdown::validate(&self.countdown_short).is_err() {
            self.countdown_short = "{seconds}s".into();
        }
        if crate::countdown::validate(&self.countdown_long).is_err() {
            self.countdown_long = "{minutes}:{seconds:02}".into();
        }
        if !valid_rgb(&self.warning) {
            self.warning = "F2CB75".into();
        }
        self.gui_text_size = if self.gui_text_size.is_finite() {
            self.gui_text_size.clamp(12.0, 32.0)
        } else {
            16.0
        };
        if self.export_background.len() != 6
            || !self
                .export_background
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
        {
            self.export_background = "00FF00".into();
        }
        for color in [&mut self.foreground, &mut self.background, &mut self.accent] {
            if color.len() != 6 || !color.bytes().all(|b| b.is_ascii_hexdigit()) {
                *color = "FFFFFF".into();
            }
        }
    }
    pub fn focal_color_rgb(&self) -> Option<&str> {
        let color = self.focal_color.as_str();
        (color.len() == 6 && color.bytes().all(|b| b.is_ascii_hexdigit())).then_some(color)
    }
    pub fn notation(&self) -> crate::numerals::CustomNotation<'_> {
        crate::numerals::CustomNotation {
            radix: self.radix,
            alphabet: &self.numeral_alphabet,
            ternary: &self.ternary_alphabet,
        }
    }
    pub fn gui_scale(&self) -> f32 {
        self.gui_text_size / 16.0
    }
}

pub fn valid_rgb(color: &str) -> bool {
    color.len() == 6 && color.bytes().all(|b| b.is_ascii_hexdigit())
}
