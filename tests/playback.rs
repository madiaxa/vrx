use proptest::prelude::*;
use std::time::Duration;
use vrx::{
    Result,
    engine::{Overrun, Playback, State, Timing},
    text::{Document, orp::Strategy},
};
fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn reader() -> Result<Playback> {
    Ok(Playback::new(
        Document::parse("one two three four five".into(), &Strategy::Classic)?.shared(),
        Timing {
            adaptive: false,
            ..Timing::default()
        },
    ))
}

#[test]
fn start_end_and_last_word_duration() -> Result<()> {
    let mut p = reader()?;
    assert_eq!(p.state(), State::Ready);
    p.play(ms(0));
    for i in 1..5 {
        let ticket = p.ticket().ok_or("missing timer")?;
        assert!(p.advance(ticket, ms(i * 200)));
        assert_eq!(p.index(), i as usize);
    }
    assert_eq!(p.state(), State::Playing);
    let t = p.ticket().ok_or("last word timer")?;
    assert!(p.advance(t, ms(1000)));
    assert_eq!(p.state(), State::Finished);
    assert_eq!(p.index(), 4);
    p.play(ms(2000));
    assert_eq!(p.index(), 0);
    assert_eq!(p.state(), State::Playing);
    Ok(())
}
#[test]
fn pause_retains_word_and_remaining_fraction() -> Result<()> {
    let mut p = reader()?;
    p.play(ms(0));
    let stale = p.ticket().ok_or("timer")?;
    p.pause(ms(75));
    assert_eq!(p.state(), State::Paused);
    assert_eq!(p.index(), 0);
    assert!(p.ticket().is_none());
    assert!(!p.advance(stale, ms(1000)));
    p.play(ms(10_000));
    assert_eq!(p.ticket().map(|t| t.deadline), Some(ms(10_125)));
    Ok(())
}
#[test]
fn jumps_seek_and_speed_never_interrupt_playback() -> Result<()> {
    let mut p = reader()?;
    p.play(ms(0));
    p.jump(2, ms(20));
    assert_eq!(p.index(), 2);
    assert_eq!(p.state(), State::Playing);
    p.jump(-1, ms(40));
    assert_eq!(p.index(), 1);
    let mut timing = p.timing().clone();
    timing.wpm = 600;
    p.set_timing(timing, ms(140));
    assert_eq!(p.index(), 1);
    assert_eq!(p.state(), State::Playing);
    assert_eq!(p.ticket().map(|t| t.deadline), Some(ms(190)));
    p.jump(i64::MAX, ms(150));
    assert_eq!(p.index(), 4);
    p.jump(i64::MIN, ms(160));
    assert_eq!(p.index(), 0);
    p.restart(ms(170));
    assert_eq!(p.index(), 0);
    assert_eq!(p.state(), State::Playing);
    Ok(())
}
#[test]
fn speed_change_while_paused_preserves_fraction() -> Result<()> {
    let mut p = reader()?;
    p.play(ms(0));
    p.pause(ms(100));
    let mut timing = p.timing().clone();
    timing.wpm = 600;
    p.set_timing(timing, ms(300));
    p.play(ms(1000));
    assert_eq!(p.ticket().map(|t| t.deadline), Some(ms(1050)));
    Ok(())
}
#[test]
fn document_replacement_and_shutdown_cancel_timers() -> Result<()> {
    let mut p = reader()?;
    p.play(ms(0));
    let old = p.ticket().ok_or("timer")?;
    p.replace(Document::parse("new".into(), &Strategy::Classic)?.shared());
    p.play(ms(0));
    assert!(!p.advance(old, ms(1000)));
    assert_eq!(p.index(), 0);
    let new = p.ticket().ok_or("timer")?;
    p.shutdown(ms(25));
    assert!(!p.advance(new, ms(200)));
    assert!(p.ticket().is_none());
    Ok(())
}
#[test]
fn preserve_never_skips_on_large_stalls() -> Result<()> {
    let mut p = reader()?;
    p.play(ms(0));
    let t = p.ticket().ok_or("timer")?;
    p.advance(t, ms(10_000));
    assert_eq!(p.index(), 1);
    assert_eq!(p.ticket().map(|t| t.deadline), Some(ms(10_200)));
    assert_eq!(p.diagnostics.skipped_words, 0);
    assert!(!p.advance(t, ms(10_001)));
    assert_eq!(p.index(), 1);
    Ok(())
}
#[test]
fn maintain_pace_catches_up_expired_words() -> Result<()> {
    let mut p = reader()?;
    let mut timing = p.timing().clone();
    timing.overrun = Overrun::MaintainPace;
    p.set_timing(timing, ms(0));
    p.play(ms(0));
    let t = p.ticket().ok_or("timer")?;
    p.advance(t, ms(650));
    assert_eq!(p.index(), 3);
    assert_eq!(p.ticket().map(|t| t.deadline), Some(ms(800)));
    assert_eq!(p.diagnostics.skipped_words, 2);
    Ok(())
}
#[test]
fn early_and_duplicate_events_are_ignored() -> Result<()> {
    let mut p = reader()?;
    p.play(ms(0));
    let t = p.ticket().ok_or("timer")?;
    assert!(!p.advance(t, ms(199)));
    assert!(p.advance(t, ms(200)));
    assert!(!p.advance(t, ms(200)));
    assert_eq!(p.index(), 1);
    Ok(())
}
#[test]
fn empty_and_error_lifecycle() {
    let mut p = Playback::new(Document::default().shared(), Timing::default());
    p.play(ms(0));
    p.seek(999, ms(0));
    assert_eq!(p.state(), State::Empty);
    assert!(p.ticket().is_none());
    p.fail();
    p.play(ms(0));
    assert_eq!(p.state(), State::Error);
}
proptest! {
    #[test]
    fn random_controls_keep_valid_indices(actions in prop::collection::vec((0u8..6, any::<u16>()), 0..1000)) {
        if let Ok(mut p) = reader() {
            let mut now = Duration::ZERO;
            for (kind, value) in actions {
                now = now.saturating_add(Duration::from_millis(u64::from(value)));
                match kind { 0 => p.toggle(now), 1 => p.jump(i64::from(value) - 32_000, now), 2 => p.seek(usize::from(value),now), 3 => p.restart(now), 4 => { if let Some(t) = p.ticket() {p.advance(t,now);} }, _ => { let mut t = p.timing().clone(); t.wpm = u32::from(value); p.set_timing(t,now); } }
                prop_assert!(p.index() < p.document().len());
                prop_assert_eq!(p.ticket().is_some(), p.state() == State::Playing);
            }
        }
    }
}
