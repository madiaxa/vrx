//! Streaming subtitle timelines and optional FFmpeg MP4 rendering.
use crate::{Result, config::Config, engine::timing::RationalClock, text::Document};
use std::{
    io::{BufWriter, Write},
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ass,
    Srt,
    Mp4,
}
impl Format {
    pub fn from_path(path: &Path) -> Result<Self> {
        match path
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("ass") => Ok(Self::Ass),
            Some("srt") => Ok(Self::Srt),
            Some("mp4") => Ok(Self::Mp4),
            _ => Err("Choose .ass, .srt, or .mp4".into()),
        }
    }
}
pub fn export(
    document: &Document,
    config: &Config,
    path: &Path,
    cancel: &AtomicBool,
) -> Result<()> {
    if document.is_empty() {
        return Err("There are no words to export".into());
    }
    let format = Format::from_path(path)?;
    if format == Format::Mp4 {
        return mp4(document, config, path, cancel);
    }
    // Stream into a sibling temporary file, then atomically publish it. This
    // avoids buffering a book-sized subtitle file in memory.
    let temporary = path.with_extension(format!("{}.partial", std::process::id()));
    let mut created = false;
    let result = (|| {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        created = true;
        let mut out = BufWriter::new(file);
        write_timeline(document, config, format, &mut out, cancel)?;
        out.flush().map_err(|e| e.to_string())?;
        out.get_ref().sync_all().map_err(|e| e.to_string())?;
        drop(out);
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() && created {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
pub fn write_timeline(
    document: &Document,
    config: &Config,
    format: Format,
    out: &mut impl Write,
    cancel: &AtomicBool,
) -> Result<()> {
    let mut timing = config.timing.clone();
    timing.sanitize();
    let mut clock = RationalClock::default();
    let mut elapsed = Duration::ZERO;
    if format != Format::Srt {
        let canvas = background(config)?;
        let (_, foreground, accent) = colors(config);
        let family = match config.font {
            crate::config::FontFamily::Sans | crate::config::FontFamily::Arial => "Arial",
            crate::config::FontFamily::Serif => "Times New Roman",
            crate::config::FontFamily::Mono => "Courier New",
            crate::config::FontFamily::Georgia => "Georgia",
        };
        writeln!(out, "[Script Info]\nTitle: VRX {}\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\nWrapStyle: 2\n\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\nStyle: Default,{family},{},&H00{},&H00{},&H00000000,&H00000000,{},0,0,0,100,100,0,0,1,0,0,5,60,60,60,1\n\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text", crate::VERSION, config.font_size * 2.0, ass_color(&foreground), ass_color(&accent), if config.bold { -1 } else { 0 }).map_err(|e| e.to_string())?;
        // A full-frame vector rectangle keeps ASS opaque in video editors too.
        // Word events remain on layer 0, above this independent canvas.
        writeln!(out,
            "Dialogue: -1,0:00:00.00,{},Default,,0,0,0,,{{\\an7\\pos(0,0)\\bord0\\shad0\\1c&H{}&\\alpha&H00&\\p1}}m 0 0 l 1920 0 1920 1080 0 1080{{\\p0}}",
            ass_time(timing.remaining(document, 0)), ass_color(canvas)
        ).map_err(|e| e.to_string())?;
    }
    for index in 0..document.len() {
        if cancel.load(Ordering::Relaxed) {
            return Err("Export cancelled".into());
        }
        let token = document.token(index).ok_or("Missing export token")?;
        let end = elapsed.saturating_add(clock.duration(&timing, token));
        let word = document.word(index).ok_or("Missing export word")?;
        if format == Format::Srt {
            writeln!(
                out,
                "{}\n{} --> {}\n{}\n",
                index + 1,
                srt_time(elapsed),
                srt_time(end),
                word.replace(['\r', '\n'], " ")
            )
            .map_err(|e| e.to_string())?;
        } else {
            // ASS/libass applies its own shaping. Full-word centering preserves
            // complex scripts; desktop per-grapheme anchoring is intentionally
            // not claimed for the interoperable subtitle representation.
            let (_, foreground, accent) = colors(config);
            let start = token.orp_start.saturating_sub(token.start);
            let finish = token.orp_end.saturating_sub(token.start);
            let escaped = if token.whole_word_focus || !config.focal_enabled {
                ass_escape(word)
            } else {
                format!(
                    "{}{{\\c&H{}&}}{}{{\\c&H{}&}}{}",
                    ass_escape(word.get(..start).unwrap_or("")),
                    ass_color(&accent),
                    ass_escape(word.get(start..finish).unwrap_or("")),
                    ass_color(&foreground),
                    ass_escape(word.get(finish..).unwrap_or(""))
                )
            };
            writeln!(
                out,
                "Dialogue: 0,{},{},Default,,0,0,0,,{{\\pos({},{})}}{}",
                ass_time(elapsed),
                ass_time(end),
                (1920.0 * config.focal_x) as u32,
                (1080.0 * config.vertical) as u32,
                escaped
            )
            .map_err(|e| e.to_string())?;
        }
        elapsed = end;
    }
    Ok(())
}
fn ass_escape(word: &str) -> String {
    word.replace('\\', "＼")
        .replace('{', "｛")
        .replace('}', "｝")
        .replace(['\r', '\n'], " ")
}
fn ass_color(value: &str) -> String {
    if value.len() == 6 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        format!(
            "{}{}{}",
            value.get(4..6).unwrap_or("FF"),
            value.get(2..4).unwrap_or("FF"),
            value.get(..2).unwrap_or("FF")
        )
    } else {
        "FFFFFF".into()
    }
}
/// Validated opaque RGB canvas; never derived from the reading theme.
pub fn background(config: &Config) -> Result<&str> {
    let value = config.export_background.as_str();
    if value.len() == 6 && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(value)
    } else {
        Err("Export background must contain six RGB hex digits (for example 00FF00)".into())
    }
}
fn colors(config: &Config) -> (String, String, String) {
    use crate::config::Appearance;
    let (background, foreground, accent) = match config.appearance {
        Appearance::Dark => ("10151D".into(), "E9EDF4".into(), "FFAD7D".into()),
        Appearance::Light => ("F6F4EF".into(), "20252D".into(), "A43D19".into()),
        Appearance::Contrast => ("000000".into(), "FFFFFF".into(), "FFFF00".into()),
        Appearance::Custom => (
            config.background.clone(),
            config.foreground.clone(),
            config.accent.clone(),
        ),
    };
    (
        background,
        foreground,
        config
            .focal_color_rgb()
            .map(str::to_owned)
            .unwrap_or(accent),
    )
}
pub fn ass_time(duration: Duration) -> String {
    let cs = duration.as_millis().div_ceil(10);
    format!(
        "{}:{:02}:{:02}.{:02}",
        cs / 360_000,
        cs / 6000 % 60,
        cs / 100 % 60,
        cs % 100
    )
}
pub fn srt_time(duration: Duration) -> String {
    let ms = duration.as_millis();
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}
fn mp4(document: &Document, config: &Config, path: &Path, cancel: &AtomicBool) -> Result<()> {
    let ffmpeg = std::env::var_os("VRX_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let available = Command::new(&ffmpeg)
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            "MP4 requires FFmpeg with libass and libx264. Install FFmpeg or export .ass/.srt."
                .to_string()
        })?;
    if !available.success() {
        return Err("FFmpeg is unavailable; export .ass or .srt instead".into());
    }
    let temporary_dir = std::env::temp_dir().join(format!(
        "vrx-video-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    ));
    std::fs::create_dir(&temporary_dir).map_err(|e| e.to_string())?;
    let result = (|| {
        let timeline = temporary_dir.join("reading.ass");
        export(document, config, &timeline, cancel)?;
        let background = background(config)?;
        let duration = config.timing.remaining(document, 0).as_secs_f64();
        let output = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path)
        };
        let partial = output.with_extension(format!("{}.partial.mp4", std::process::id()));
        let log =
            std::fs::File::create(temporary_dir.join("ffmpeg.log")).map_err(|e| e.to_string())?;
        let mut child = Command::new(&ffmpeg)
            .current_dir(&temporary_dir)
            .args([
                "-nostdin",
                "-n",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c=0x{background}:s=1920x1080:r=60"),
                "-vf",
                "ass=reading.ass",
                "-t",
                &format!("{duration:.9}"),
                "-c:v",
                "libx264",
                "-threads",
                "2",
                "-pix_fmt",
                "yuv420p",
                "-movflags",
                "+faststart",
            ])
            .arg(&partial)
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .map_err(|e| e.to_string())?;
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&partial);
                return Err("Export cancelled".into());
            }
            let status = match child.try_wait() {
                Ok(status) => status,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&partial);
                    return Err(error.to_string());
                }
            };
            match status {
                Some(status) if status.success() => {
                    std::fs::rename(&partial, &output).map_err(|e| e.to_string())?;
                    break;
                }
                Some(_) => {
                    let log = std::fs::read_to_string(temporary_dir.join("ffmpeg.log"))
                        .unwrap_or_default();
                    let _ = std::fs::remove_file(&partial);
                    return Err(format!(
                        "FFmpeg failed: {}",
                        log.chars().take(2000).collect::<String>()
                    ));
                }
                None => std::thread::park_timeout(Duration::from_millis(100)),
            }
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&temporary_dir);
    result
}
