//! A deliberately small CLI; stdout is machine-readable for agent workflows.
use serde::{Deserialize, Serialize};
use std::{
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};
use vrx::{
    Result,
    config::Config,
    engine::{Playback, State, Timing},
    import,
    text::{Document, orp::Strategy},
};

const HELP: &str = concat!(
    "VRX v",
    env!("CARGO_PKG_VERSION"),
    " — native RSVP reader\n\n  vrx [gui] [FILE]                    Open the native reader\n  vrx inspect FILE                   Document metadata as JSON\n  vrx export FILE OUTPUT [WPM] [RGB]  Export .ass, .srt or .mp4\n  vrx diagnose [SAMPLES]              Absolute-deadline OS timer measurements\n  vrx api [FILE]                     JSON-lines control protocol over stdin/stdout\n  vrx --version                      Package version\n\nUse VRX_CONFIG_DIR to isolate settings; VRX_FFMPEG selects an FFmpeg executable.\nSee docs/API.md for the library and JSON protocol.\n"
);
pub fn run() -> Result<()> {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    let rest: Vec<_> = args.collect();
    let text = first.as_ref().and_then(|s| s.to_str());
    match text {
        None | Some("gui") => {
            crate::app::run(rest.first().map(PathBuf::from)).map_err(|e| e.to_string())
        }
        Some("--help" | "-h" | "help") => write_stdout(HELP.as_bytes()),
        Some("--version" | "-V" | "version") => {
            write_stdout(format!("{}\n", vrx::VERSION).as_bytes())
        }
        Some("inspect") => {
            let path = rest
                .first()
                .map(PathBuf::from)
                .ok_or("Usage: vrx inspect FILE")?;
            let start = Instant::now();
            let document = read(&path, Strategy::Classic)?;
            json(
                &serde_json::json!({"version":vrx::VERSION,"words":document.len(),"source_bytes":document.source().len(),"estimated_document_bytes":document.estimated_bytes(),"token_bytes":std::mem::size_of::<vrx::text::Token>(),"fingerprint":format!("{:016x}",document.fingerprint),"load_ms":start.elapsed().as_secs_f64()*1000.0,"estimated_seconds_at_300_adaptive":Timing::default().remaining(&document,0).as_secs_f64()}),
            )
        }
        Some("export") => {
            let input = rest
                .first()
                .map(PathBuf::from)
                .ok_or("Usage: vrx export FILE OUTPUT [WPM] [RGB]")?;
            let output = rest
                .get(1)
                .map(PathBuf::from)
                .ok_or("Usage: vrx export FILE OUTPUT [WPM] [RGB]")?;
            // CLI exports use explicit defaults, never silently depend on GUI state.
            let mut config = Config::default();
            if let Some(wpm) = rest.get(2) {
                config.timing.wpm = wpm
                    .to_str()
                    .ok_or("WPM must be UTF-8")?
                    .parse()
                    .map_err(|_| "WPM must be an integer")?;
            }
            if let Some(background) = rest.get(3) {
                config.export_background = background
                    .to_str()
                    .ok_or("RGB must be UTF-8 hex")?
                    .trim_start_matches('#')
                    .into();
                vrx::export::background(&config)?;
            }
            config.sanitize();
            let document = read(&input, config.orp)?;
            if output.exists() {
                return Err("Output already exists. Choose a new filename.".into());
            }
            vrx::export::export(&document, &config, &output, &AtomicBool::new(false))?;
            json(
                &serde_json::json!({"ok":true,"output":output,"words":document.len(),"nominal_wpm":config.timing.wpm,"adaptive":config.timing.adaptive}),
            )
        }
        Some("diagnose") => {
            let samples = rest
                .first()
                .and_then(|s| s.to_str())
                .map(|s| s.parse::<usize>().map_err(|_| "SAMPLES must be an integer"))
                .transpose()?
                .unwrap_or(30)
                .clamp(5, 10_000);
            diagnose(samples)
        }
        Some("api") => api(rest.first().map(PathBuf::from)),
        Some(s) if s.starts_with('-') => Err(format!("Unknown option {s}. Use --help.")),
        _ => crate::app::run(first.map(PathBuf::from)).map_err(|e| e.to_string()),
    }
}
fn read(path: &Path, strategy: Strategy) -> Result<Document> {
    import::load(path, strategy, &AtomicBool::new(false))
}
fn write_stdout(bytes: &[u8]) -> Result<()> {
    io::stdout()
        .lock()
        .write_all(bytes)
        .map_err(|e| e.to_string())
}
fn json(value: &impl Serialize) -> Result<()> {
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, value).map_err(|e| e.to_string())?;
    out.write_all(b"\n")
        .and_then(|_| out.flush())
        .map_err(|e| e.to_string())
}
fn diagnose(samples: usize) -> Result<()> {
    let source = "word ".repeat(samples + 2);
    let document = Document::parse(source, &Strategy::Classic)?.shared();
    for wpm in [200, 400, 800, 1200, 2000, 4000] {
        let timing = Timing {
            wpm,
            adaptive: false,
            ..Timing::default()
        };
        let mut playback = Playback::new(document.clone(), timing);
        let epoch = Instant::now();
        playback.play(Duration::ZERO);
        let mut lateness = Vec::with_capacity(samples);
        let mut extensions = 0;
        for _ in 0..samples {
            let Some(ticket) = playback.ticket() else {
                break;
            };
            let deadline = epoch + ticket.deadline;
            loop {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }
                std::thread::park_timeout(deadline.saturating_duration_since(now));
            }
            let now = epoch.elapsed();
            lateness.push(
                now.saturating_sub(ticket.deadline)
                    .as_nanos()
                    .min(u128::from(u64::MAX)) as u64,
            );
            playback.advance(ticket, now);
            extensions = playback.diagnostics.schedule_extensions;
        }
        lateness.sort_unstable();
        let percentile = |p: usize| {
            lateness
                .get((lateness.len().saturating_sub(1) * p).div_ceil(100))
                .copied()
                .unwrap_or(0) as f64
                / 1e6
        };
        json(
            &serde_json::json!({"metric":"OS absolute-deadline wake lateness; no GUI/display","wpm":wpm,"samples":lateness.len(),"p50_ms":percentile(50),"p95_ms":percentile(95),"p99_ms":percentile(99),"max_ms":percentile(100),"schedule_extensions":extensions,"elapsed_seconds":epoch.elapsed().as_secs_f64()}),
        )?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Status,
    Load {
        path: PathBuf,
    },
    Text {
        text: String,
    },
    Play,
    Pause,
    Restart,
    Seek {
        index: usize,
    },
    Jump {
        words: i64,
    },
    Speed {
        wpm: u32,
    },
    Adaptive {
        enabled: bool,
    },
    /// Deterministically move the virtual clock. Never sleeps and never moves back.
    Advance {
        nanoseconds: u64,
    },
    Export {
        path: PathBuf,
        #[serde(default)]
        background: Option<String>,
    },
    Shutdown,
}
fn api(path: Option<PathBuf>) -> Result<()> {
    let config = Config::default();
    let document = match path {
        Some(path) => read(&path, config.orp)?,
        None => Document::default(),
    };
    let mut playback = Playback::new(document.shared(), config.timing);
    let mut now = Duration::ZERO;
    json(
        &serde_json::json!({"protocol":1,"version":vrx::VERSION,"clock":"virtual nanoseconds; advance explicitly","ready":true}),
    )?;
    let mut input = io::stdin().lock();
    loop {
        // Cap each request at 8 MiB. read_line would otherwise permit unbounded
        // allocation before validating an agent-provided request.
        let mut line = Vec::new();
        let mut oversized = false;
        loop {
            let buffer = input.fill_buf().map_err(|e| e.to_string())?;
            if buffer.is_empty() {
                break;
            }
            let newline = buffer.iter().position(|b| *b == b'\n');
            let take = newline.map_or(buffer.len(), |i| i + 1);
            if line.len().saturating_add(take) <= 8 * 1024 * 1024 {
                line.extend_from_slice(buffer.get(..take).ok_or("Invalid input chunk")?);
            } else {
                oversized = true;
            }
            input.consume(take);
            if newline.is_some() {
                break;
            }
        }
        if line.is_empty() && !oversized {
            break;
        }
        if oversized {
            json(&serde_json::json!({"ok":false,"error":"Request exceeds 8 MiB"}))?;
            continue;
        }
        let request = serde_json::from_slice::<Command>(&line);
        let command = match request {
            Ok(command) => command,
            Err(e) => {
                json(&serde_json::json!({"ok":false,"error":e.to_string()}))?;
                continue;
            }
        };
        let shutdown = matches!(command, Command::Shutdown);
        let result: Result<()> = (|| {
            match command {
                Command::Status => {}
                Command::Play => playback.play(now),
                Command::Pause => playback.pause(now),
                Command::Restart => playback.restart(now),
                Command::Seek { index } => playback.seek(index, now),
                Command::Jump { words } => playback.jump(words, now),
                Command::Speed { wpm } => {
                    let mut timing = playback.timing().clone();
                    timing.wpm = wpm;
                    playback.set_timing(timing, now);
                }
                Command::Adaptive { enabled } => {
                    let mut timing = playback.timing().clone();
                    timing.adaptive = enabled;
                    playback.set_timing(timing, now);
                }
                Command::Load { path } => {
                    playback.replace(read(&path, Strategy::Classic)?.shared())
                }
                Command::Text { text } => {
                    playback.replace(Document::parse(text, &Strategy::Classic)?.shared())
                }
                Command::Advance { nanoseconds } => {
                    let target = now.saturating_add(Duration::from_nanos(nanoseconds));
                    // Preserve semantics: process each exact virtual deadline.
                    // Bounded to keep a pathological command from monopolizing input.
                    let mut iterations = 0usize;
                    while let Some(ticket) = playback.ticket() {
                        if ticket.deadline > target {
                            break;
                        }
                        if iterations >= 1_000_000 {
                            return Err(
                                "Advance exceeds one million transitions; split the request".into(),
                            );
                        }
                        now = ticket.deadline;
                        playback.advance(ticket, now);
                        iterations += 1;
                    }
                    now = target;
                }
                Command::Export { path, background } => {
                    if path.exists() {
                        return Err("Output already exists".into());
                    }
                    let mut config = Config {
                        timing: playback.timing().clone(),
                        ..Config::default()
                    };
                    if let Some(background) = background {
                        config.export_background = background.trim_start_matches('#').into();
                    }
                    vrx::export::export(
                        playback.document(),
                        &config,
                        &path,
                        &AtomicBool::new(false),
                    )?;
                }
                Command::Shutdown => playback.shutdown(now),
            }
            Ok(())
        })();
        match result {
            Ok(()) => json(
                &serde_json::json!({"ok":true,"state":playback.state(),"index":playback.index(),"word":playback.document().word(playback.index()),"words":playback.document().len(),"now_ns":now.as_nanos(),"deadline_ns":playback.ticket().map(|t|t.deadline.as_nanos()),"wpm":playback.timing().wpm,"adaptive":playback.timing().adaptive,"diagnostics":playback.diagnostics}),
            )?,
            Err(e) => json(&serde_json::json!({"ok":false,"error":e}))?,
        }
        if shutdown || playback.state() == State::Error {
            break;
        }
    }
    playback.shutdown(now);
    Ok(())
}
