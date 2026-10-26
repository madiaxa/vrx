use std::time::Duration;
use vrx::{
    Result,
    engine::{Playback, Timing, timing::RationalClock},
    text::{Document, orp::Strategy},
};

#[test]
fn base_durations_and_adaptive_maximum_not_product() -> Result<()> {
    let d = Document::parse(
        "word longlonglong, done.\n\nnext".into(),
        &Strategy::Classic,
    )?;
    let t = Timing::default();
    assert_eq!(
        t.duration(d.token(0).ok_or("word")?),
        Duration::from_millis(200)
    );
    assert_eq!(
        t.duration(d.token(1).ok_or("long")?),
        Duration::from_millis(260)
    );
    assert_eq!(
        t.duration(d.token(2).ok_or("paragraph")?),
        Duration::from_millis(400)
    );
    for (wpm, ms) in [(300, 200), (600, 100), (1200, 50), (2000, 30)] {
        let strict = Timing {
            wpm,
            adaptive: false,
            ..Timing::default()
        };
        assert_eq!(
            strict.duration(d.token(2).ok_or("token")?),
            Duration::from_millis(ms)
        );
    }
    Ok(())
}
#[test]
fn rational_clock_has_no_accumulating_rounding_error() -> Result<()> {
    let d = Document::parse("word".into(), &Strategy::Classic)?;
    let token = d.token(0).ok_or("token")?;
    let timing = Timing {
        wpm: 799,
        adaptive: false,
        ..Timing::default()
    };
    let mut clock = RationalClock::default();
    let mut sum = Duration::ZERO;
    for _ in 0..100_000 {
        sum += clock.duration(&timing, token);
    }
    assert_eq!(sum.as_nanos(), 60_000_000_000u128 * 100_000 / 799);
    Ok(())
}
#[test]
fn ordinary_wake_jitter_does_not_accumulate() -> Result<()> {
    let d = Document::parse("word ".repeat(1000), &Strategy::Classic)?;
    let mut p = Playback::new(
        d.shared(),
        Timing {
            adaptive: false,
            ..Timing::default()
        },
    );
    p.play(Duration::ZERO);
    for i in 1..=999u64 {
        let t = p.ticket().ok_or("timer")?;
        assert_eq!(t.deadline, Duration::from_millis(i * 200));
        p.advance(t, t.deadline + Duration::from_millis(5));
    }
    assert_eq!(p.diagnostics.schedule_extensions, 0);
    Ok(())
}
#[test]
fn remaining_matches_full_schedule_with_section_pauses() -> Result<()> {
    let d = Document::parse(
        "Hi, longlonglong; done.\n\nNext, end!".into(),
        &Strategy::Classic,
    )?;
    let t = Timing {
        section_pause_ms: 500,
        ..Timing::default()
    };
    let expected: Duration = (0..d.len())
        .filter_map(|i| d.token(i))
        .map(|token| t.duration(token))
        .sum();
    assert_eq!(t.remaining(&d, 0), expected);
    assert_eq!(t.remaining(&d, d.len()), Duration::ZERO);
    Ok(())
}

#[test]
fn independent_punctuation_pauses_and_remaining_estimate() -> Result<()> {
    let document = Document::parse(
        "hello: hello; \"hello\" (hello) hello— next".into(),
        &Strategy::Classic,
    )?;
    let timing = Timing {
        colon: 2100,
        semicolon: 2200,
        quote: 2300,
        parenthesis: 2400,
        dash: 2500,
        ..Timing::default()
    };
    for (i, millis) in [420, 440, 460, 480, 500, 200].into_iter().enumerate() {
        assert_eq!(
            timing.duration(document.token(i).ok_or("token")?),
            Duration::from_millis(millis)
        );
    }
    let expected: Duration = (0..document.len())
        .filter_map(|i| document.token(i))
        .map(|t| timing.duration(t))
        .sum();
    assert_eq!(timing.remaining(&document, 0), expected);
    let strict = Timing {
        adaptive: false,
        ..timing
    };
    assert_eq!(
        strict.duration(document.token(2).ok_or("quote")?),
        Duration::from_millis(200)
    );
    Ok(())
}
