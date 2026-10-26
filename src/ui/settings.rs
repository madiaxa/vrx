use crate::{
    message::{Message, Setting},
    ui::anchor::AnchorPad,
};
use iced::{
    Element, Font, Length, font,
    widget::{
        button, checkbox, column, container, pick_list, row, scrollable, slider, text, text_input,
    },
};
use std::collections::BTreeMap;
use vrx::text::orp::Strategy;
use vrx::{
    config::{Appearance, Config, FontFamily, GuideShape},
    engine::Overrun,
    keybinds::Action,
};

pub const NUMBER_FONT: Font = Font {
    family: font::Family::Name("Clockette Mono"),
    ..Font::DEFAULT
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Number {
    Wpm,
    Jump,
    Step,
    Size,
    GuiSize,
    GuideSize,
    GuideGap,
    Long,
    Comma,
    Colon,
    Semicolon,
    Sentence,
    Paragraph,
    Quote,
    Parenthesis,
    Dash,
    Section,
    X,
    Y,
}
impl Number {
    pub const ALL: [Self; 19] = [
        Self::Wpm,
        Self::Jump,
        Self::Step,
        Self::Size,
        Self::GuiSize,
        Self::GuideSize,
        Self::GuideGap,
        Self::Long,
        Self::Comma,
        Self::Colon,
        Self::Semicolon,
        Self::Sentence,
        Self::Paragraph,
        Self::Quote,
        Self::Parenthesis,
        Self::Dash,
        Self::Section,
        Self::X,
        Self::Y,
    ];
    pub fn scale(self) -> u32 {
        if matches!(
            self,
            Self::Long
                | Self::Comma
                | Self::Colon
                | Self::Semicolon
                | Self::Sentence
                | Self::Paragraph
                | Self::Quote
                | Self::Parenthesis
                | Self::Dash
                | Self::X
                | Self::Y
        ) {
            1000
        } else {
            1
        }
    }
    pub fn bounds(self) -> (u32, u32) {
        match self {
            Self::Wpm => (20, 10000),
            Self::Jump => (1, 100000),
            Self::Step => (1, 1000),
            Self::Size => (20, 360),
            Self::GuiSize => (12, 32),
            Self::GuideSize => (25, 400),
            Self::GuideGap => (0, 100),
            Self::Section => (0, 10000),
            Self::X | Self::Y => (2000, 98000),
            _ => (1000, 5000),
        }
    }
    pub fn value(self, c: &Config) -> u32 {
        match self {
            Self::Wpm => c.timing.wpm,
            Self::Jump => c.jump_words,
            Self::Step => c.speed_step,
            Self::Size => c.font_size.round() as u32,
            Self::GuiSize => c.gui_text_size.round() as u32,
            Self::GuideSize => c.guide_size_percent,
            Self::GuideGap => c.guide_gap_percent,
            Self::Long => c.timing.long,
            Self::Comma => c.timing.comma,
            Self::Colon => {
                if c.timing.colon == 0 {
                    c.timing.clause
                } else {
                    c.timing.colon
                }
            }
            Self::Semicolon => {
                if c.timing.semicolon == 0 {
                    c.timing.clause
                } else {
                    c.timing.semicolon
                }
            }
            Self::Sentence => c.timing.sentence,
            Self::Paragraph => c.timing.paragraph,
            Self::Quote => c.timing.quote,
            Self::Parenthesis => c.timing.parenthesis,
            Self::Dash => c.timing.dash,
            Self::Section => c.timing.section_pause_ms,
            Self::X => (c.focal_x * 100000.0).round() as u32,
            Self::Y => (c.vertical * 100000.0).round() as u32,
        }
    }
    pub fn apply(self, c: &mut Config, value: u32) {
        match self {
            Self::Wpm => c.timing.wpm = value,
            Self::Jump => c.jump_words = value,
            Self::Step => c.speed_step = value,
            Self::Size => c.font_size = value as f32,
            Self::GuiSize => c.gui_text_size = value as f32,
            Self::GuideSize => c.guide_size_percent = value,
            Self::GuideGap => c.guide_gap_percent = value,
            Self::Long => c.timing.long = value,
            Self::Comma => c.timing.comma = value,
            Self::Colon => c.timing.colon = value,
            Self::Semicolon => c.timing.semicolon = value,
            Self::Sentence => c.timing.sentence = value,
            Self::Paragraph => c.timing.paragraph = value,
            Self::Quote => c.timing.quote = value,
            Self::Parenthesis => c.timing.parenthesis = value,
            Self::Dash => c.timing.dash = value,
            Self::Section => c.timing.section_pause_ms = value,
            Self::X => c.focal_x = value as f32 / 100000.0,
            Self::Y => c.vertical = value as f32 / 100000.0,
        }
    }
    fn slider_denominator(self, c: &Config) -> u32 {
        if self.scale() > 1 {
            u32::from(c.notation().base())
        } else {
            1
        }
    }
    pub fn slider_range(self, c: &Config) -> std::ops::RangeInclusive<u32> {
        let (min, max) = self.bounds();
        let denominator = u64::from(self.slider_denominator(c));
        let scale = u64::from(self.scale());
        ((u64::from(min) * denominator).div_ceil(scale) as u32)
            ..=((u64::from(max) * denominator) / scale) as u32
    }
    pub fn slider_value(self, c: &Config) -> u32 {
        let denominator = u64::from(self.slider_denominator(c));
        let scale = u64::from(self.scale());
        ((u64::from(self.value(c)) * denominator + scale / 2) / scale) as u32
    }
    pub fn slider_quantity(self, c: &Config, tick: u32) -> u32 {
        let range = self.slider_range(c);
        let tick = tick.clamp(*range.start(), *range.end());
        let denominator = u64::from(self.slider_denominator(c));
        let scale = u64::from(self.scale());
        ((u64::from(tick) * scale + denominator / 2) / denominator) as u32
    }
    pub fn formatted(self, c: &Config) -> String {
        c.notation().scaled(self.value(c), self.scale())
    }
}
#[derive(Debug)]
pub struct Fields {
    pub numbers: BTreeMap<Number, String>,
    pub bindings: BTreeMap<Action, String>,
    pub notice: Option<String>,
    pub radix: String,
}
impl Fields {
    pub fn new(c: &Config) -> Self {
        Self {
            numbers: Number::ALL
                .into_iter()
                .map(|n| (n, n.formatted(c)))
                .collect(),
            bindings: Action::ALL
                .into_iter()
                .map(|a| (a, c.keys.label(a).into()))
                .collect(),
            notice: None,
            radix: c.radix.to_string(),
        }
    }
    pub fn get(&self, n: Number) -> &str {
        self.numbers.get(&n).map(String::as_str).unwrap_or("")
    }
    pub fn binding(&self, a: Action) -> &str {
        self.bindings.get(&a).map(String::as_str).unwrap_or("")
    }
    pub fn refresh(&mut self, n: Number, c: &Config) {
        self.numbers.insert(n, n.formatted(c));
    }
}
fn number<'a>(
    c: &Config,
    fields: &'a Fields,
    n: Number,
    label: &'static str,
) -> Element<'a, Message> {
    let (min, max) = n.bounds();
    let hint = format!(
        "{}–{}",
        c.notation().scaled(min, n.scale()),
        c.notation().scaled(max, n.scale())
    );
    row![
        column![text(label).size(13), text(hint).font(NUMBER_FONT).size(10)].width(Length::Fill),
        text_input("", fields.get(n))
            .font(NUMBER_FONT)
            .size(14)
            .on_input(move |s| Message::NumberInput(n, s))
            .on_submit(Message::NumberApply(n))
            .width(152)
    ]
    .spacing(10)
    .align_y(iced::Center)
    .into()
}
pub fn view<'a>(
    c: &'a Config,
    fields: &'a Fields,
    listening: Option<Action>,
) -> Element<'a, Message> {
    let color = |label: &'static str, value: &str, change: fn(String) -> Setting| {
        row![
            text(label).size(13).width(Length::Fill),
            text_input("RGB hex", value)
                .on_input(move |s| Message::Set(change(s)))
                .width(140)
        ]
        .spacing(10)
    };
    let mut content = column![
        row![
            text("Personalize").size(22).width(Length::Fill),
            button("Done").on_press(Message::Settings)
        ],
        text("Click any number to edit. Enter applies; Done saves.").size(12)
    ]
    .spacing(14)
    .padding(18)
    .width(Length::Fill);
    if let Some(notice) = &fields.notice {
        content = content.push(
            text(notice)
                .size(12)
                .color(iced::Color::from_rgb8(244, 131, 131)),
        );
    }
    content=content.push(text("READING & NUMBERS").size(11))
        .push(row![
            column![text("Radix / base").size(13), text("2–60 · decimal input").size(10)].width(Length::Fill),
            text_input("2–60", &fields.radix)
                .font(NUMBER_FONT)
                .size(14)
                .on_input(Message::RadixInput)
                .on_submit(Message::RadixApply)
                .width(152)
        ].spacing(10).align_y(iced::Center))
        .push(text("Radix selector stays decimal. Ternary uses T, 0, 1. RGB hex, versions, files and API formats retain their own notation.").size(11))
        .push(number(c,fields,Number::Wpm,"Nominal WPM"))
        .push(number(c,fields,Number::Jump,"Words per arrow / jump button"))
        .push(number(c,fields,Number::Step,"WPM step"))
        .push(checkbox(c.timing.adaptive).label("Adaptive timing").on_toggle(|v|Message::Set(Setting::Adaptive(v))));
    for (n, label) in [
        (Number::Long, "Long word ×"),
        (Number::Comma, "Comma ×"),
        (Number::Colon, "Colon ×"),
        (Number::Semicolon, "Semicolon ×"),
        (Number::Sentence, "Sentence ending ×"),
        (Number::Paragraph, "Paragraph ending ×"),
        (Number::Quote, "Quotation marks ×"),
        (Number::Parenthesis, "Parentheses / brackets ×"),
        (Number::Dash, "Em / en dash ×"),
    ] {
        content = content.push(number(c, fields, n, label)).push(
            slider(n.slider_range(c), n.slider_value(c), move |v| {
                Message::NumberSlider(n, v)
            })
            .step(1u32),
        );
    }
    content=content.push(text("Pause sliders move in one fractional digit of the chosen base. Typed values keep thousandth precision.").size(11))
        .push(number(c,fields,Number::Section,"Extra section pause · ms"))
        .push(pick_list([Overrun::Preserve,Overrun::MaintainPace],Some(c.timing.overrun),|v|Message::Set(Setting::Policy(v))))
        .push(text("FOCAL POINT & POSITION").size(11))
        .push(checkbox(c.focal_enabled).label("Enable focal point / ORP highlight").on_toggle(|v|Message::Set(Setting::Focal(v))))
        .push(pick_list(Strategy::ALL,Some(c.orp),|v|Message::Set(Setting::Orp(v))))
        .push(checkbox(c.guide).label("Show focal guide when enabled").on_toggle(|v|Message::Set(Setting::Guide(v))))
        .push(pick_list(GuideShape::ALL,Some(c.guide_shape),|v|Message::Set(Setting::GuideShape(v))))
        .push(color("Focal color · empty = theme",&c.focal_color,Setting::FocalColor))
        .push(number(c,fields,Number::GuideSize,"Guide size · % of default"))
        .push(number(c,fields,Number::GuideGap,"Guide gap · % of reading size"))
        .push(text("Guide marks scale with the reading type and shrink with words that need to fit.").size(11))
        .push(text("Drag the target in this pad or the reading area. Focus mode centers the whole word.").size(12))
        .push(AnchorPad{config:c})
        .push(number(c,fields,Number::X,"Horizontal position · %"))
        .push(number(c,fields,Number::Y,"Vertical position · %"))
        .push(text("TYPOGRAPHY & COLOR").size(11))
        .push(pick_list(FontFamily::ALL,Some(c.font),|v|Message::Set(Setting::Font(v))))
        .push(number(c,fields,Number::Size,"RSVP type size · px"))
        .push(slider(20..=360,Number::Size.value(c),|v|Message::NumberValue(Number::Size,v)))
        .push(number(c,fields,Number::GuiSize,"Interface text size · px"))
        .push(checkbox(c.bold).label("Bold RSVP type").on_toggle(|v|Message::Set(Setting::Bold(v))))
        .push(pick_list(Appearance::ALL,Some(c.appearance),|v|Message::Set(Setting::Appearance(v))))
        .push(color("Custom foreground",&c.foreground,Setting::Foreground))
        .push(color("Custom background",&c.background,Setting::Background))
        .push(color("Custom accent",&c.accent,Setting::Accent))
        .push(text("EXPORT").size(11))
        .push(color("Export background",&c.export_background,Setting::ExportBackground))
        .push(text("RGB hex · 00FF00 is chroma green. Independent of the reading background. Used by MP4 and ASS; SRT has no color.").size(11))
        .push(text("KEYBINDINGS").size(11))
        .push(text("Edit and press Enter, or use Record. Separate alternate shortcuts with a semicolon. Clear an old assignment before reusing it.").size(12));
    for action in Action::ALL {
        content = content.push(text(action.label()).size(12)).push(
            row![
                text_input("Unbound", fields.binding(action))
                    .size(12)
                    .on_input(move |s| Message::BindingInput(action, s))
                    .on_submit(Message::ApplyBinding(action))
                    .width(Length::Fill),
                button(if listening == Some(action) {
                    "Press key…"
                } else {
                    "Record"
                })
                .on_press(Message::RecordBinding(action)),
                button("Clear").on_press(Message::ClearBinding(action))
            ]
            .spacing(6),
        );
    }
    content=content.push(button("Restore default keybindings").on_press(Message::ResetBindings))
        .push(text("BEHAVIOR").size(11))
        .push(checkbox(c.minimal_playback).label("Hide controls during playback").on_toggle(|v|Message::Set(Setting::Minimal(v))))
        .push(checkbox(c.diagnostics).label("Developer diagnostics").on_toggle(|v|Message::Set(Setting::Diagnostics(v))))
        .push(text("Complex scripts use whole-word focus. Native screen-reader support is limited by Iced.").size(11));
    container(scrollable(content))
        .style(container::bordered_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn focused_revision_radix_sliders_and_short_numbers_round_trip() -> vrx::Result<()> {
        let mut c = Config::default();
        for base in 2..=60 {
            c.radix = base;
            for value in [
                0, 1, 500, 1000, 1150, 1300, 1500, 1800, 2000, 5000, 23456, 98000,
            ] {
                let spelling = c.notation().scaled(value, 1000);
                assert_eq!(
                    c.notation().parse_quantity(&spelling, 1000)?,
                    i128::from(value)
                );
            }
            for tick in Number::Comma.slider_range(&c) {
                let value = Number::Comma.slider_quantity(&c, tick);
                Number::Comma.apply(&mut c, value);
                let formatted = Number::Comma.formatted(&c);
                assert_eq!(
                    c.notation().parse_quantity(&formatted, 1000)?,
                    i128::from(value)
                );
                assert_eq!(Number::Comma.slider_value(&c), tick);
                assert!(
                    formatted
                        .split('.')
                        .nth(1)
                        .is_none_or(|s| s.chars().count() <= 1)
                );
            }
        }
        assert_eq!(vrx::numerals::Notation(12).scaled(1083, 1000), "1.1");
        c.guide_size_percent = 175;
        c.guide_gap_percent = 25;
        c.focal_color = "12ABEF".into();
        c.guide_shape = GuideShape::Dots;
        let encoded = serde_json::to_vec(&c).map_err(|e| e.to_string())?;
        let restored: Config = serde_json::from_slice(&encoded).map_err(|e| e.to_string())?;
        assert_eq!(restored.guide_size_percent, 175);
        assert_eq!(restored.guide_gap_percent, 25);
        assert_eq!(restored.guide_shape, GuideShape::Dots);
        assert_eq!(restored.focal_color_rgb(), Some("12ABEF"));
        let defaults: Config = serde_json::from_str("{}").map_err(|e| e.to_string())?;
        assert_eq!(defaults.guide_size_percent, 100);
        Ok(())
    }
}
