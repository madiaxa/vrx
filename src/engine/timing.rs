use crate::text::{Boundary, Document, Token};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overrun {
    #[default]
    Preserve,
    MaintainPace,
}
impl std::fmt::Display for Overrun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Preserve => "Preserve every word",
            Self::MaintainPace => "Maintain pace (may skip)",
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timing {
    pub wpm: u32,
    pub adaptive: bool,
    pub long: u32,
    pub comma: u32,
    pub clause: u32,
    /// Zero inherits the legacy clause weight when loading an older config.
    pub colon: u32,
    pub semicolon: u32,
    pub quote: u32,
    pub parenthesis: u32,
    pub dash: u32,
    pub sentence: u32,
    pub paragraph: u32,
    /// Additional pause after tokens at paragraph/section boundaries.
    pub section_pause_ms: u32,
    pub overrun: Overrun,
}
impl Default for Timing {
    fn default() -> Self {
        Self {
            wpm: 300,
            adaptive: true,
            long: 1150,
            comma: 1300,
            clause: 1500,
            colon: 0,
            semicolon: 0,
            quote: 1150,
            parenthesis: 1200,
            dash: 1400,
            sentence: 1800,
            paragraph: 2000,
            section_pause_ms: 0,
            overrun: Overrun::Preserve,
        }
    }
}
impl Timing {
    pub fn sanitize(&mut self) {
        self.wpm = self.wpm.clamp(20, 10_000);
        if self.colon == 0 {
            self.colon = self.clause;
        }
        if self.semicolon == 0 {
            self.semicolon = self.clause;
        }
        for weight in [
            &mut self.long,
            &mut self.comma,
            &mut self.clause,
            &mut self.colon,
            &mut self.semicolon,
            &mut self.quote,
            &mut self.parenthesis,
            &mut self.dash,
            &mut self.sentence,
            &mut self.paragraph,
        ] {
            *weight = (*weight).clamp(1000, 5000);
        }
        self.section_pause_ms = self.section_pause_ms.min(10_000);
    }
    pub fn weight(&self, boundary: Boundary, long: bool) -> u32 {
        if !self.adaptive {
            return 1000;
        }
        let punctuation = match boundary {
            Boundary::Normal => 1000,
            Boundary::Comma => self.comma,
            Boundary::Clause => self.clause,
            Boundary::Sentence => self.sentence,
            Boundary::Paragraph => self.paragraph,
        };
        punctuation
            .max(if long { self.long } else { 1000 })
            .clamp(1000, 5000)
    }
    pub fn token_weight(&self, boundary: Boundary, long: bool, punctuation: u8) -> u32 {
        if !self.adaptive {
            return 1000;
        }
        let mut weight = if boundary == Boundary::Clause && punctuation & 12 != 0 {
            if long { self.long } else { 1000 }
        } else {
            self.weight(boundary, long)
        };
        for (flag, value) in [
            (1, self.quote),
            (2, self.parenthesis),
            (
                4,
                if self.colon == 0 {
                    self.clause
                } else {
                    self.colon
                },
            ),
            (
                8,
                if self.semicolon == 0 {
                    self.clause
                } else {
                    self.semicolon
                },
            ),
            (16, self.dash),
        ] {
            if punctuation & flag != 0 {
                weight = weight.max(value);
            }
        }
        weight.clamp(1000, 5000)
    }
    pub fn numerator(&self, token: &Token) -> u64 {
        60_000_000_000u64
            * u64::from(self.token_weight(token.boundary, token.graphemes >= 10, token.punctuation))
    }
    pub fn denominator(&self) -> u64 {
        u64::from(self.wpm.clamp(20, 10_000)) * 1000
    }
    pub fn duration(&self, token: &Token) -> Duration {
        Duration::from_nanos(self.numerator(token) / self.denominator())
            .saturating_add(self.section_pause(token.boundary))
    }
    pub fn section_pause(&self, boundary: Boundary) -> Duration {
        if self.adaptive && boundary == Boundary::Paragraph {
            Duration::from_millis(u64::from(self.section_pause_ms.min(10_000)))
        } else {
            Duration::ZERO
        }
    }
    pub fn remaining(&self, document: &Document, from: usize) -> Duration {
        let total: u128 = document
            .remaining_groups(from)
            .map(|(boundary, punctuation, long, count)| {
                let ns = 60_000_000_000u128
                    * u128::from(self.token_weight(boundary, long, punctuation))
                    / u128::from(self.denominator())
                    + self.section_pause(boundary).as_nanos();
                ns * count as u128
            })
            .sum();
        Duration::from_nanos(total.min(u128::from(u64::MAX)) as u64)
    }
}

/// Carries the division remainder so fractional nanoseconds cannot accumulate.
#[derive(Debug, Default)]
pub struct RationalClock {
    remainder: u64,
}
impl RationalClock {
    pub fn reset(&mut self) {
        self.remainder = 0;
    }
    pub fn duration(&mut self, timing: &Timing, token: &Token) -> Duration {
        let numerator = timing.numerator(token).saturating_add(self.remainder);
        let denominator = timing.denominator();
        self.remainder = numerator % denominator;
        Duration::from_nanos(numerator / denominator)
            .saturating_add(timing.section_pause(token.boundary))
    }
}
