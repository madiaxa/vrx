use super::orp::OrpAlgorithm;
use crate::Result;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Boundary {
    Normal,
    Comma,
    Clause,
    Sentence,
    Paragraph,
}

/// Byte offsets are validated when constructing the document. No per-word strings.
#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub start: usize,
    pub end: usize,
    pub orp_start: usize,
    pub orp_end: usize,
    pub display_start: usize,
    pub display_end: usize,
    pub graphemes: u32,
    pub boundary: Boundary,
    pub punctuation: u8,
    /// Complex / bidirectional scripts are shaped as a whole and centered.
    pub whole_word_focus: bool,
}

#[derive(Debug)]
struct Category {
    boundary: Boundary,
    punctuation: u8,
    long: bool,
    starts: Vec<usize>,
}

#[derive(Debug, Default)]
pub struct Document {
    source: String,
    tokens: Vec<Token>,
    sentences: Vec<usize>,
    paragraphs: Vec<usize>,
    // Sparse category indexes make remaining-time queries logarithmic, independent
    // of document length. Each category accounts for long-word status as well.
    categories: Vec<Category>,
    pub fingerprint: u64,
}

impl Document {
    pub fn parse(source: String, algorithm: &dyn OrpAlgorithm) -> Result<Self> {
        Self::parse_cancellable(source, algorithm, &AtomicBool::new(false))
    }
    pub fn parse_cancellable(
        source: String,
        algorithm: &dyn OrpAlgorithm,
        cancel: &AtomicBool,
    ) -> Result<Self> {
        // A generous explicit bound gives predictable overflow behavior on 32-bit
        // platforms and avoids pretending arbitrary allocations are recoverable.
        if source.len() > 512 * 1024 * 1024 {
            return Err("Document exceeds the 512 MiB UTF-8 limit".into());
        }
        let mut tokens: Vec<Token> = Vec::new();
        let mut sentences = Vec::new();
        let mut paragraphs = Vec::new();
        let mut categories: Vec<Category> = Vec::new();
        let mut previous_end = 0;
        let mut sentence_start = true;
        let mut paragraph_start = true;
        let mut fingerprint = 0xcbf29ce484222325u64;
        for byte in source.bytes() {
            fingerprint = (fingerprint ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        let mut cursor = 0;
        while cursor < source.len() {
            if tokens.len().is_multiple_of(4096) && cancel.load(Ordering::Relaxed) {
                return Err("Loading cancelled".into());
            }
            let rest = source.get(cursor..).ok_or("Invalid tokenizer cursor")?;
            let skip = rest
                .grapheme_indices(true)
                .find(|(_, g)| !separator(g))
                .map_or(rest.len(), |(i, _)| i);
            cursor += skip;
            if cursor == source.len() {
                break;
            }
            let start = cursor;
            let rest = source.get(start..).ok_or("Invalid token boundary")?;
            let length = rest
                .grapheme_indices(true)
                .find_map(|(offset, grapheme)| {
                    if separator(grapheme) {
                        Some(offset)
                    } else if grapheme.starts_with(['/', '—']) {
                        // Keep the slash/em dash and any attached combining marks in
                        // the preceding token; never cut a grapheme cluster.
                        Some(offset + grapheme.len())
                    } else {
                        None
                    }
                })
                .unwrap_or(rest.len());
            cursor += length;
            let word = source.get(start..cursor).ok_or("Invalid UTF-8 token")?;
            let gap = source
                .get(previous_end..start)
                .ok_or("Invalid whitespace boundary")?;
            let paragraph_gap = has_paragraph_break(gap);
            if paragraph_gap {
                if let Some(previous) = tokens.last_mut() {
                    previous.boundary = Boundary::Paragraph;
                }
                paragraph_start = true;
                sentence_start = true;
            }
            if sentence_start {
                sentences.push(tokens.len());
            }
            if paragraph_start {
                paragraphs.push(tokens.len());
            }
            paragraph_start = false;
            let count = word.graphemes(true).count();
            let focus = algorithm.index(count).min(count.saturating_sub(1));
            let mut orp_start = start;
            let mut orp_end = cursor;
            let mut display_start = start;
            let mut display_end = cursor;
            // Bound shaping cost for adversarial single-token inputs. Preserve a
            // 256-grapheme excerpt around the focus, with no segmentation in view.
            let first = focus.saturating_sub(96);
            let last = first.saturating_add(256).min(count);
            for (i, (offset, grapheme)) in word.grapheme_indices(true).enumerate() {
                if i == focus {
                    orp_start = start + offset;
                    orp_end = orp_start + grapheme.len();
                }
                if count > 256 && i == first {
                    display_start = start + offset;
                }
                if count > 256 && i + 1 == last {
                    display_end = start + offset + grapheme.len();
                }
            }
            let trailing = word.trim_end_matches(['"', '\'', '”', '’', ')', ']', '}', '»']);
            let boundary = if trailing
                .ends_with(['.', '!', '?', '。', '！', '？', '…', '؟', '।', '॥'])
            {
                Boundary::Sentence
            } else if trailing.ends_with([';', ':', '；', '：', '؛']) {
                Boundary::Clause
            } else if trailing.ends_with([',', '，', '、', '،']) {
                Boundary::Comma
            } else {
                Boundary::Normal
            };
            sentence_start = boundary == Boundary::Sentence;
            let whole_word_focus = word.chars().any(|c| matches!(c as u32, 0x0590..=0x08ff | 0x0900..=0x109f | 0x1780..=0x17ff | 0xa800..=0xabff | 0xfb1d..=0xfdff | 0xfe70..=0xfeff | 0x200e..=0x200f | 0x202a..=0x202e | 0x2066..=0x2069));
            let mut punctuation = 0u8;
            if word.starts_with(['\"', '\'', '“', '‘', '«', '‹'])
                || word.ends_with(['\"', '\'', '”', '’', '»', '›'])
            {
                punctuation |= 1;
            }
            if word.contains(['(', ')', '（', '）', '[', ']', '{', '}']) {
                punctuation |= 2;
            }
            if trailing.ends_with([':', '：']) {
                punctuation |= 4;
            }
            if trailing.ends_with([';', '；', '؛']) {
                punctuation |= 8;
            }
            if word.contains(['—', '–']) || word == "--" {
                punctuation |= 16;
            }
            tokens.push(Token {
                start,
                end: cursor,
                orp_start,
                orp_end,
                display_start,
                display_end,
                graphemes: count.min(u32::MAX as usize) as u32,
                boundary,
                punctuation,
                whole_word_focus,
            });
            previous_end = cursor;
        }
        let mut lookup = [usize::MAX; 320];
        for (i, token) in tokens.iter().enumerate() {
            let long = token.graphemes >= 10;
            let key = (token.boundary as usize * 2 + usize::from(long)) * 32
                + usize::from(token.punctuation);
            let existing = lookup.get(key).copied().unwrap_or(usize::MAX);
            let group = if existing == usize::MAX {
                let index = categories.len();
                categories.push(Category {
                    boundary: token.boundary,
                    punctuation: token.punctuation,
                    long,
                    starts: Vec::new(),
                });
                if let Some(slot) = lookup.get_mut(key) {
                    *slot = index;
                }
                index
            } else {
                existing
            };
            if let Some(category) = categories.get_mut(group) {
                category.starts.push(i);
            }
        }
        Ok(Self {
            source,
            tokens,
            sentences,
            paragraphs,
            categories,
            fingerprint,
        })
    }
    pub fn len(&self) -> usize {
        self.tokens.len()
    }
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn token(&self, index: usize) -> Option<&Token> {
        self.tokens.get(index)
    }
    pub fn word(&self, index: usize) -> Option<&str> {
        self.token(index)
            .and_then(|t| self.source.get(t.start..t.end))
    }
    pub fn display(&self, index: usize) -> Option<(&str, usize, usize, bool)> {
        let t = self.token(index)?;
        Some((
            self.source.get(t.display_start..t.display_end)?,
            t.orp_start.saturating_sub(t.display_start),
            t.orp_end.saturating_sub(t.display_start),
            t.whole_word_focus,
        ))
    }
    pub fn sentence_start(&self, index: usize, forward: bool) -> usize {
        boundary_seek(&self.sentences, index, forward, self.len())
    }
    pub fn paragraph_start(&self, index: usize, forward: bool) -> usize {
        boundary_seek(&self.paragraphs, index, forward, self.len())
    }
    pub fn remaining_categories(&self, from: usize) -> [usize; 10] {
        let mut result = [0usize; 10];
        for (boundary, _, long, count) in self.remaining_groups(from) {
            if let Some(total) = result.get_mut(boundary as usize * 2 + usize::from(long)) {
                *total += count;
            }
        }
        result
    }
    pub fn remaining_groups(
        &self,
        from: usize,
    ) -> impl Iterator<Item = (Boundary, u8, bool, usize)> + '_ {
        self.categories.iter().map(move |c| {
            (
                c.boundary,
                c.punctuation,
                c.long,
                c.starts
                    .len()
                    .saturating_sub(c.starts.partition_point(|i| *i < from)),
            )
        })
    }
    pub fn estimated_bytes(&self) -> usize {
        self.source.capacity()
            + self.tokens.capacity() * std::mem::size_of::<Token>()
            + (self.sentences.capacity()
                + self.paragraphs.capacity()
                + self
                    .categories
                    .iter()
                    .map(|c| c.starts.capacity())
                    .sum::<usize>())
                * std::mem::size_of::<usize>()
    }
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

fn boundary_seek(starts: &[usize], index: usize, forward: bool, len: usize) -> usize {
    if forward {
        starts
            .get(starts.partition_point(|i| *i <= index))
            .copied()
            .unwrap_or(len.saturating_sub(1))
    } else {
        starts
            .get(starts.partition_point(|i| *i < index).saturating_sub(1))
            .copied()
            .unwrap_or(0)
    }
}

fn has_paragraph_break(gap: &str) -> bool {
    let mut chars = gap.chars().peekable();
    let mut lines = 0usize;
    while let Some(c) = chars.next() {
        if c == '\u{2029}' {
            return true;
        }
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                let _ = chars.next();
            }
            lines += 1;
        } else if matches!(c, '\n' | '\u{2028}' | '\u{85}') {
            lines += 1;
        }
        if lines >= 2 {
            return true;
        }
    }
    false
}

// Treat a whitespace-based grapheme (including decorative combining marks) as
// a whole separator. Never start a token inside the original source's cluster.
fn separator(grapheme: &str) -> bool {
    grapheme.chars().next().is_some_and(char::is_whitespace)
}
