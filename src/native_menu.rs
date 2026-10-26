//! macOS main-thread menu ownership and a bounded, event-driven Iced bridge.
use crate::message::Message;
use iced::{
    Subscription,
    futures::{SinkExt, StreamExt, channel::mpsc},
};
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use std::sync::{Mutex, OnceLock};
#[derive(Clone, Copy)]
enum Action {
    About,
    Preferences,
    Quit,
}
static SIGNAL: OnceLock<Mutex<Option<mpsc::Sender<Action>>>> = OnceLock::new();
pub struct NativeMenu {
    _menu: Menu,
}
pub fn install() -> Result<NativeMenu, String> {
    // Called by App::update after the native window exists, on its main thread.
    let about = MenuItem::new("About VRX", true, None);
    let preferences = MenuItem::new("Personalize…", true, "CmdOrCtrl+,".parse().ok());
    let quit = MenuItem::new("Quit VRX", true, "CmdOrCtrl+Q".parse().ok());
    let app = Submenu::with_items(
        "VRX",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(),
            &preferences,
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &quit,
        ],
    )
    .map_err(|e| e.to_string())?;
    let menu = Menu::with_items(&[&app]).map_err(|e| e.to_string())?;
    let about_id = about.id().clone();
    let preferences_id = preferences.id().clone();
    let quit_id = quit.id().clone();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let action = if event.id == about_id {
            Some(Action::About)
        } else if event.id == preferences_id {
            Some(Action::Preferences)
        } else if event.id == quit_id {
            Some(Action::Quit)
        } else {
            None
        };
        if let Some(action) = action
            && let Ok(mut guard) = SIGNAL.get_or_init(|| Mutex::new(None)).lock()
            && let Some(sender) = guard.as_mut()
        {
            let _ = sender.try_send(action);
        }
    }));
    menu.init_for_nsapp();
    Ok(NativeMenu { _menu: menu })
}
pub fn subscription() -> Subscription<Message> {
    Subscription::run(|| {
        iced::stream::channel(1, |mut output: mpsc::Sender<Message>| async move {
            let (sender, mut receiver) = mpsc::channel(1);
            if let Ok(mut guard) = SIGNAL.get_or_init(|| Mutex::new(None)).lock() {
                *guard = Some(sender);
            }
            while let Some(action) = receiver.next().await {
                let message = match action {
                    Action::About => Message::About,
                    Action::Preferences => Message::Settings,
                    Action::Quit => Message::Quit,
                };
                if output.send(message).await.is_err() {
                    break;
                }
            }
        })
    })
}
