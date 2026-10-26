use super::{Overrun, Timing, timing::RationalClock};
use crate::text::Document;
use serde::Serialize;
use std::{sync::Arc, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Empty,
    Ready,
    Playing,
    Paused,
    Finished,
    Error,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Ticket {
    pub generation: u64,
    pub deadline: Duration,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Diagnostics {
    pub updates: u64,
    pub late_events: u64,
    pub schedule_extensions: u64,
    pub skipped_words: u64,
    pub last_lateness_ns: u64,
    pub max_lateness_ns: u64,
    pub last_update_ns: u64,
    pub last_submission_ns: u64,
}

/// All time arguments are elapsed monotonic time from the caller's fixed epoch.
/// No Iced types, OS timer assumptions, allocations, or I/O in advance().
#[derive(Debug)]
pub struct Playback {
    document: Arc<Document>,
    state: State,
    index: usize,
    generation: u64,
    deadline: Option<Duration>,
    remaining: Duration,
    clock: RationalClock,
    timing: Timing,
    pub diagnostics: Diagnostics,
}
impl Playback {
    pub fn new(document: Arc<Document>, mut timing: Timing) -> Self {
        timing.sanitize();
        let state = if document.is_empty() {
            State::Empty
        } else {
            State::Ready
        };
        Self {
            document,
            state,
            index: 0,
            generation: 0,
            deadline: None,
            remaining: Duration::ZERO,
            clock: RationalClock::default(),
            timing,
            diagnostics: Diagnostics::default(),
        }
    }
    pub fn document(&self) -> &Arc<Document> {
        &self.document
    }
    pub fn state(&self) -> State {
        self.state
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn timing(&self) -> &Timing {
        &self.timing
    }
    /// Remaining throughput estimate, including the unconsumed part of the
    /// current interval; finished/empty documents have no remaining time.
    pub fn estimated_remaining(&self, now: Duration) -> Duration {
        if matches!(self.state, State::Empty | State::Finished | State::Error) {
            return Duration::ZERO;
        }
        let total = self.timing.remaining(&self.document, self.index);
        let current = self
            .document
            .token(self.index)
            .map(|token| self.timing.duration(token))
            .unwrap_or_default();
        let interval = match self.state {
            State::Playing => self
                .deadline
                .map(|deadline| deadline.saturating_sub(now))
                .unwrap_or(current),
            State::Paused if !self.remaining.is_zero() => self.remaining,
            _ => current,
        };
        total.saturating_sub(current).saturating_add(interval)
    }
    pub fn ticket(&self) -> Option<Ticket> {
        self.deadline.map(|deadline| Ticket {
            generation: self.generation,
            deadline,
        })
    }
    fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.deadline = None;
    }
    fn duration(&mut self) -> Duration {
        self.document
            .token(self.index)
            .map(|t| self.clock.duration(&self.timing, t))
            .unwrap_or(Duration::ZERO)
    }
    pub fn play(&mut self, now: Duration) {
        if matches!(self.state, State::Playing | State::Empty | State::Error) {
            return;
        }
        if self.state == State::Finished {
            self.index = 0;
            self.remaining = Duration::ZERO;
        }
        self.invalidate();
        self.clock.reset();
        let duration = if self.remaining.is_zero() {
            self.duration()
        } else {
            self.remaining
        };
        self.deadline = Some(now.saturating_add(duration));
        self.remaining = Duration::ZERO;
        self.state = State::Playing;
    }
    pub fn pause(&mut self, now: Duration) {
        if self.state != State::Playing {
            return;
        }
        // A late pause does not consume unseen words.
        self.remaining = self
            .deadline
            .map(|d| d.saturating_sub(now))
            .unwrap_or_default()
            .max(Duration::from_nanos(1));
        self.invalidate();
        self.state = State::Paused;
    }
    pub fn toggle(&mut self, now: Duration) {
        if self.state == State::Playing {
            self.pause(now);
        } else {
            self.play(now);
        }
    }
    pub fn seek(&mut self, index: usize, now: Duration) {
        let playing = self.state == State::Playing;
        self.invalidate();
        self.index = index.min(self.document.len().saturating_sub(1));
        self.remaining = Duration::ZERO;
        self.clock.reset();
        self.state = if self.document.is_empty() {
            State::Empty
        } else {
            State::Paused
        };
        if playing {
            self.play(now);
        }
    }
    pub fn jump(&mut self, words: i64, now: Duration) {
        let target = if words < 0 {
            self.index
                .saturating_sub(words.unsigned_abs().min(usize::MAX as u64) as usize)
        } else {
            self.index.saturating_add(words as usize)
        };
        self.seek(target, now);
    }
    pub fn restart(&mut self, now: Duration) {
        self.seek(0, now);
    }
    pub fn replace(&mut self, document: Arc<Document>) {
        self.invalidate();
        self.document = document;
        self.index = 0;
        self.remaining = Duration::ZERO;
        self.clock.reset();
        self.diagnostics = Diagnostics::default();
        self.state = if self.document.is_empty() {
            State::Empty
        } else {
            State::Ready
        };
    }
    pub fn fail(&mut self) {
        self.invalidate();
        self.state = State::Error;
    }
    pub fn shutdown(&mut self, now: Duration) {
        self.pause(now);
        self.invalidate();
    }
    pub fn set_timing(&mut self, mut timing: Timing, now: Duration) {
        timing.sanitize();
        let old = self
            .document
            .token(self.index)
            .map(|t| self.timing.duration(t))
            .unwrap_or_default();
        let left = if self.state == State::Playing {
            self.deadline
                .map(|d| d.saturating_sub(now))
                .unwrap_or_default()
        } else {
            self.remaining
        };
        self.timing = timing;
        self.clock.reset();
        let new = self
            .document
            .token(self.index)
            .map(|t| self.timing.duration(t))
            .unwrap_or_default();
        let scaled = if old.is_zero() {
            new
        } else {
            Duration::from_nanos(
                (new.as_nanos().saturating_mul(left.as_nanos()) / old.as_nanos())
                    .min(u128::from(u64::MAX)) as u64,
            )
        };
        if self.state == State::Playing {
            self.invalidate();
            self.deadline = Some(now.saturating_add(scaled.max(Duration::from_nanos(1))));
        } else if !self.remaining.is_zero() {
            self.remaining = scaled.max(Duration::from_nanos(1));
        }
    }
    /// Returns true only when a valid due event changes the displayed word/state.
    pub fn advance(&mut self, ticket: Ticket, now: Duration) -> bool {
        if self.state != State::Playing || self.ticket() != Some(ticket) || now < ticket.deadline {
            return false;
        }
        let lateness = now.saturating_sub(ticket.deadline);
        let ns = lateness.as_nanos().min(u128::from(u64::MAX)) as u64;
        self.diagnostics.last_lateness_ns = ns;
        self.diagnostics.max_lateness_ns = self.diagnostics.max_lateness_ns.max(ns);
        if lateness > Duration::from_millis(2) {
            self.diagnostics.late_events = self.diagnostics.late_events.saturating_add(1);
        }
        self.diagnostics.updates = self.diagnostics.updates.saturating_add(1);
        let mut deadline = ticket.deadline;
        let mut advanced = 0usize;
        loop {
            if self.index.saturating_add(1) >= self.document.len() {
                self.invalidate();
                self.state = State::Finished;
                break;
            }
            self.index += 1;
            advanced += 1;
            let duration = self.duration();
            if self.timing.overrun == Overrun::Preserve {
                // Keep absolute deadlines through ordinary OS wake jitter.
                // Rebase only when the next word's entire slot has expired;
                // otherwise small positive wake errors would accumulate drift.
                let origin = if lateness >= duration {
                    self.diagnostics.schedule_extensions =
                        self.diagnostics.schedule_extensions.saturating_add(1);
                    now
                } else {
                    deadline
                };
                self.deadline = Some(origin.saturating_add(duration));
                break;
            }
            deadline = deadline.saturating_add(duration);
            if deadline > now {
                self.deadline = Some(deadline);
                break;
            }
            if advanced >= 4096 {
                self.diagnostics.schedule_extensions =
                    self.diagnostics.schedule_extensions.saturating_add(1);
                self.deadline = Some(now.saturating_add(duration));
                break;
            }
        }
        self.diagnostics.skipped_words = self
            .diagnostics
            .skipped_words
            .saturating_add(advanced.saturating_sub(1) as u64);
        true
    }
}
