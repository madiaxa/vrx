//! In-window dialogs. Opaque layers consume pointer events; App routes keyboard
//! events only to the active dialog so playback shortcuts cannot leak through.
use crate::{
    message::{ConfirmKind, Message},
    ui::{
        reader::{Mode, Reader},
        theme,
    },
};
use iced::{
    Element, Length, Size,
    widget::{button, column, container, row, scrollable, text, text_input},
};
use vrx::config::Config;

pub fn modal<'a>(
    base: Element<'a, Message>,
    body: Element<'a, Message>,
    size: Size,
) -> Element<'a, Message> {
    let card = container(scrollable(body))
        .padding(20)
        .style(container::bordered_box)
        .width((size.width - 32.0).clamp(240.0, 720.0))
        .max_height((size.height - 32.0).max(140.0));
    let backdrop = container(card)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.6).into()),
            ..Default::default()
        });
    iced::widget::stack![base, iced::widget::opaque(backdrop)].into()
}
pub fn about<'a>() -> Element<'a, Message> {
    column![
        text("VRX").size(32).width(Length::Fill).align_x(iced::Center),
        text(format!("Visual Reader X · {}", vrx::VERSION))
            .size(18).width(Length::Fill).align_x(iced::Center),
        text("Released under the Unlicense (public domain dedication).")
            .width(Length::Fill).align_x(iced::Center),
        text("Third-party libraries retain their own licenses. Axa Mono uses the SIL Open Font License 1.1. Full license texts are included with the application.")
            .size(12).width(Length::Fill).align_x(iced::Center),
        button("Close").on_press(Message::ClosePopup),
    ]
    .spacing(16)
    .width(Length::Fill)
    .align_x(iced::Center)
    .into()
}
pub fn confirm<'a>(kind: ConfirmKind, c: &'a Config) -> Element<'a, Message> {
    let (title, description, label) = match kind {
        ConfirmKind::Defaults => (
            "Restore all defaults?",
            "This replaces your current personalization with VRX's defaults. Your reading text and recent documents are kept.",
            "Restore defaults",
        ),
        ConfirmKind::Discard => (
            "Discard changes?",
            "Delete the changes made since you opened Personalize and restore the previous settings?",
            "Discard changes",
        ),
        ConfirmKind::Keybindings => (
            "Restore default keybindings?",
            "This replaces all custom shortcut assignments. Other settings and reading text are kept.",
            "Restore keybindings",
        ),
    };
    column![
        text(title).size(23),
        text(description),
        row![
            button("Cancel").on_press(Message::ClosePopup),
            button(label)
                .on_press(Message::Confirm)
                .style(move |_, status| theme::warning_button(c, status))
        ]
        .spacing(12)
    ]
    .spacing(18)
    .into()
}
pub fn export<'a>(
    c: &'a Config,
    error: Option<&'a str>,
    picking: bool,
    size: Size,
) -> Element<'a, Message> {
    let rgb = if vrx::config::valid_rgb(&c.export_background) {
        theme::hex(&c.export_background)
    } else {
        iced::Color::BLACK
    };
    let width = ((size.width - 32.0).clamp(240.0, 720.0) - 40.0).min(480.0);
    let thumbnail = container(
        container(Reader::preview(c, "VRX", Mode::Export))
            .width(width)
            .height(width * 9.0 / 16.0)
            .style(move |_| container::Style {
                background: Some(rgb.into()),
                ..Default::default()
            }),
    )
    .center_x(Length::Fill);
    let header = row![
        text("Export reading").size(24).width(Length::Fill),
        button("Cancel").on_press(Message::ClosePopup)
    ];
    let color = row![
        text_input("00FF00", &c.export_background).on_input(Message::ExportBackground),
        container(text(" "))
            .width(32)
            .height(32)
            .style(move |_| container::Style {
                background: Some(rgb.into()),
                ..Default::default()
            })
    ]
    .spacing(10);
    let mut content=column![header,text("Preview · VRX").size(12),thumbnail,text("Background color · RGB hex").size(13),color]
        .spacing(14)
        .push(text("This export's background is independent of your reading theme. Personalize controls the default for future exports.").size(12))
        .push(text("MP4 and ASS include the canvas. SRT is unstyled. The preview uses the output's whole-word positioning; desktop guide marks are not exported.").size(12));
    if let Some(error) = error {
        content = content.push(text(error).color(theme::hex(&c.warning)));
    }
    content = content.push(if picking {
        button("Choosing file…")
    } else {
        button("Choose file and export").on_press(Message::ChooseExport)
    });
    content.into()
}
