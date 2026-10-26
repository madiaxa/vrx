use std::{path::PathBuf, sync::atomic::AtomicBool};
use vrx::{
    Result,
    config::Config,
    export::{self, Format},
    import,
    persistence::{self, Saved},
    text::{Document, orp::Strategy},
};
fn directory(name: &str) -> Result<PathBuf> {
    let path = std::env::temp_dir().join(format!("vrx-test-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}
#[test]
fn persistence_roundtrip_corruption_and_resume_identity() -> Result<()> {
    let dir = directory("persistence")?;
    let path = dir.join("settings.json");
    let mut saved = Saved::default();
    saved.config.timing.wpm = 789;
    saved.remember(PathBuf::from("book.txt"), 123, 999);
    persistence::save(&path, &saved)?;
    persistence::save(&path, &saved)?;
    let (loaded, warning) = persistence::load(&path);
    assert!(warning.is_none());
    assert_eq!(loaded.config.timing.wpm, 789);
    assert_eq!(
        loaded.position(std::path::Path::new("book.txt"), 123),
        Some(999)
    );
    assert!(
        loaded
            .position(std::path::Path::new("book.txt"), 124)
            .is_none()
    );
    std::fs::write(&path, b"broken").map_err(|e| e.to_string())?;
    assert!(persistence::load(&path).1.is_some());
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    Ok(())
}
#[test]
fn utf8_bom_malformed_and_unsupported_files() -> Result<()> {
    let dir = directory("import")?;
    let path = dir.join("text.txt");
    std::fs::write(&path, "\u{feff}hello café").map_err(|e| e.to_string())?;
    let d = import::load(&path, Strategy::Classic, &AtomicBool::new(false))?;
    assert_eq!(d.word(0), Some("hello"));
    std::fs::write(&path, [0xff, 0xfe]).map_err(|e| e.to_string())?;
    assert!(import::load(&path, Strategy::Classic, &AtomicBool::new(false)).is_err());
    assert!(
        import::load(
            &dir.join("book.pdf"),
            Strategy::Classic,
            &AtomicBool::new(false)
        )
        .is_err()
    );
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    Ok(())
}
#[test]
fn timed_subtitle_exports_and_cancellation() -> Result<()> {
    let d = Document::parse("one two three".into(), &Strategy::Classic)?;
    let mut config = Config::default();
    config.timing.adaptive = false;
    let mut srt = Vec::new();
    export::write_timeline(&d, &config, Format::Srt, &mut srt, &AtomicBool::new(false))?;
    let srt = String::from_utf8(srt).map_err(|e| e.to_string())?;
    assert!(srt.contains("00:00:00,000 --> 00:00:00,200\none"));
    assert!(srt.contains("00:00:00,400 --> 00:00:00,600\nthree"));
    let mut ass = Vec::new();
    export::write_timeline(&d, &config, Format::Ass, &mut ass, &AtomicBool::new(false))?;
    let ass = String::from_utf8(ass).map_err(|e| e.to_string())?;
    assert_eq!(ass.matches("Dialogue: 0,").count(), 3);
    assert!(
        export::write_timeline(
            &d,
            &config,
            Format::Ass,
            &mut Vec::new(),
            &AtomicBool::new(true)
        )
        .is_err()
    );
    let dir = directory("export")?;
    let path = dir.join("reading.ass");
    export::export(&d, &config, &path, &AtomicBool::new(false))?;
    assert!(path.exists());
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    Ok(())
}
#[test]
fn dozenal_release_labels() {
    assert_eq!(vrx::version::label(0, 1), "v0.0.1");
    assert_eq!(vrx::version::label(10, 11), "v0.↊.↋");
    assert_eq!(vrx::version::dozenal(12), "10");
}

#[test]
fn older_config_loads_new_preferences_and_keeps_version() -> Result<()> {
    let config: Config = serde_json::from_str(r#"{"font_size":360,"timing":{"clause":1750}}"#)
        .map_err(|e| e.to_string())?;
    assert!(config.focal_enabled);
    assert_eq!(config.radix, 10);
    assert_eq!(config.gui_text_size, 16.0);
    assert_eq!(
        config.keys.label(vrx::keybinds::Action::Personalize),
        "Escape; CmdOrCtrl+Comma"
    );
    let d = Document::parse("word:".into(), &Strategy::Classic)?;
    assert_eq!(
        config.timing.duration(d.token(0).ok_or("token")?),
        std::time::Duration::from_millis(350)
    );
    assert_eq!(vrx::VERSION, "v0.0.1");
    let mut config = config;
    config.radix = 3;
    config.focal_enabled = false;
    config.gui_text_size = 24.0;
    let encoded = serde_json::to_vec(&config).map_err(|e| e.to_string())?;
    let decoded: Config = serde_json::from_slice(&encoded).map_err(|e| e.to_string())?;
    assert_eq!(decoded.radix, 3);
    assert!(!decoded.focal_enabled);
    assert_eq!(decoded.gui_text_size, 24.0);
    Ok(())
}
