use proptest::prelude::*;
use unicode_segmentation::UnicodeSegmentation;
use vrx::{
    Result,
    import::{Importer, Markdown},
    text::{
        Boundary, Document,
        orp::{OrpAlgorithm, Strategy},
    },
};

#[test]
fn whitespace_and_paragraphs() -> Result<()> {
    let d = Document::parse(
        " one\t two\u{a0}three\r\nfour.\r\n \r\nfive\u{2029}six ".into(),
        &Strategy::Classic,
    )?;
    assert_eq!(d.len(), 6);
    assert_eq!(d.word(2), Some("three"));
    assert_eq!(d.token(3).map(|t| t.boundary), Some(Boundary::Paragraph));
    assert_eq!(d.token(4).map(|t| t.boundary), Some(Boundary::Paragraph));
    assert_eq!(d.paragraph_start(2, true), 4);
    assert_eq!(d.sentence_start(4, true), 5);
    assert_eq!(d.sentence_start(5, false), 4);
    Ok(())
}
#[test]
fn orp_default_mapping_and_alternatives() {
    for (n, expected) in [
        (0, 0),
        (1, 0),
        (2, 1),
        (5, 1),
        (6, 2),
        (9, 2),
        (10, 3),
        (13, 3),
        (14, 5),
        (20, 7),
    ] {
        assert_eq!(Strategy::Classic.index(n), expected);
    }
    for n in 0..1000 {
        for strategy in Strategy::ALL {
            assert!(strategy.index(n) <= n.saturating_sub(1));
        }
    }
}
#[test]
fn emoji_and_combining_marks_are_indivisible() -> Result<()> {
    let d = Document::parse("e\u{301}a 👨‍👩‍👧‍👦✨ 🇺🇸🇨🇦".into(), &Strategy::Classic)?;
    let focus = |i| {
        d.token(i)
            .and_then(|t| d.source().get(t.orp_start..t.orp_end))
    };
    assert_eq!(focus(0), Some("a"));
    assert_eq!(focus(1), Some("✨"));
    assert_eq!(focus(2), Some("🇨🇦"));
    assert_eq!(d.token(1).map(|t| t.graphemes), Some(2));
    Ok(())
}
#[test]
fn punctuation_and_unicode_sentence_stops() -> Result<()> {
    let d = Document::parse(
        "Hello, wait; listen: done!” 次。 مرحبا؟".into(),
        &Strategy::Classic,
    )?;
    let kinds: Vec<_> = (0..d.len())
        .filter_map(|i| d.token(i).map(|t| t.boundary))
        .collect();
    assert_eq!(
        kinds,
        [
            Boundary::Comma,
            Boundary::Clause,
            Boundary::Clause,
            Boundary::Sentence,
            Boundary::Sentence,
            Boundary::Sentence
        ]
    );
    Ok(())
}
#[test]
fn scripts_have_explicit_fallback() -> Result<()> {
    let d = Document::parse(
        "العربية שלום हिन्दी 中文没有空格 ภาษาไทย".into(),
        &Strategy::Classic,
    )?;
    assert_eq!(d.len(), 5); // Whitespace segmentation, no dictionary guessing.
    assert!(d.token(0).is_some_and(|t| t.whole_word_focus));
    assert!(d.token(1).is_some_and(|t| t.whole_word_focus));
    assert!(d.token(2).is_some_and(|t| t.whole_word_focus));
    assert_eq!(d.word(3), Some("中文没有空格"));
    Ok(())
}
#[test]
fn empty_single_and_long_tokens() -> Result<()> {
    for source in ["", " \n\t"] {
        assert!(Document::parse(source.into(), &Strategy::Classic)?.is_empty());
    }
    assert_eq!(Document::parse("one".into(), &Strategy::Classic)?.len(), 1);
    let d = Document::parse("e\u{301}".repeat(100_000), &Strategy::Classic)?;
    let t = d.token(0).ok_or("missing token")?;
    assert_eq!(t.graphemes, 100_000);
    let (display, start, end, _) = d.display(0).ok_or("missing display")?;
    assert_eq!(display.graphemes(true).count(), 256);
    assert_eq!(display.get(start..end), Some("e\u{301}"));
    assert_eq!(d.word(0).map(str::len), Some(300_000));
    Ok(())
}
#[test]
fn markdown_is_readable() -> Result<()> {
    let readable = Markdown.convert("# A title\n\nRead **bold** and [a link](https://example.com).\n\n- item\n- next\n\n`code`\n\n<div>hidden HTML block</div>".into())?;
    assert!(readable.contains("Read bold and a link."));
    assert!(!readable.contains("https://"));
    let d = Document::parse(readable, &Strategy::Classic)?;
    assert_eq!(d.word(0), Some("A"));
    assert_eq!(d.token(1).map(|t| t.boundary), Some(Boundary::Paragraph));
    Ok(())
}
#[test]
fn cancellation_is_observed() {
    assert!(
        Document::parse_cancellable(
            "word ".repeat(5000),
            &Strategy::Classic,
            &std::sync::atomic::AtomicBool::new(true)
        )
        .is_err()
    );
}
proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]
    #[test]
    fn arbitrary_unicode_has_valid_offsets(source in ".{0,1024}") {
        let parsed = Document::parse(source.clone(), &Strategy::Classic);
        prop_assert!(parsed.is_ok());
        if let Ok(d) = parsed {
            let mut boundaries: std::collections::HashSet<usize> = source.grapheme_indices(true).map(|(i,_)|i).collect();
            boundaries.insert(source.len());
            for i in 0..d.len() {
                if let Some(t) = d.token(i) {
                    prop_assert!(boundaries.contains(&t.start) && boundaries.contains(&t.end));
                    prop_assert!(boundaries.contains(&t.orp_start) && boundaries.contains(&t.orp_end));
                    prop_assert!(t.start <= t.orp_start && t.orp_start < t.orp_end && t.orp_end <= t.end);
                    let focus = d.source().get(t.orp_start..t.orp_end);
                    prop_assert!(focus.is_some_and(|s| s.graphemes(true).count() == 1));
                }
            }
        }
    }
}

#[test]
fn whitespace_combining_cluster_is_never_split() -> Result<()> {
    let d = Document::parse("one \u{301}two\r\rthree".into(), &Strategy::Classic)?;
    assert_eq!(d.word(0), Some("one"));
    assert_eq!(d.word(1), Some("two"));
    assert_eq!(d.token(1).map(|t| t.boundary), Some(Boundary::Paragraph));
    Ok(())
}

#[test]
fn revision_center_forty_percent_and_em_dash() -> Result<()> {
    for (length, index) in [(1, 0), (2, 0), (3, 1), (4, 1), (5, 2), (6, 2)] {
        assert_eq!(Strategy::Center.index(length), index);
    }
    assert_eq!(Strategy::FortyPercent.index(10), 3);
    let legacy: Strategy = serde_json::from_str("\"thirty_percent\"").map_err(|e| e.to_string())?;
    assert_eq!(legacy, Strategy::FortyPercent);
    let d = Document::parse("true/false—maybe—yes".into(), &Strategy::Classic)?;
    let words: Vec<_> = (0..d.len()).filter_map(|i| d.word(i)).collect();
    assert_eq!(words, ["true/", "false—", "maybe—", "yes"]);
    assert_eq!(d.token(1).map(|t| t.punctuation & 16), Some(16));
    Ok(())
}
