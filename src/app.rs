use crate::ui::settings::Number;
use crate::{
    message::{ConfirmKind, Message, Setting},
    ui,
};
use iced::{
    Element, Event, Length, Subscription, Task, Theme, event,
    keyboard::{self, Key, key::Named},
    widget::{button, column, container, pick_list, row, rule, slider, text},
    window,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use vrx::keybinds::{Action, Chord, canonical_key};
use vrx::{
    engine::{Playback, State, Ticket},
    import,
    persistence::{self, Saved},
    text::Document,
};

const DEMO: &str = "Welcome to VRX, an extremely capable RSVP reader written in blazingly fast Rust.\n\nThe goal of VRX is to provide a swift and smooth reading experience for any amount of text, from paragraphs to pages (and beyond). It also allows the same customization options you'd expect from AxaSuite: radix choice, color options, and every keybinding you could possibly need.\nUse your left/right arrow keys to move forwards/backwards in the text. Change your pace with the up and down keys; alternatively, control it at the bottom right corner of the program.\n\nYou may be wondering, \"Why is this program called VRX?\" It's an acronym that stands for \"Visual Reader X\". Unfortunately, the X doesn't stand for anything in particular—at least, for now.\nThis sample text contains every character with adaptive timing. Now that you have gotten a feel for VRX, it's time to read rapidly! Read responsibly, too.";

pub struct App {
    pub engine: Playback,
    saved: Saved,
    config_path: Option<PathBuf>,
    path: Option<PathBuf>,
    epoch: Instant,
    settings: bool,
    settings_snapshot: Option<Box<vrx::config::Config>>,
    popup: Option<Popup>,
    version_hover: bool,
    export_picker: u64,
    #[cfg(target_os = "macos")]
    native_menu: Option<crate::native_menu::NativeMenu>,
    fields: ui::settings::Fields,
    listening: Option<Action>,
    editor: Option<TextEdit>,
    editor_generation: u64,
    reading: bool,
    fullscreen: bool,
    mode_generation: u64,
    apply_window: WindowAction,
    notice: Option<String>,
    loading: Option<(u64, Arc<AtomicBool>)>,
    pending_input: Option<Input>,
    restore_after_load: Option<(usize, bool)>,
    next_load: u64,
    exporting: Option<Arc<AtomicBool>>,
    saving: bool,
    save_again: bool,
    last_save: Duration,
    closing: Option<window::Id>,
    submission: Arc<AtomicU64>,
    wake_lateness: Duration,
    throughput_start: Duration,
    throughput_words: u64,
    #[cfg(feature = "smoke")]
    native_bench: Option<crate::native_bench::Bench>,
}
enum Popup {
    About,
    Confirm(ConfirmKind),
    Export(Box<ExportDraft>),
}
struct ExportDraft {
    config: vrx::config::Config,
    document: Arc<Document>,
    error: Option<String>,
    picking: bool,
    generation: u64,
}

struct TextEdit {
    content: Option<iced::widget::text_editor::Content>,
    generation: u64,
}

#[derive(Debug)]
enum Input {
    Path(PathBuf),
    Paste(String),
    Reindex { index: usize, playing: bool },
}
#[derive(Default)]
enum WindowAction {
    #[default]
    None,
    Fullscreen(bool),
}

impl App {
    pub fn boot(initial: Option<PathBuf>) -> (Self, Task<Message>) {
        let (config_path, saved, notice) = match persistence::config_path() {
            Ok(path) => {
                let (saved, notice) = persistence::load(&path);
                (Some(path), saved, notice)
            }
            Err(e) => (None, Saved::default(), Some(e)),
        };
        let parsed = Document::parse(DEMO.into(), &saved.config.orp)
            .or_else(|_| Document::parse(String::new(), &saved.config.orp));
        // Document::parse(empty) has no failure branch; handle the Result anyway.
        let document = match parsed {
            Ok(d) => d,
            Err(e) => {
                eprintln!("VRX could not initialize: {e}");
                return Self::empty_boot(saved, config_path, e);
            }
        };
        let engine = Playback::new(Arc::new(document), saved.config.timing.clone());
        let mut app = Self::new(engine, saved, config_path, notice);
        let task = initial
            .map(|p| app.begin_input(Input::Path(p)))
            .unwrap_or_else(Task::none);
        #[cfg(target_os = "macos")]
        let task = Task::batch([task, window::latest().map(|_| Message::InstallMenu)]);
        #[cfg(feature = "smoke")]
        let task = if std::env::var_os("VRX_REVIEW_DIR").is_some() {
            Task::batch([task, smoke_after(500, 30)])
        } else if std::env::var_os("VRX_EDITOR_SMOKE_DIR").is_some() {
            Task::batch([task, smoke_after(500, 20)])
        } else if std::env::var_os("VRX_SMOKE_DIR").is_some() {
            Task::batch([task, smoke_after(500, 0)])
        } else if app.native_bench.is_some() {
            Task::batch([task, smoke_after(500, 10)])
        } else {
            task
        };
        (app, task)
    }
    fn empty_boot(
        saved: Saved,
        config_path: Option<PathBuf>,
        error: String,
    ) -> (Self, Task<Message>) {
        // Empty::default is an infallible construction, no allocation required.
        let engine = Playback::new(Arc::new(Document::default()), saved.config.timing.clone());
        (
            Self::new(engine, saved, config_path, Some(error)),
            Task::none(),
        )
    }
    fn new(
        engine: Playback,
        saved: Saved,
        config_path: Option<PathBuf>,
        notice: Option<String>,
    ) -> Self {
        let mut saved = saved;
        saved.config.timing = engine.timing().clone();
        let fields = ui::settings::Fields::new(&saved.config);
        Self {
            engine,
            fields,
            listening: None,
            editor: None,
            editor_generation: 0,
            saved,
            config_path,
            path: None,
            epoch: Instant::now(),
            settings: false,
            settings_snapshot: None,
            popup: None,
            version_hover: false,
            export_picker: 0,
            #[cfg(target_os = "macos")]
            native_menu: None,
            reading: false,
            fullscreen: false,
            mode_generation: 0,
            apply_window: WindowAction::None,
            notice,
            loading: None,
            pending_input: None,
            restore_after_load: None,
            next_load: 0,
            exporting: None,
            saving: false,
            save_again: false,
            last_save: Duration::ZERO,
            closing: None,
            submission: Arc::new(AtomicU64::new(0)),
            wake_lateness: Duration::ZERO,
            throughput_start: Duration::ZERO,
            throughput_words: 0,
            #[cfg(feature = "smoke")]
            native_bench: crate::native_bench::Bench::from_env(),
        }
    }
    fn now(&self) -> Duration {
        self.epoch.elapsed()
    }
    fn begin_input(&mut self, input: Input) -> Task<Message> {
        self.engine.pause(self.now());
        if let Some((_, cancel)) = &self.loading {
            cancel.store(true, Ordering::Relaxed);
            self.pending_input = Some(input);
            return Task::none();
        }
        self.next_load = self.next_load.wrapping_add(1);
        self.restore_after_load = match &input {
            Input::Reindex { index, playing } => Some((*index, *playing)),
            _ => None,
        };
        let generation = self.next_load;
        let cancel = Arc::new(AtomicBool::new(false));
        self.loading = Some((generation, cancel.clone()));
        let strategy = self.saved.config.orp;
        let old = self.engine.document().clone();
        let path = match &input {
            Input::Path(path) => Some(path.clone()),
            Input::Reindex { .. } => self.path.clone(),
            Input::Paste(_) => None,
        };
        Task::perform(
            async move {
                let mut path = path;
                let document = match input {
                    Input::Path(input_path) => match std::fs::canonicalize(&input_path) {
                        Ok(canonical) => {
                            path = Some(canonical.clone());
                            import::load(&canonical, strategy, &cancel)
                        }
                        Err(error) => Err(format!("Cannot locate document: {error}")),
                    },
                    Input::Paste(source) => Document::parse_cancellable(source, &strategy, &cancel),
                    Input::Reindex { .. } => {
                        Document::parse_cancellable(old.source().to_owned(), &strategy, &cancel)
                    }
                };
                (generation, path, document.map(Arc::new))
            },
            |(generation, path, result)| Message::Loaded(generation, path, result),
        )
    }
    fn remember(&mut self) {
        self.saved.config.timing = self.engine.timing().clone();
        if let Some(path) = &self.path {
            self.saved.remember(
                path.clone(),
                self.engine.document().fingerprint,
                self.engine.index(),
            );
        }
    }
    fn save(&mut self) -> Task<Message> {
        self.remember();
        if self.saving {
            self.save_again = true;
            return Task::none();
        }
        let Some(path) = self.config_path.clone() else {
            return Task::none();
        };
        self.saving = true;
        self.last_save = self.now();
        let mut saved = self.saved.clone();
        if let Some(snapshot) = &self.settings_snapshot {
            saved.config = (**snapshot).clone();
        }
        saved.config.sanitize();
        Task::perform(
            async move { persistence::save(&path, &saved) },
            Message::Saved,
        )
    }
    fn set_speed(&mut self, delta: i32) {
        let mut timing = self.engine.timing().clone();
        timing.wpm = (i64::from(timing.wpm) + i64::from(delta)).clamp(20, 10_000) as u32;
        self.engine.set_timing(timing.clone(), self.now());
        self.saved.config.timing = timing;
        self.fields.refresh(Number::Wpm, &self.saved.config);
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let update_start = Instant::now();
        let task = self.handle(message);
        if self.saved.config.diagnostics {
            self.engine.diagnostics.last_update_ns =
                update_start.elapsed().as_nanos().min(u128::from(u64::MAX)) as u64;
            self.engine.diagnostics.last_submission_ns = self.submission.load(Ordering::Relaxed);
        }
        task
    }
    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            #[cfg(feature = "smoke")]
            Message::Smoke(stage) => return self.smoke(stage),
            #[cfg(feature = "smoke")]
            Message::Capture(stage, Some(id)) => {
                return window::screenshot(id).map(move |s| Message::Screenshot(stage, s));
            }
            #[cfg(feature = "smoke")]
            Message::Capture(_, None) => return iced::exit(),
            #[cfg(feature = "smoke")]
            Message::Screenshot(stage, screenshot) => {
                if let Some(directory) =
                    std::env::var_os("VRX_REVIEW_DIR").or_else(|| std::env::var_os("VRX_SMOKE_DIR"))
                {
                    let path = PathBuf::from(directory).join(format!("reader-{stage}.ppm"));
                    let mut image = format!(
                        "P6\n{} {}\n255\n",
                        screenshot.size.width, screenshot.size.height
                    )
                    .into_bytes();
                    for pixel in screenshot.rgba.as_chunks::<4>().0 {
                        if let Some(rgb) = pixel.get(..3) {
                            image.extend_from_slice(rgb);
                        }
                    }
                    if let Err(e) = std::fs::write(path, image) {
                        eprintln!("Screenshot failed: {e}");
                    }
                }
                return smoke_after(300, stage.saturating_add(1));
            }
            #[cfg(target_os = "macos")]
            Message::InstallMenu => match crate::native_menu::install() {
                Ok(menu) => self.native_menu = Some(menu),
                Err(error) => self.notice = Some(format!("Native menu: {error}")),
            },
            Message::About => {
                self.engine.pause(self.now());
                self.popup = Some(Popup::About);
                return unfocus();
            }
            Message::VersionHover(hover) => self.version_hover = hover,
            Message::Quit => {
                return window::latest().map(|id| match id {
                    Some(id) => Message::Close(id),
                    None => Message::Dismiss,
                });
            }
            Message::ClosePopup => {
                self.popup = None;
                return unfocus();
            }
            Message::RequestConfirm(kind) => {
                if self.settings {
                    self.popup = Some(Popup::Confirm(kind));
                }
                return unfocus();
            }
            Message::Confirm => {
                if let Some(Popup::Confirm(kind)) = self.popup.take() {
                    return match kind {
                        ConfirmKind::Defaults => {
                            self.restore_preferences(vrx::config::Config::default(), false)
                        }
                        ConfirmKind::Discard => {
                            let config = self
                                .settings_snapshot
                                .take()
                                .map(|c| *c)
                                .unwrap_or_else(|| self.saved.config.clone());
                            self.restore_preferences(config, true)
                        }
                        ConfirmKind::Keybindings => self.handle(Message::ResetBindings),
                    };
                }
            }
            Message::AlphabetInput(value) => self.fields.alphabet = value,
            Message::TernaryInput(value) => self.fields.ternary = value,
            Message::AlphabetApply => {
                if self.apply_alphabet() {
                    return unfocus();
                }
            }
            Message::CountdownPreset(preset) => {
                let (short, long) = match preset {
                    1 => ("{seconds}s", "{minutes}m {seconds:02}s"),
                    2 => ("{seconds} seconds", "{minutes} minutes {seconds} seconds"),
                    3 => ("{seconds}", "{minutes}:{seconds:02}"),
                    _ => ("{seconds}s", "{minutes}:{seconds:02}"),
                };
                self.saved.config.countdown_short = short.into();
                self.saved.config.countdown_long = long.into();
            }
            Message::EditText => {
                if self.settings {
                    let task = self.handle(Message::Settings);
                    if self.settings {
                        return task;
                    }
                    return Task::batch([task, self.handle(Message::EditText)]);
                }
                if self.editor.is_some() || self.loading.is_some() {
                    return Task::none();
                }
                if self.engine.document().source().len() > 16 * 1024 * 1024 {
                    self.notice=Some("The in-window editor supports up to 16 MiB. Reading larger files is still available.".into());
                    return Task::none();
                }
                self.engine.pause(self.now());
                self.reading = false;
                self.settings = false;
                self.listening = None;
                self.editor_generation = self.editor_generation.wrapping_add(1);
                let generation = self.editor_generation;
                self.editor = Some(TextEdit {
                    content: None,
                    generation,
                });
                let document = self.engine.document().clone();
                let prepare = Task::perform(
                    async move { iced::widget::text_editor::Content::with_text(document.source()) },
                    move |content| Message::EditorReady(generation, content),
                );
                return Task::batch([self.save(), prepare, unfocus()]);
            }
            Message::EditorReady(generation, content) => {
                if let Some(editor) = &mut self.editor
                    && editor.generation == generation
                {
                    editor.content = Some(content);
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::focusable::focus(
                            iced::advanced::widget::Id::new("reading-text-editor"),
                        ),
                    );
                }
            }
            Message::EditAction(action) => {
                if let Some(content) = self.editor.as_mut().and_then(|e| e.content.as_mut()) {
                    content.perform(action);
                }
            }
            Message::ClearText => {
                if let Some(editor) = &mut self.editor {
                    // Invalidate any clipboard/preparation reply from before Clear.
                    self.editor_generation = self.editor_generation.wrapping_add(1);
                    editor.generation = self.editor_generation;
                    editor.content = Some(iced::widget::text_editor::Content::new());
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::focusable::focus(
                            iced::advanced::widget::Id::new("reading-text-editor"),
                        ),
                    );
                }
            }
            Message::AddSampleText => {
                if let Some(editor) = &mut self.editor {
                    self.editor_generation = self.editor_generation.wrapping_add(1);
                    editor.generation = self.editor_generation;
                    editor.content = Some(iced::widget::text_editor::Content::with_text(DEMO));
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::focusable::focus(
                            iced::advanced::widget::Id::new("reading-text-editor"),
                        ),
                    );
                }
            }
            Message::CancelEdit => {
                self.editor = None;
                self.editor_generation = self.editor_generation.wrapping_add(1);
                return unfocus();
            }
            Message::PasteEdit => {
                if let Some(editor) = &self.editor {
                    let generation = editor.generation;
                    return iced::clipboard::read()
                        .map(move |text| Message::EditorClipboard(generation, text));
                }
            }
            Message::EditorClipboard(generation, source) => {
                if let Some(editor) = &mut self.editor
                    && editor.generation == generation
                    && let Some(content) = &mut editor.content
                    && let Some(source) = source
                {
                    use iced::widget::text_editor::{Action, Edit};
                    content.perform(Action::Edit(Edit::Paste(Arc::new(source))));
                    return iced::advanced::widget::operate(
                        iced::advanced::widget::operation::focusable::focus(
                            iced::advanced::widget::Id::new("reading-text-editor"),
                        ),
                    );
                }
            }
            Message::ApplyEdit => {
                if let Some(editor) = self.editor.take() {
                    if let Some(content) = editor.content {
                        let source = content.text();
                        if source != self.engine.document().source() {
                            return self.begin_input(Input::Paste(source));
                        }
                    } else {
                        self.editor = Some(editor);
                    }
                }
            }
            Message::Toggle => {
                if self.loading.is_some() {
                    return Task::none();
                }
                if self.engine.state() != State::Playing {
                    self.throughput_start = self.now();
                    self.throughput_words = 0;
                }
                self.engine.toggle(self.now());
                if self.engine.state() != State::Playing {
                    return self.save();
                }
            }
            Message::Jump(words) => self.engine.jump(words, self.now()),
            Message::Sentence(forward) => {
                let i = self
                    .engine
                    .document()
                    .sentence_start(self.engine.index(), forward);
                self.engine.seek(i, self.now());
            }
            Message::Paragraph(forward) => {
                let i = self
                    .engine
                    .document()
                    .paragraph_start(self.engine.index(), forward);
                self.engine.seek(i, self.now());
            }
            Message::Restart => self.engine.restart(self.now()),
            Message::Speed(delta) => self.set_speed(delta),
            Message::Seek(value) => {
                let index = ((self.engine.document().len().saturating_sub(1) as f64)
                    * value.clamp(0.0, 1.0))
                .round() as usize;
                self.engine.seek(index, self.now());
            }
            Message::Open | Message::Picked(_) | Message::Clipboard(_)
                if self.editor.is_some() || self.popup.is_some() =>
            {
                return Task::none();
            }
            Message::Load(_) if self.editor.is_some() || self.popup.is_some() => {
                return Task::none();
            }
            Message::Paste if self.editor.is_some() => return Task::none(),
            Message::Open => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .add_filter("Readable text", &["txt", "md", "markdown"])
                            .pick_file()
                            .await
                            .map(|p| p.path().to_owned())
                    },
                    Message::Picked,
                );
            }
            Message::Picked(Some(path)) | Message::Load(path) => {
                self.remember();
                let task = self.begin_input(Input::Path(path));
                return Task::batch([self.save(), task]);
            }
            Message::Paste => return iced::clipboard::read().map(Message::Clipboard),
            Message::Clipboard(Some(source)) => {
                self.remember();
                let task = self.begin_input(Input::Paste(source));
                return Task::batch([self.save(), task]);
            }
            Message::Clipboard(None) => {
                self.notice = Some("The clipboard contains no readable text.".into())
            }
            Message::Loaded(generation, path, result) => {
                if self.loading.as_ref().map(|(g, _)| *g) != Some(generation) {
                    return Task::none();
                }
                let cancelled = self
                    .loading
                    .as_ref()
                    .is_some_and(|(_, c)| c.load(Ordering::Relaxed));
                self.loading = None;
                let restore = self.restore_after_load.take();
                if let Some(input) = self.pending_input.take() {
                    return self.begin_input(input);
                }
                if cancelled {
                    self.notice = Some("Loading cancelled.".into());
                    return Task::none();
                }
                match result {
                    Ok(document) => {
                        let position = restore.map(|(index, _)| index).or_else(|| {
                            path.as_ref()
                                .and_then(|p| self.saved.position(p, document.fingerprint))
                        });
                        self.engine.replace(document);
                        self.path = path;
                        if let Some(position) = position {
                            self.engine.seek(position, self.now());
                        }
                        if restore.is_some_and(|(_, playing)| playing) {
                            self.engine.play(self.now());
                        }
                        self.notice = if self.engine.document().is_empty() {
                            Some("This document contains no readable words.".into())
                        } else {
                            None
                        };
                        return self.save();
                    }
                    Err(e) => self.notice = Some(e),
                }
            }
            Message::CancelLoad => {
                if let Some((_, cancel)) = &self.loading {
                    cancel.store(true, Ordering::Relaxed);
                }
                self.pending_input = None;
            }
            Message::Timer(ticket, wake) => {
                self.wake_lateness = wake.saturating_duration_since(
                    self.epoch
                        .checked_add(ticket.deadline)
                        .unwrap_or(self.epoch),
                );
                #[cfg(feature = "smoke")]
                if let Some(bench) = &mut self.native_bench {
                    let result = bench.observe(
                        ticket.deadline,
                        wake.saturating_duration_since(self.epoch),
                        self.epoch.elapsed(),
                        self.submission.load(Ordering::Relaxed),
                    );
                    match result {
                        Some(Some(wpm)) => {
                            let mut timing = self.engine.timing().clone();
                            timing.wpm = wpm;
                            self.saved.config.timing = timing.clone();
                            self.fields.refresh(Number::Wpm, &self.saved.config);
                            self.engine.set_timing(timing, self.now());
                            self.engine.restart(self.now());
                            return Task::none();
                        }
                        Some(None) => {
                            self.engine.pause(self.now());
                            return iced::exit();
                        }
                        None => {}
                    }
                }
                if self.engine.advance(ticket, self.now()) {
                    self.throughput_words = self.throughput_words.saturating_add(1);
                }
                if self.now().saturating_sub(self.last_save) >= Duration::from_secs(30)
                    || self.engine.state() == State::Finished
                {
                    return self.save();
                }
            }
            Message::Keyboard(key, modifiers, captured, repeat) => {
                if self.popup.is_some() {
                    if key == Key::Named(Named::Escape) && !repeat {
                        return self.handle(Message::ClosePopup);
                    }
                    return Task::none();
                }
                if self.editor.is_some() {
                    if key == Key::Named(Named::Escape) && !repeat {
                        return self.handle(Message::CancelEdit);
                    }
                    return Task::none();
                }
                if let Some(action) = self.listening.take() {
                    if let Some(chord) = key_chord(&key, modifiers) {
                        return self.assign_binding(action, &chord.label());
                    }
                    self.listening = Some(action);
                    return Task::none();
                }
                if !captured && let Some(words) = single_word_shortcut(&key, modifiers) {
                    return self.handle(Message::Jump(words));
                }
                if let Some(chord) = key_chord(&key, modifiers)
                    && let Some(action) = self
                        .saved
                        .config
                        .keys
                        .action(&chord, cfg!(target_os = "macos"))
                    && (!captured || action == Action::Personalize)
                    && (!repeat || action.repeatable())
                {
                    return self.action(action);
                }
            }
            Message::Key(key, modifiers) => {
                if self.popup.is_some() {
                    if key == Key::Named(Named::Escape) {
                        return self.handle(Message::ClosePopup);
                    }
                    return Task::none();
                }
                if self.editor.is_some() {
                    if key == Key::Named(Named::Escape) {
                        return self.handle(Message::CancelEdit);
                    }
                    return Task::none();
                }
                if let Some(words) = single_word_shortcut(&key, modifiers) {
                    return self.handle(Message::Jump(words));
                }
                if let Some(chord) = key_chord(&key, modifiers)
                    && let Some(action) = self
                        .saved
                        .config
                        .keys
                        .action(&chord, cfg!(target_os = "macos"))
                {
                    return self.action(action);
                }
            }
            Message::RadixInput(value) => {
                self.fields.radix = value;
                self.apply_radix(false);
            }
            Message::RadixApply => {
                if self.apply_radix(true) {
                    return unfocus();
                }
            }
            Message::NumberInput(number, value) => {
                self.fields.numbers.insert(number, value);
                self.apply_number(number, false);
            }
            Message::NumberApply(number) => {
                if self.apply_number(number, true) {
                    return unfocus();
                }
            }
            Message::NumberSlider(number, tick) => {
                let value = number.slider_quantity(&self.saved.config, tick);
                return self.handle(Message::NumberValue(number, value));
            }
            Message::NumberValue(number, value) => {
                number.apply(&mut self.saved.config, value);
                self.sync_timing();
                self.fields.refresh(number, &self.saved.config);
            }
            Message::Anchor(x, y) => {
                self.saved.config.focal_x = x.clamp(0.02, 0.98);
                self.saved.config.vertical = y.clamp(0.02, 0.98);
                self.fields.refresh(Number::X, &self.saved.config);
                self.fields.refresh(Number::Y, &self.saved.config);
            }
            Message::BindingInput(action, value) => {
                self.fields.bindings.insert(action, value);
            }
            Message::ApplyBinding(action) => {
                let value = self.fields.binding(action).to_owned();
                return self.assign_binding(action, &value);
            }
            Message::RecordBinding(action) => {
                self.listening = Some(action);
                self.fields.notice = Some(format!("Press the new shortcut for {}", action.label()));
                return unfocus();
            }
            Message::ClearBinding(action) => {
                return self.assign_binding(action, "");
            }
            Message::ResetBindings => {
                self.saved.config.keys = vrx::keybinds::Keymap::default();
                self.fields.bindings = Action::ALL
                    .into_iter()
                    .map(|action| (action, self.saved.config.keys.label(action).to_owned()))
                    .collect();
                self.fields.notice = None;
                self.listening = None;
            }
            Message::ToggleAdaptive => {
                self.fields.adaptive_expanded = !self.fields.adaptive_expanded
            }
            Message::ToggleBindings => {
                self.fields.bindings_expanded = !self.fields.bindings_expanded;
                if !self.fields.bindings_expanded && self.listening.take().is_some() {
                    self.fields.notice = None;
                }
            }
            Message::ToggleNotation => {
                self.fields.notation_expanded = !self.fields.notation_expanded
            }
            Message::RefreshWindowMode(id) => {
                self.mode_generation = self.mode_generation.wrapping_add(1);
                let generation = self.mode_generation;
                return window::mode(id).map(move |mode| Message::WindowMode(generation, mode));
            }
            Message::WindowMode(generation, mode) => {
                if generation == self.mode_generation {
                    self.fullscreen = mode == window::Mode::Fullscreen;
                }
            }
            Message::Settings => {
                self.popup = None;
                if self.settings {
                    // Resolve every editable field before leaving Personalize.
                    if !self.apply_alphabet() || !self.apply_radix(true) {
                        return Task::none();
                    }
                    let format_validation = vrx::countdown::validate(
                        &self.saved.config.countdown_short,
                    )
                    .and_then(|_| vrx::countdown::validate(&self.saved.config.countdown_long));
                    if let Err(error) = format_validation {
                        self.fields.notice = Some(error);
                        return Task::none();
                    }
                    if !self.saved.config.dropdown_color.is_empty()
                        && !vrx::config::valid_rgb(&self.saved.config.dropdown_color)
                    {
                        self.fields.notice=Some("Dropdown color needs six RGB hex digits, or leave it empty for the theme accent".into());
                        return Task::none();
                    }
                    for color in [
                        &self.saved.config.foreground,
                        &self.saved.config.background,
                        &self.saved.config.accent,
                        &self.saved.config.warning,
                    ] {
                        if !vrx::config::valid_rgb(color) {
                            self.fields.notice = Some("Colors require six RGB hex digits".into());
                            return Task::none();
                        }
                    }
                    if let Err(error) = vrx::export::background(&self.saved.config) {
                        self.fields.notice = Some(error);
                        return Task::none();
                    }
                    if !self.saved.config.focal_color.is_empty()
                        && self.saved.config.focal_color_rgb().is_none()
                    {
                        self.fields.notice = Some("Use six RGB hex digits for focal color, or leave it empty for the theme accent".into());
                        return Task::none();
                    }
                    for number in Number::ALL {
                        if number == Number::SideOpacity && !self.saved.config.focal_enabled {
                            self.fields.refresh(number, &self.saved.config);
                            continue;
                        }
                        if !self.apply_number(number, true) {
                            return Task::none();
                        }
                    }
                    for action in Action::ALL {
                        let value = self.fields.binding(action).to_owned();
                        if let Err(error) = self.saved.config.keys.assign(action, &value) {
                            self.fields.notice = Some(error);
                            self.fields.bindings_expanded = true;
                            return Task::none();
                        }
                    }
                    self.saved.config.sanitize();
                    self.fields = ui::settings::Fields::new(&self.saved.config);
                    self.settings = false;
                    self.settings_snapshot = None;
                    self.listening = None;
                    return self.save();
                }
                self.settings_snapshot = Some(Box::new(self.saved.config.clone()));
                self.settings = true;
                self.reading = false;
                self.fields = ui::settings::Fields::new(&self.saved.config);
            }
            Message::Set(setting) => return self.setting(setting),
            Message::Reading => {
                if self.settings {
                    let task = self.handle(Message::Settings);
                    if !self.settings {
                        self.reading = !self.reading;
                    }
                    return task;
                }
                self.reading = !self.reading;
            }
            Message::Fullscreen => {
                self.fullscreen = !self.fullscreen;
                self.mode_generation = self.mode_generation.wrapping_add(1);
                self.apply_window = WindowAction::Fullscreen(self.fullscreen);
                return window::latest().map(Message::WindowAction);
            }
            Message::WindowAction(Some(id)) => match std::mem::take(&mut self.apply_window) {
                WindowAction::Fullscreen(fullscreen) => {
                    return window::set_mode(
                        id,
                        if fullscreen {
                            window::Mode::Fullscreen
                        } else {
                            window::Mode::Windowed
                        },
                    );
                }
                WindowAction::None => {}
            },
            Message::Export => {
                if self.exporting.is_some() || self.engine.document().is_empty() {
                    return Task::none();
                }
                self.engine.pause(self.now());
                self.export_picker = self.export_picker.wrapping_add(1);
                self.popup = Some(Popup::Export(Box::new(ExportDraft {
                    config: self.saved.config.clone(),
                    document: self.engine.document().clone(),
                    error: None,
                    picking: false,
                    generation: self.export_picker,
                })));
                return unfocus();
            }
            Message::ExportBackground(color) => {
                if let Some(Popup::Export(draft)) = &mut self.popup {
                    draft.config.export_background = color;
                }
            }
            Message::ChooseExport => {
                if let Some(Popup::Export(draft)) = &mut self.popup {
                    if draft.picking {
                        return Task::none();
                    }
                    if let Err(error) = vrx::export::background(&draft.config) {
                        draft.error = Some(error);
                        return Task::none();
                    }
                    draft.picking = true;
                    draft.error = None;
                    let generation = draft.generation;
                    return Task::perform(
                        async {
                            rfd::AsyncFileDialog::new()
                                .set_file_name("reading.ass")
                                .add_filter("Timed vector subtitles", &["ass"])
                                .add_filter("SubRip subtitles", &["srt"])
                                .add_filter("MP4 (requires FFmpeg)", &["mp4"])
                                .save_file()
                                .await
                                .map(|p| p.path().to_owned())
                        },
                        move |path| Message::ExportPath(generation, path),
                    );
                }
            }
            Message::ExportPath(generation, path) => {
                if !matches!(&self.popup,Some(Popup::Export(d)) if d.generation==generation) {
                    return Task::none();
                }
                let Some(path) = path else {
                    if let Some(Popup::Export(draft)) = &mut self.popup {
                        draft.picking = false;
                    }
                    return Task::none();
                };
                if self.exporting.is_some() {
                    return Task::none();
                }
                if let Some(Popup::Export(draft)) = self.popup.take() {
                    let cancel = Arc::new(AtomicBool::new(false));
                    self.exporting = Some(cancel.clone());
                    let document = draft.document;
                    let config = draft.config;
                    return Task::perform(
                        async move { vrx::export::export(&document, &config, &path, &cancel) },
                        Message::Exported,
                    );
                }
            }
            Message::Exported(result) => {
                self.exporting = None;
                self.notice = Some(match result {
                    Ok(()) => "Export complete.".into(),
                    Err(e) => e,
                });
                if self.closing.is_some() && !self.saving {
                    return iced::exit();
                }
            }
            Message::CancelExport => {
                if let Some(cancel) = &self.exporting {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            Message::Saved(result) => {
                self.saving = false;
                if let Err(e) = result {
                    self.notice = Some(format!("Could not save settings: {e}"));
                }
                if self.save_again {
                    self.save_again = false;
                    return self.save();
                }
                if self.closing.is_some() && self.exporting.is_none() {
                    return iced::exit();
                }
            }
            Message::Close(id) => {
                self.engine.shutdown(self.now());
                self.closing = Some(id);
                if let Some((_, cancel)) = &self.loading {
                    cancel.store(true, Ordering::Relaxed);
                }
                if let Some(cancel) = &self.exporting {
                    cancel.store(true, Ordering::Relaxed);
                }
                self.pending_input = None;
                let task = self.save();
                if self.saving || self.exporting.is_some() {
                    return task;
                }
                return iced::exit();
            }
            Message::Dismiss => {
                self.notice = None;
                self.fields.notice = None;
            }
            Message::Picked(None) | Message::WindowAction(None) => {}
        }
        Task::none()
    }
    fn action(&mut self, action: Action) -> Task<Message> {
        let jump = i64::from(self.saved.config.jump_words);
        let step = self.saved.config.speed_step as i32;
        let message = match action {
            Action::Toggle => Message::Toggle,
            Action::Backward => Message::Jump(-jump),
            Action::Forward => Message::Jump(jump),
            Action::PreviousWord => Message::Jump(-1),
            Action::NextWord => Message::Jump(1),
            Action::PreviousSentence => Message::Sentence(false),
            Action::NextSentence => Message::Sentence(true),
            Action::PreviousParagraph => Message::Paragraph(false),
            Action::NextParagraph => Message::Paragraph(true),
            Action::SpeedUp => Message::Speed(step),
            Action::SpeedDown => Message::Speed(-step),
            Action::Restart => Message::Restart,
            Action::Focus => Message::Reading,
            Action::Fullscreen => Message::Fullscreen,
            Action::Personalize => Message::Settings,
            Action::Open => Message::Open,
            Action::Paste => Message::Paste,
            Action::Export => Message::Export,
            Action::Edit => Message::EditText,
            Action::About => Message::About,
            Action::ExitFullscreen => {
                self.fullscreen = false;
                self.mode_generation = self.mode_generation.wrapping_add(1);
                self.apply_window = WindowAction::Fullscreen(false);
                return window::latest().map(Message::WindowAction);
            }
        };
        self.handle(message)
    }
    fn sync_timing(&mut self) {
        if self.engine.timing() != &self.saved.config.timing {
            self.engine
                .set_timing(self.saved.config.timing.clone(), self.now());
        }
    }
    fn apply_alphabet(&mut self) -> bool {
        let validation = vrx::numerals::validate_alphabet(
            &self.fields.alphabet,
            usize::from(self.saved.config.radix),
            60,
        )
        .and_then(|_| vrx::numerals::validate_alphabet(&self.fields.ternary, 3, 3));
        if let Err(error) = validation {
            self.fields.notice = Some(error);
            self.fields.notation_expanded = true;
            return false;
        }
        let changed = self.saved.config.numeral_alphabet != self.fields.alphabet
            || self.saved.config.ternary_alphabet != self.fields.ternary;
        self.saved.config.numeral_alphabet = self.fields.alphabet.clone();
        self.saved.config.ternary_alphabet = self.fields.ternary.clone();
        if changed {
            for number in Number::ALL {
                self.fields.refresh(number, &self.saved.config);
            }
        }
        self.fields.notice = None;
        true
    }
    fn restore_preferences(
        &mut self,
        mut config: vrx::config::Config,
        close: bool,
    ) -> Task<Message> {
        config.sanitize();
        let reindex = config.orp != self.saved.config.orp;
        let playing = self.engine.state() == State::Playing;
        let index = self.engine.index();
        self.saved.config = config;
        self.sync_timing();
        self.fields = ui::settings::Fields::new(&self.saved.config);
        self.listening = None;
        if close {
            self.settings = false;
            self.settings_snapshot = None;
        }
        let task = if reindex {
            self.begin_input(Input::Reindex { index, playing })
        } else {
            Task::none()
        };
        if close {
            Task::batch([task, self.save()])
        } else {
            task
        }
    }
    fn apply_radix(&mut self, explicit: bool) -> bool {
        let parsed = self.fields.radix.trim().parse::<u8>();
        match parsed {
            Ok(radix)
                if (2..=60).contains(&radix)
                    && self.saved.config.numeral_alphabet.chars().count() >= usize::from(radix) =>
            {
                if self.saved.config.radix != radix {
                    self.saved.config.radix = radix;
                    for number in Number::ALL {
                        self.fields.refresh(number, &self.saved.config);
                    }
                }
                if explicit {
                    self.fields.radix = radix.to_string();
                    self.fields.notice = None;
                }
                true
            }
            _ => {
                if explicit {
                    self.fields.notation_expanded = true;
                    self.fields.notice = Some("Enter a decimal radix from 2 through 60, with at least that many numeral symbols".into());
                }
                false
            }
        }
    }
    fn apply_number(&mut self, number: Number, explicit: bool) -> bool {
        let notation = self.saved.config.notation();
        let (min, max) = number.bounds();
        let parsed = notation.parse_quantity(self.fields.get(number), number.scale());
        let valid = parsed.and_then(|value| {
            let value =
                u32::try_from(value).map_err(|_| "Enter a nonnegative number".to_owned())?;
            if value < min || value > max {
                Err(format!(
                    "Use {} through {}",
                    notation.scaled(min, number.scale()),
                    notation.scaled(max, number.scale())
                ))
            } else {
                Ok(value)
            }
        });
        match valid {
            Ok(value) => {
                number.apply(&mut self.saved.config, value);
                self.sync_timing();
                if explicit {
                    self.fields.refresh(number, &self.saved.config);
                    self.fields.notice = None;
                }
                true
            }
            Err(error) => {
                if explicit {
                    self.fields.notice = Some(error);
                    match number {
                        Number::Jump | Number::Step => self.fields.bindings_expanded = true,
                        Number::Long
                        | Number::Comma
                        | Number::Colon
                        | Number::Semicolon
                        | Number::Sentence
                        | Number::Paragraph
                        | Number::Quote
                        | Number::Parenthesis
                        | Number::Dash
                        | Number::Section => self.fields.adaptive_expanded = true,
                        _ => {}
                    }
                }
                false
            }
        }
    }
    fn assign_binding(&mut self, action: Action, text: &str) -> Task<Message> {
        match self.saved.config.keys.assign(action, text) {
            Ok(()) => {
                self.fields
                    .bindings
                    .insert(action, self.saved.config.keys.label(action).into());
                self.fields.notice = None;
            }
            Err(error) => self.fields.notice = Some(error),
        }
        Task::none()
    }
    fn setting(&mut self, setting: Setting) -> Task<Message> {
        let config = &mut self.saved.config;
        let mut timing_changed = false;
        match setting {
            Setting::Appearance(v) => config.appearance = v,
            Setting::Font(v) => config.font = v,
            Setting::Focal(v) => {
                config.focal_enabled = v;
                if !v {
                    self.fields.refresh(Number::SideOpacity, config);
                }
            }
            Setting::Radix(v) => {
                config.radix = v.clamp(2, 60);
                self.fields = ui::settings::Fields::new(config);
            }
            Setting::Bold(v) => config.bold = v,
            Setting::Guide(v) => config.guide = v,
            Setting::GuideShape(v) => config.guide_shape = v,
            Setting::FocalColor(v) => config.focal_color = v,
            Setting::Adaptive(v) => {
                config.timing.adaptive = v;
                timing_changed = true;
            }
            Setting::Policy(v) => {
                config.timing.overrun = v;
                timing_changed = true;
            }
            Setting::Orp(v) => {
                config.orp = v;
                self.remember();
                return self.begin_input(Input::Reindex {
                    index: self.engine.index(),
                    playing: self.engine.state() == State::Playing,
                });
            }
            Setting::Foreground(v) => config.foreground = v,
            Setting::Background(v) => config.background = v,
            Setting::ExportBackground(v) => config.export_background = v,
            Setting::Accent(v) => config.accent = v,
            Setting::DropdownColor(v) => config.dropdown_color = v,
            Setting::Warning(v) => config.warning = v,
            Setting::CountdownShort(v) => config.countdown_short = v,
            Setting::CountdownLong(v) => config.countdown_long = v,
            Setting::Diagnostics(v) => config.diagnostics = v,
            Setting::Minimal(v) => config.minimal_playback = v,
        }
        if timing_changed {
            self.engine
                .set_timing(self.saved.config.timing.clone(), self.now());
        }
        Task::none()
    }
    pub fn subscription(&self) -> Subscription<Message> {
        let events = event::listen_with(|event, status, id| match event {
            Event::Window(window::Event::CloseRequested) => Some(Message::Close(id)),
            Event::Window(window::Event::Resized(_) | window::Event::Focused) => {
                Some(Message::RefreshWindowMode(id))
            }
            Event::Window(window::Event::FileDropped(path)) => Some(Message::Load(path)),
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                modifiers,
                repeat,
                ..
            }) => Some(Message::Keyboard(
                key,
                modifiers,
                status == event::Status::Captured,
                repeat,
            )),
            _ => None,
        });
        #[cfg(target_os = "macos")]
        let events = Subscription::batch([events, crate::native_menu::subscription()]);
        let timer = self
            .engine
            .ticket()
            .map(|ticket| Subscription::run_with((ticket, self.epoch), deadline_stream))
            .unwrap_or_else(Subscription::none);
        Subscription::batch([events, timer])
    }
    pub fn theme(&self) -> Theme {
        ui::theme::theme(&self.saved.config)
    }
    pub fn title(&self) -> String {
        match self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
        {
            Some(name) => format!("VRX {} · {name}", vrx::VERSION),
            None => format!("VRX {}", vrx::VERSION),
        }
    }
    pub fn view(&self) -> Element<'_, Message> {
        iced::widget::responsive(|size| {
            let base = self.view_sized(size);
            let base = if self.editor.is_some() {
                self.editor_view(base, size)
            } else {
                base
            };
            if let Some(popup) = &self.popup {
                let body = match popup {
                    Popup::About => ui::popups::about(),
                    Popup::Confirm(kind) => ui::popups::confirm(*kind, &self.saved.config),
                    Popup::Export(draft) => ui::popups::export(
                        &draft.config,
                        draft.error.as_deref(),
                        draft.picking,
                        size,
                    ),
                };
                ui::popups::modal(base, body, size)
            } else {
                base
            }
        })
        .into()
    }
    fn editor_view<'a>(
        &'a self,
        base: Element<'a, Message>,
        size: iced::Size,
    ) -> Element<'a, Message> {
        let Some(editor) = &self.editor else {
            return base;
        };
        let mut body=column![row![text("Edit reading text").size(22).width(Length::Fill),button("Cancel").on_press(Message::CancelEdit)],text("Edit the current text below. Apply replaces this reading session; the original file is not overwritten.").size(12)].spacing(12);
        if let Some(content) = &editor.content {
            body = body.push(
                iced::widget::text_editor(content)
                    .id("reading-text-editor")
                    .height(Length::Fill)
                    .size(16)
                    .on_action(Message::EditAction),
            );
            body = body.push(
                row![
                    button("Paste").on_press(Message::PasteEdit),
                    button("Clear text").on_press(Message::ClearText),
                    button("Use sample").on_press(Message::AddSampleText),
                    container(button("Apply changes").on_press(Message::ApplyEdit))
                        .width(Length::Fill)
                        .align_x(iced::alignment::Horizontal::Right),
                ]
                .spacing(12)
                .align_y(iced::Center),
            );
        } else {
            body = body.push(text("Preparing editor…").height(Length::Fill));
        }
        let card = container(body.padding(20))
            .style(container::bordered_box)
            .width((size.width - 32.0).clamp(260.0, 850.0))
            .height((size.height - 40.0).max(180.0));
        let backdrop = container(card)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.55).into()),
                ..container::Style::default()
            });
        iced::widget::stack![base, iced::widget::opaque(backdrop)].into()
    }
    fn view_sized(&self, size: iced::Size) -> Element<'_, Message> {
        let width = size.width;
        let c = &self.saved.config;
        let n = c.notation();
        let document = self.engine.document();
        let hidden = self.reading || (c.minimal_playback && self.engine.state() == State::Playing);
        let (word, start, end, whole) = document
            .display(self.engine.index())
            .unwrap_or(("Ready", 1, 2, false));
        let reader = ui::reader::Reader {
            word,
            focus: (start, end, whole),
            config: c,
            centered: hidden,
            mode: ui::reader::Mode::Reading,
            editing_anchor: self.settings && !hidden,
            epoch: self.epoch,
            submission: Some(self.submission.clone()),
        };
        if hidden {
            return reader.into();
        }
        let (panel_width, panel_max_width) = ui::settings::panel_widths(self.fullscreen);
        if self.settings && width < panel_width + 700.0 {
            return container(
                container(ui::settings::view(c, &self.fields, self.listening))
                    .width(Length::Fill)
                    .max_width(panel_max_width),
            )
            .center_x(Length::Fill)
            .height(Length::Fill)
            .into();
        }
        let compact = width - if self.settings { panel_width } else { 0.0 } < 900.0;
        let (_, _, accent) = ui::theme::palette(c);
        let brand = row![
            text("VRX").size(24).color(accent),
            iced::widget::mouse_area(
                button(
                    iced::widget::rich_text([iced::widget::span::<(), iced::Font>(vrx::VERSION)
                        .underline(self.version_hover)])
                    .size(11)
                )
                .style(button::text)
                .padding(3)
                .on_press(Message::About)
            )
            .on_enter(Message::VersionHover(true))
            .on_exit(Message::VersionHover(false))
        ]
        .spacing(10)
        .align_y(iced::Center);
        let files = row![
            button("Open")
                .on_press(Message::Open)
                .style(button::secondary),
            button("Export")
                .on_press(Message::Export)
                .style(button::secondary),
            iced::widget::tooltip(
                button("✎")
                    .on_press(Message::EditText)
                    .style(button::secondary),
                "Edit current text",
                iced::widget::tooltip::Position::Bottom,
            ),
        ]
        .spacing(8);
        let personalize = button("Personalize")
            .on_press(Message::Settings)
            .style(button::secondary);
        let mut main = column![]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);
        if compact {
            main = main
                .push(
                    row![brand, iced::widget::space::horizontal(), personalize]
                        .align_y(iced::Center),
                )
                .push(files);
        } else {
            main = main.push(
                row![brand, iced::widget::space::horizontal(), files, personalize]
                    .spacing(10)
                    .align_y(iced::Center),
            );
        }
        if let Some(name) = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
        {
            main = main.push(text(name).size(12));
        }
        main = main.push(reader);
        let progress = if document.len() > 1 {
            self.engine.index() as f64 / document.len().saturating_sub(1) as f64
        } else {
            0.0
        };
        let remaining = self.engine.estimated_remaining(self.now());
        main = main.push(slider(0.0..=1.0, progress, Message::Seek).step(0.0001));
        main = main.push(
            row![
                text(format!(
                    "{} / {} words",
                    n.integer(if document.is_empty() {
                        0
                    } else {
                        self.engine.index() as i128 + 1
                    }),
                    n.integer(document.len() as i128)
                ))
                .font(ui::settings::NUMBER_FONT)
                .size(12)
                .width(Length::Fill),
                text(format!(
                    "{} remaining",
                    vrx::countdown::remaining(remaining, c)
                ))
                .font(ui::settings::NUMBER_FONT)
                .size(12)
            ]
            .spacing(10),
        );
        let label = match self.engine.state() {
            State::Playing => "Pause",
            State::Finished => "Read again",
            _ => "Play",
        };
        let jump = i64::from(c.jump_words);
        let step = c.speed_step as i32;
        let navigation = row![
            button("←")
                .on_press(Message::Jump(-jump))
                .padding([10, 14])
                .style(button::secondary),
            button(label).on_press(Message::Toggle).padding([12, 18]),
            button("→")
                .on_press(Message::Jump(jump))
                .padding([10, 14])
                .style(button::secondary),
            button("Restart")
                .on_press(Message::Restart)
                .style(button::secondary),
        ]
        .spacing(6)
        .align_y(iced::Center);
        let speed = row![
            button("−")
                .on_press(Message::Speed(-step))
                .style(button::secondary),
            iced::widget::text_input("WPM", self.fields.get(Number::Wpm))
                .font(ui::settings::NUMBER_FONT)
                .size(18)
                .width(120)
                .on_input(|s| Message::NumberInput(Number::Wpm, s))
                .on_submit(Message::NumberApply(Number::Wpm)),
            text("WPM").size(12),
            button("+")
                .on_press(Message::Speed(step))
                .style(button::secondary)
        ]
        .spacing(8)
        .align_y(iced::Center);
        if compact {
            main = main.push(navigation).push(speed);
        } else {
            main = main.push(
                row![navigation, iced::widget::space::horizontal(), speed].align_y(iced::Center),
            );
        }
        if !self.saved.recent.is_empty() {
            let recent: Vec<PathChoice> = self
                .saved
                .recent
                .iter()
                .map(|r| PathChoice(r.path.clone()))
                .collect();
            main = main.push(
                pick_list(recent, None::<PathChoice>, |p| Message::Load(p.0))
                    .placeholder("Recent documents")
                    .text_size(12),
            );
        }
        if self.loading.is_some() {
            main = main.push(
                row![
                    text("Preparing document…").size(13),
                    button("Cancel").on_press(Message::CancelLoad)
                ]
                .spacing(10),
            );
        }
        if self.exporting.is_some() {
            main = main.push(
                row![
                    text("Exporting…").size(13),
                    button("Cancel export").on_press(Message::CancelExport)
                ]
                .spacing(10),
            );
        }
        if let Some(notice) = self.notice.as_ref().or(self.fields.notice.as_ref()) {
            main = main.push(
                row![
                    text(notice).size(12).width(Length::Fill),
                    button("Dismiss").on_press(Message::Dismiss)
                ]
                .spacing(10),
            );
        }
        if c.diagnostics {
            let d = &self.engine.diagnostics;
            let ms = |ns: u64| n.scaled((ns / 1000).min(u64::from(u32::MAX)) as u32, 1000);
            main=main.push(rule::horizontal(1)).push(text(format!("Wake {} ms · UI {} ms · update {} ms · overruns {} · skipped {} · document {} MiB · physical presentation unavailable",ms(self.wake_lateness.as_nanos().min(u128::from(u64::MAX)) as u64),ms(d.last_lateness_ns),ms(d.last_update_ns),n.integer(i128::from(d.late_events)),n.integer(i128::from(d.skipped_words)),n.scaled((document.estimated_bytes() as f64/1048.576) as u32,1000))).font(ui::settings::NUMBER_FONT).size(10));
        }
        let main: Element<'_, Message> = if size.height < 580.0 {
            iced::widget::scrollable(container(main.height(680).padding(20)).width(Length::Fill))
                .height(Length::Fill)
                .into()
        } else {
            container(main.padding(20))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };
        if self.settings {
            row![
                main,
                container(ui::settings::view(c, &self.fields, self.listening)).width(panel_width)
            ]
            .into()
        } else {
            main
        }
    }
}

#[cfg(feature = "smoke")]
fn smoke_after(milliseconds: u64, stage: u8) -> Task<Message> {
    Task::perform(
        async move {
            futures_timer::Delay::new(Duration::from_millis(milliseconds)).await;
            stage
        },
        Message::Smoke,
    )
}
#[cfg(feature = "smoke")]
impl App {
    fn smoke(&mut self, stage: u8) -> Task<Message> {
        match stage {
            30 => {
                let task = self.handle(Message::Settings);
                Task::batch([task, smoke_after(250, 31)])
            }
            31 | 33 | 35 | 37 | 39 | 41 => {
                window::latest().map(move |id| Message::Capture(stage, id))
            }
            32 => {
                let task = self.handle(Message::RequestConfirm(ConfirmKind::Defaults));
                Task::batch([task, smoke_after(200, 33)])
            }
            34 => {
                let task = self.handle(Message::About);
                Task::batch([task, smoke_after(200, 35)])
            }
            36 => {
                let _ = self.handle(Message::ClosePopup);
                let task = self.handle(Message::Export);
                Task::batch([task, smoke_after(200, 37)])
            }
            38 => {
                let _ = self.handle(Message::ClosePopup);
                let _ = self.handle(Message::Settings);
                self.reading = true;
                self.engine.seek(3, self.now());
                self.saved.config.side_opacity_percent = 40;
                smoke_after(200, 39)
            }
            40 => {
                self.engine.seek(8, self.now());
                smoke_after(200, 41)
            }
            42 => {
                if let Some(directory) = std::env::var_os("VRX_REVIEW_DIR") {
                    #[cfg(target_os = "macos")]
                    let menu_installed = self.native_menu.is_some();
                    #[cfg(not(target_os = "macos"))]
                    let menu_installed = false;
                    let value = serde_json::json!({"version":vrx::VERSION,"native_menu_installed":menu_installed,
                        "final_word":self.engine.document().word(self.engine.index()),"focus":self.reading});
                    if let Ok(bytes) = serde_json::to_vec_pretty(&value) {
                        let _ = std::fs::write(PathBuf::from(directory).join("review.json"), bytes);
                    }
                }
                iced::exit()
            }

            20 => Task::batch([self.handle(Message::EditText), smoke_after(1000, 21)]),
            21 => {
                let prefilled = self
                    .editor
                    .as_ref()
                    .and_then(|e| e.content.as_ref())
                    .map(iced::widget::text_editor::Content::text);
                if prefilled.is_none() {
                    return smoke_after(100, 21);
                }
                if prefilled.as_deref() != Some(self.engine.document().source()) {
                    eprintln!("Editor smoke: current text was not prefilled");
                    return iced::exit();
                }
                use iced::widget::text_editor::{Action, Edit};
                let _ = self.handle(Message::EditAction(Action::SelectAll));
                let _ = self.handle(Message::EditAction(Action::Edit(Edit::Paste(Arc::new(
                    "Edited in VRX.\n\nA new reading session.".into(),
                )))));
                Task::batch([self.handle(Message::ApplyEdit), smoke_after(500, 22)])
            }
            22 => {
                if self.loading.is_some() {
                    return smoke_after(100, 22);
                }
                if let Some(directory) = std::env::var_os("VRX_EDITOR_SMOKE_DIR") {
                    let directory = PathBuf::from(directory);
                    let result = (|| -> vrx::Result<()> {
                        std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                        let value = serde_json::json!({
                            "version":vrx::VERSION,
                            "current_text_prefilled":true,
                            "editor_closed":self.editor.is_none(),
                            "replacement_loaded":self.engine.document().source() == "Edited in VRX.\n\nA new reading session.",
                            "ready_at_start":self.engine.state()==State::Ready && self.engine.index()==0,
                            "original_file_not_overwritten":self.path.is_none(),
                            "screenshots_or_logo_checks":false,
                        });
                        let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
                        std::fs::write(directory.join("editor.json"), bytes)
                            .map_err(|e| e.to_string())
                    })();
                    if let Err(error) = result {
                        eprintln!("Editor smoke: {error}");
                    }
                }
                iced::exit()
            }
            10 => {
                if let Some(bench) = &self.native_bench {
                    let mut timing = self.engine.timing().clone();
                    timing.adaptive = false;
                    timing.wpm = bench.first_speed();
                    self.saved.config.diagnostics = true;
                    self.saved.config.timing = timing.clone();
                    self.engine.set_timing(timing, self.now());
                    self.engine.restart(self.now());
                    self.engine.play(self.now());
                }
                Task::none()
            }
            0 => {
                self.saved.config.diagnostics = true;
                window::latest().map(|id| Message::Capture(0, id))
            }
            1 => {
                self.engine.play(self.now());
                smoke_after(1000, 2)
            }
            2 => {
                let before = self.engine.index();
                let playing = self.engine.state() == State::Playing;
                self.engine.jump(5, self.now());
                self.set_speed(25);
                let uninterrupted = self.engine.state() == State::Playing;
                self.engine.pause(self.now());
                if let Some(directory) = std::env::var_os("VRX_SMOKE_DIR") {
                    let report = serde_json::json!({"advanced":before>0,"was_playing":playing,"jump_and_speed_kept_playing":uninterrupted,"after_index":self.engine.index(),"wpm":self.engine.timing().wpm,"diagnostics":self.engine.diagnostics});
                    if let Ok(bytes) = serde_json::to_vec_pretty(&report) {
                        let _ = std::fs::write(
                            PathBuf::from(directory).join("native-smoke.json"),
                            bytes,
                        );
                    }
                }
                self.settings = true;
                window::latest().map(|id| Message::Capture(2, id))
            }
            3 => {
                self.settings = false;
                self.reading = true;
                window::latest().map(|id| Message::Capture(3, id))
            }
            4 => {
                self.reading = false;
                self.saved.config.appearance = vrx::config::Appearance::Light;
                window::latest().map(|id| Message::Capture(4, id))
            }
            5 => {
                self.settings = false;
                self.reading = true;
                self.saved.config.focal_enabled = false;
                self.saved.config.gui_text_size = 16.0;
                self.engine.seek(9, self.now());
                window::latest().map(|id| Message::Capture(5, id))
            }
            6 => {
                self.saved.config.gui_text_size = 24.0;
                window::latest().map(|id| Message::Capture(6, id))
            }
            7 => {
                self.reading = false;
                self.settings = true;
                self.saved.config.radix = 3;
                self.fields = ui::settings::Fields::new(&self.saved.config);
                window::latest().map(|id| Message::Capture(7, id))
            }
            8 => {
                self.saved.config.radix = 12;
                self.saved.config.gui_text_size = 16.0;
                self.saved.config.timing.wpm = 131;
                self.sync_timing();
                self.fields = ui::settings::Fields::new(&self.saved.config);
                window::latest().map(|id| Message::Capture(8, id))
            }
            _ => {
                if stage == 255 {
                    return iced::exit();
                }
                let task = self.save();
                if self.saving {
                    self.closing = None;
                    return task.chain(Task::perform(
                        async {
                            futures_timer::Delay::new(Duration::from_millis(300)).await;
                        },
                        |_| Message::Smoke(255),
                    ));
                }
                iced::exit()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PathChoice(PathBuf);
impl std::fmt::Display for PathChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.to_string_lossy())
    }
}

fn deadline_stream(
    data: &(Ticket, Instant),
) -> impl iced::futures::Stream<Item = Message> + Send + 'static + use<> {
    let (ticket, epoch) = *data;
    iced::futures::stream::once(async move {
        let deadline = epoch.checked_add(ticket.deadline).unwrap_or(epoch);
        futures_timer::Delay::new(deadline.saturating_duration_since(Instant::now())).await;
        Message::Timer(ticket, Instant::now())
    })
}

fn unfocus() -> Task<Message> {
    iced::advanced::widget::operate(iced::advanced::widget::operation::focusable::unfocus())
}

fn single_word_shortcut(key: &Key, modifiers: keyboard::Modifiers) -> Option<i64> {
    if !(modifiers.control() || modifiers.logo()) || modifiers.alt() || modifiers.shift() {
        return None;
    }
    match key {
        Key::Named(Named::ArrowLeft) => Some(-1),
        Key::Named(Named::ArrowRight) => Some(1),
        _ => None,
    }
}

fn key_chord(key: &Key, modifiers: keyboard::Modifiers) -> Option<Chord> {
    let key = match key.as_ref() {
        Key::Named(Named::Shift | Named::Control | Named::Alt | Named::Super | Named::Meta) => {
            return None;
        }
        Key::Named(named) => canonical_key(&format!("{named:?}")),
        Key::Character(character) => canonical_key(character),
        _ => return None,
    };
    Some(Chord {
        key,
        ctrl: modifiers.control(),
        alt: modifiers.alt(),
        shift: modifiers.shift(),
        logo: modifiers.logo(),
        command: false,
    })
}

pub fn run(path: Option<PathBuf>) -> iced::Result {
    iced::application(move || App::boot(path.clone()), App::update, App::view)
        .title(App::title)
        .font(include_bytes!("../assets/AxaMono-Regular.ttf").as_slice())
        .scale_factor(|app: &App| app.saved.config.gui_scale())
        .theme(App::theme)
        .subscription(App::subscription)
        .window(window::Settings {
            size: iced::Size::new(1100.0, 760.0),
            icon: window::icon::from_rgba(
                include_bytes!("../assets/icons/window.rgba").to_vec(),
                64,
                64,
            )
            .ok(),
            min_size: Some(iced::Size::new(720.0, 540.0)),
            exit_on_close_request: false,
            ..window::Settings::default()
        })
        .centered()
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn app() -> vrx::Result<App> {
        let mut saved = Saved::default();
        saved.config.timing.adaptive = false;
        saved.config.jump_words = 3;
        saved.config.speed_step = 75;
        let engine = Playback::new(
            Document::parse(DEMO.into(), &saved.config.orp)?.shared(),
            saved.config.timing.clone(),
        );
        Ok(App::new(engine, saved, None, None))
    }
    #[test]
    fn configured_keyboard_jumps_and_speed_keep_reading() -> vrx::Result<()> {
        let mut app = app()?;
        let _ = app.update(Message::Key(
            Key::Named(Named::Space),
            keyboard::Modifiers::empty(),
        ));
        let _ = app.update(Message::Key(
            Key::Named(Named::ArrowRight),
            keyboard::Modifiers::empty(),
        ));
        assert_eq!(app.engine.index(), 3);
        assert_eq!(app.engine.state(), State::Playing);
        let _ = app.update(Message::Key(
            Key::Named(Named::ArrowUp),
            keyboard::Modifiers::empty(),
        ));
        assert_eq!(app.engine.timing().wpm, 375);
        assert_eq!(app.engine.index(), 3);
        assert_eq!(app.engine.state(), State::Playing);
        let _ = app.update(Message::Key(
            Key::Named(Named::ArrowLeft),
            keyboard::Modifiers::empty(),
        ));
        assert_eq!(app.engine.index(), 0);
        assert_eq!(app.engine.state(), State::Playing);
        Ok(())
    }
    #[test]
    fn numeric_fields_can_be_cleared_and_retyped() -> vrx::Result<()> {
        let mut app = app()?;
        let _ = app.update(Message::NumberInput(Number::Wpm, String::new()));
        assert_eq!(app.fields.get(Number::Wpm), "");
        assert_eq!(app.engine.timing().wpm, 300);
        let _ = app.update(Message::NumberInput(Number::Wpm, "6".into()));
        let _ = app.update(Message::NumberInput(Number::Wpm, "600".into()));
        assert_eq!(app.fields.get(Number::Wpm), "600");
        assert_eq!(app.engine.timing().wpm, 600);
        Ok(())
    }
    #[test]
    fn obsolete_document_load_cannot_replace_current_text() -> vrx::Result<()> {
        let mut app = app()?;
        let original = app.engine.document().fingerprint;
        app.loading = Some((9, Arc::new(AtomicBool::new(false))));
        let _ = app.update(Message::Loaded(8, None, Ok(Document::default().shared())));
        assert_eq!(app.engine.document().fingerprint, original);
        assert_eq!(app.loading.as_ref().map(|(g, _)| *g), Some(9));
        Ok(())
    }
    #[test]
    fn shortcuts_rebind_and_personalize_opens_even_from_captured_input() -> vrx::Result<()> {
        let mut app = app()?;
        let _ = app.update(Message::Keyboard(
            Key::Named(Named::Escape),
            keyboard::Modifiers::empty(),
            true,
            false,
        ));
        assert!(app.settings);
        let _ = app.update(Message::Settings);
        assert!(!app.settings);
        let _ = app.assign_binding(Action::Focus, "J");
        let _ = app.update(Message::Key(
            Key::Character("h".into()),
            keyboard::Modifiers::empty(),
        ));
        assert!(!app.reading);
        let _ = app.update(Message::Key(
            Key::Character("j".into()),
            keyboard::Modifiers::empty(),
        ));
        assert!(app.reading);
        let _ = app.update(Message::Keyboard(
            Key::Named(Named::Escape),
            keyboard::Modifiers::empty(),
            false,
            false,
        ));
        assert!(app.settings);
        assert!(!app.reading);
        Ok(())
    }
    #[test]
    fn radix_edits_quantities_and_keeps_independent_text_sizes() -> vrx::Result<()> {
        let mut app = app()?;
        let _ = app.setting(Setting::Radix(12));
        let _ = app.update(Message::NumberInput(Number::Step, "↊".into()));
        assert_eq!(app.saved.config.speed_step, 10);
        let _ = app.update(Message::NumberInput(Number::Size, "260".into()));
        assert_eq!(app.saved.config.font_size, 360.0);
        let _ = app.setting(Setting::Radix(3));
        let _ = app.update(Message::NumberInput(Number::Jump, "1T".into()));
        assert_eq!(app.saved.config.jump_words, 2);
        let _ = app.update(Message::NumberValue(Number::GuiSize, 24));
        assert_eq!(app.saved.config.font_size, 360.0);
        assert_eq!(app.saved.config.gui_text_size, 24.0);
        let _ = app.update(Message::Anchor(0.23456, 0.76543));
        let _ = app.update(Message::Settings);
        let _ = app.update(Message::Settings);
        assert!((app.saved.config.focal_x - 0.23456).abs() < 0.00002);
        assert!((app.saved.config.vertical - 0.76543).abs() < 0.00002);
        Ok(())
    }
    #[test]
    fn editor_prefills_current_text_and_cancels_without_changing_it() -> vrx::Result<()> {
        use iced::widget::text_editor::Content;
        let mut app = app()?;
        let _ = app.update(Message::Toggle);
        let _ = app.update(Message::Jump(3));
        let _ = app.update(Message::EditText);
        assert_eq!(app.engine.state(), State::Paused);
        let generation = app.editor_generation;
        let original = app.engine.document().source().to_owned();
        let _ = app.update(Message::EditorReady(
            generation,
            Content::with_text(&original),
        ));
        assert_eq!(
            app.editor
                .as_ref()
                .and_then(|e| e.content.as_ref())
                .map(Content::text),
            Some(original.clone())
        );
        let _ = app.update(Message::Keyboard(
            Key::Named(Named::Space),
            keyboard::Modifiers::empty(),
            true,
            false,
        ));
        let _ = app.update(Message::Key(
            Key::Named(Named::ArrowRight),
            keyboard::Modifiers::empty(),
        ));
        assert_eq!(app.engine.state(), State::Paused);
        assert_eq!(app.engine.index(), 3);
        let _ = app.update(Message::Key(
            Key::Named(Named::Escape),
            keyboard::Modifiers::empty(),
        ));
        assert!(app.editor.is_none());
        assert_eq!(app.engine.document().source(), original);
        let _ = app.update(Message::EditorReady(
            generation,
            Content::with_text("obsolete"),
        ));
        assert!(app.editor.is_none());
        assert!(!app.settings);
        Ok(())
    }
    #[test]
    fn editor_applies_replacement_and_reopens_the_edited_buffer() -> vrx::Result<()> {
        use iced::widget::text_editor::{Action as EditAction, Content, Edit};
        let mut app = app()?;
        app.path = Some(PathBuf::from("unchanged-original.txt"));
        let _ = app.update(Message::EditText);
        let generation = app.editor_generation;
        let _ = app.update(Message::EditorReady(generation, Content::with_text(DEMO)));
        let _ = app.update(Message::EditAction(EditAction::SelectAll));
        let replacement = "New text.\n\nEmoji 👩‍👩‍👧‍👦 and e\u{301}.";
        let _ = app.update(Message::EditAction(EditAction::Edit(Edit::Paste(
            Arc::new(replacement.into()),
        ))));
        assert_eq!(
            app.editor
                .as_ref()
                .and_then(|e| e.content.as_ref())
                .map(Content::text)
                .as_deref(),
            Some(replacement)
        );
        let _ = app.update(Message::ApplyEdit);
        assert!(app.editor.is_none());
        let generation = app
            .loading
            .as_ref()
            .map(|(g, _)| *g)
            .ok_or("Missing document reindex request")?;
        let document = Document::parse(replacement.into(), &app.saved.config.orp)?.shared();
        let _ = app.update(Message::Loaded(generation, None, Ok(document)));
        assert_eq!(app.engine.document().source(), replacement);
        assert_eq!(app.engine.index(), 0);
        assert_eq!(app.engine.state(), State::Ready);
        assert!(app.path.is_none());
        let _ = app.update(Message::EditText);
        let _ = app.update(Message::EditorReady(
            app.editor_generation,
            Content::with_text(app.engine.document().source()),
        ));
        let _ = app.update(Message::ApplyEdit);
        assert!(app.editor.is_none());
        assert!(app.loading.is_none());
        assert_eq!(app.engine.document().source(), replacement);
        Ok(())
    }
    #[test]
    fn editor_ignores_stale_preparation_and_external_input() -> vrx::Result<()> {
        use iced::widget::text_editor::Content;
        let mut app = app()?;
        let _ = app.update(Message::EditText);
        let stale = app.editor_generation;
        let _ = app.update(Message::CancelEdit);
        let _ = app.update(Message::EditText);
        let _ = app.update(Message::EditorReady(stale, Content::with_text("stale")));
        assert!(app.editor.as_ref().is_some_and(|e| e.content.is_none()));
        let _ = app.update(Message::Clipboard(Some("late paste".into())));
        let _ = app.update(Message::Picked(Some(PathBuf::from("late.txt"))));
        let _ = app.update(Message::Load(PathBuf::from("dropped.txt")));
        assert!(app.loading.is_none());
        assert_eq!(app.engine.document().source(), DEMO);
        Ok(())
    }
    #[test]
    fn focused_revision_clear_text_is_local_and_rejects_old_clipboard() -> vrx::Result<()> {
        use iced::widget::text_editor::Content;
        let mut app = app()?;
        let _ = app.update(Message::Jump(3));
        let _ = app.update(Message::EditText);
        let generation = app.editor_generation;
        let _ = app.update(Message::EditorReady(generation, Content::with_text(DEMO)));
        let _ = app.update(Message::ClearText);
        assert_eq!(
            app.editor
                .as_ref()
                .and_then(|e| e.content.as_ref())
                .map(Content::text)
                .as_deref(),
            Some("")
        );
        let _ = app.update(Message::EditorClipboard(
            generation,
            Some("late paste".into()),
        ));
        assert_eq!(
            app.editor
                .as_ref()
                .and_then(|e| e.content.as_ref())
                .map(Content::text)
                .as_deref(),
            Some("")
        );
        assert_eq!(app.engine.document().source(), DEMO);
        assert_eq!(app.engine.index(), 3);
        let _ = app.update(Message::CancelEdit);
        assert_eq!(app.engine.document().source(), DEMO);
        let _ = app.update(Message::EditText);
        let _ = app.update(Message::ClearText);
        let _ = app.update(Message::ApplyEdit);
        assert!(app.loading.is_some());
        let generation = app
            .loading
            .as_ref()
            .map(|(g, _)| *g)
            .ok_or("Missing reindex request")?;
        let _ = app.update(Message::Loaded(
            generation,
            None,
            Ok(Document::default().shared()),
        ));
        assert_eq!(app.engine.state(), State::Empty);
        Ok(())
    }
    #[test]
    fn revision_discard_defaults_modal_and_export_are_isolated() -> vrx::Result<()> {
        let mut app = app()?;
        let original = app.saved.config.timing.wpm;
        let _ = app.update(Message::Settings);
        let _ = app.update(Message::NumberInput(Number::Wpm, "650".into()));
        let _ = app.update(Message::Set(Setting::Warning("FF0000".into())));
        let _ = app.update(Message::RequestConfirm(ConfirmKind::Discard));
        assert!(matches!(
            app.popup,
            Some(Popup::Confirm(ConfirmKind::Discard))
        ));
        let _ = app.update(Message::Confirm);
        assert_eq!(app.engine.timing().wpm, original);
        assert!(!app.settings);
        assert!(app.settings_snapshot.is_none());
        let _ = app.update(Message::Settings);
        let _ = app.update(Message::RequestConfirm(ConfirmKind::Defaults));
        let _ = app.update(Message::ClosePopup);
        assert_eq!(app.saved.config.jump_words, 3);
        let _ = app.update(Message::RequestConfirm(ConfirmKind::Defaults));
        let _ = app.update(Message::Confirm);
        assert_eq!(app.saved.config.jump_words, 10);
        assert!(app.settings);
        let _ = app.update(Message::Settings);
        let background = app.saved.config.export_background.clone();
        let _ = app.update(Message::Export);
        let _ = app.update(Message::ExportBackground("123456".into()));
        assert_eq!(app.saved.config.export_background, background);
        assert!(
            matches!(&app.popup,Some(Popup::Export(d)) if d.config.export_background=="123456")
        );
        let _ = app.update(Message::ClosePopup);
        let _ = app.update(Message::ExportPath(1, Some(PathBuf::from("late.ass"))));
        assert!(app.exporting.is_none());
        Ok(())
    }
    #[test]
    fn revision_command_arrows_always_move_one_word_and_keep_playing() -> vrx::Result<()> {
        let mut app = app()?;
        let _ = app.update(Message::Toggle);
        let _ = app.update(Message::Jump(12));
        for modifier in [keyboard::Modifiers::CTRL, keyboard::Modifiers::LOGO] {
            let _ = app.update(Message::Key(Key::Named(Named::ArrowLeft), modifier));
            assert_eq!(app.engine.index(), 11);
            assert_eq!(app.engine.state(), State::Playing);
            let _ = app.update(Message::Key(Key::Named(Named::ArrowRight), modifier));
            assert_eq!(app.engine.index(), 12);
        }
        Ok(())
    }
    #[test]
    fn personalize_notation_and_fullscreen_width_policy() -> vrx::Result<()> {
        let mut app = app()?;
        let _ = app.update(Message::Settings);
        assert!(!app.fields.notation_expanded);
        assert_eq!(ui::settings::panel_widths(app.fullscreen), (525.0, 650.0));
        let _ = app.update(Message::ToggleNotation);
        let _ = app.update(Message::RadixInput("12".into()));
        assert!(app.fields.notation_expanded);
        assert_eq!(app.saved.config.radix, 12);
        let old_generation = app.mode_generation;
        let _ = app.update(Message::Fullscreen);
        let _ = app.update(Message::WindowMode(old_generation, window::Mode::Windowed));
        assert!(app.fullscreen);
        assert!(matches!(app.apply_window, WindowAction::Fullscreen(true)));
        assert_eq!(ui::settings::panel_widths(app.fullscreen), (420.0, 520.0));
        let fullscreen_generation = app.mode_generation;
        let _ = app.action(Action::ExitFullscreen);
        let _ = app.update(Message::WindowMode(
            fullscreen_generation,
            window::Mode::Fullscreen,
        ));
        assert!(!app.fullscreen);
        assert_eq!(ui::settings::panel_widths(app.fullscreen), (525.0, 650.0));
        let _ = app.update(Message::Settings);
        assert!(!app.settings);
        let _ = app.update(Message::Settings);
        assert!(!app.fields.notation_expanded);
        Ok(())
    }
    #[test]
    fn personalize_sections_preserve_drafts_and_custom_color() -> vrx::Result<()> {
        let mut app = app()?;
        let _ = app.update(Message::Settings);
        assert!(!app.fields.adaptive_expanded && !app.fields.bindings_expanded);
        let _ = app.update(Message::ToggleAdaptive);
        let _ = app.update(Message::NumberInput(Number::Long, "invalid".into()));
        let _ = app.update(Message::ToggleAdaptive);
        let _ = app.update(Message::Settings);
        assert!(app.settings && app.fields.adaptive_expanded);
        assert_eq!(app.fields.get(Number::Long), "invalid");
        let _ = app.update(Message::NumberInput(Number::Long, "1.5".into()));
        let _ = app.update(Message::ToggleBindings);
        let _ = app.update(Message::RecordBinding(Action::Toggle));
        let _ = app.update(Message::ToggleBindings);
        assert!(app.listening.is_none());
        let _ = app.update(Message::ToggleBindings);
        let _ = app.update(Message::ResetBindings);
        assert!(app.fields.bindings_expanded);
        assert_eq!(app.fields.get(Number::Long), "1.5");
        let _ = app.update(Message::Set(Setting::DropdownColor("67AB89".into())));
        assert_eq!(
            ui::theme::dropdown_accent(&app.saved.config),
            ui::theme::hex("67AB89")
        );
        let encoded = serde_json::to_string(&app.saved.config).map_err(|e| e.to_string())?;
        let decoded: vrx::config::Config =
            serde_json::from_str(&encoded).map_err(|e| e.to_string())?;
        assert_eq!(decoded.dropdown_color, "67AB89");
        let _ = app.update(Message::Settings);
        assert!(!app.settings);
        let _ = app.update(Message::Set(Setting::DropdownColor(String::new())));
        assert_eq!(
            ui::theme::dropdown_accent(&app.saved.config),
            ui::theme::palette(&app.saved.config).2
        );
        Ok(())
    }
}
