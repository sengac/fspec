//! **TUI-115** — the per-session record of inputs submitted while the
//! session is busy (Running or Compacting).
//!
//! Feature: spec/features/running-composer-steer-and-queue-hints.feature
//!
//! A steer is echoed by the session at once (`StreamChunk::UserInput`);
//! that echo is tagged `You (steer): `. A queued input is shown at submit
//! time as `Queued #n: ` and listed until its turn starts; the session's
//! start echo (`resume_queued` → `send_input`, oldest first) then drops it
//! from the list and is not shown a second time.

use std::collections::VecDeque;

use codelet_rpc_types::StreamChunk;
use ratatui::style::Color;

use super::SessionContext;
use crate::store::agent_view::chunk_processor::flush_in_flight_drop_empty;
use crate::terminal::sanitize::sanitize_for_terminal;
use crate::views::agent::{ChunkKind, ChunkSource, UserInputTag};

/// One input queued behind a busy turn, numbered in submission order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedInput {
    pub n: u32,
    pub text: String,
}

/// Steers awaiting their echo and queued inputs awaiting their turn.
#[derive(Debug, Default)]
pub struct BusyInputs {
    steers: VecDeque<String>,
    queued: Vec<QueuedInput>,
}

impl BusyInputs {
    /// Queued inputs that have not started yet, oldest first.
    pub fn queued(&self) -> &[QueuedInput] {
        &self.queued
    }

    /// The chunk kind for the session's echo of `text`, or `None` when
    /// the echo starts a queued input already shown as `Queued #n`.
    pub(crate) fn echo_kind(&mut self, text: &str) -> Option<ChunkKind> {
        if self.steers.front().is_some_and(|steer| steer == text) {
            self.steers.pop_front();
            return Some(ChunkKind::TaggedUserInput(UserInputTag::Steer));
        }
        if self
            .queued
            .first()
            .is_some_and(|queued| queued.text == text)
        {
            self.queued.remove(0);
            return None;
        }
        Some(ChunkKind::UserInput)
    }

    /// The number the next queued input gets: one past the highest listed
    /// (a resync can list an entry ahead of older-numbered ones), 1 when
    /// nothing is listed.
    fn next_queued_n(&self) -> u32 {
        match self.queued.iter().map(|queued| queued.n).max() {
            Some(highest) => highest + 1,
            None => 1,
        }
    }

    /// **TUI-119**: replace the queued list with the session's
    /// authoritative `queued_inputs` (oldest first) from an interrupt. A
    /// listed entry matching the next text in order keeps its number;
    /// listed entries the session no longer holds are dropped; a text
    /// never listed gets the next number after the highest listed one.
    pub(crate) fn resync_queued(&mut self, queued_inputs: &[String]) {
        let mut next_n = self.next_queued_n();
        let listed = std::mem::take(&mut self.queued);
        let mut rest = listed.as_slice();
        for text in queued_inputs {
            match rest.iter().position(|entry| &entry.text == text) {
                Some(offset) => {
                    self.queued.push(rest[offset].clone());
                    rest = &rest[offset + 1..];
                }
                None => {
                    self.queued.push(QueuedInput {
                        n: next_n,
                        text: text.clone(),
                    });
                    next_n += 1;
                }
            }
        }
    }
}

impl SessionContext {
    /// **TUI-119**: a /clear resets the scrollback and forgets every steer
    /// and queued input, so `Queued #n` numbering restarts at 1.
    pub fn reset_for_clear(&mut self) {
        self.reset_scrollback();
        self.busy_inputs = BusyInputs::default();
    }

    /// **TUI-119**: a LIVE `Interrupted` / `InterruptedWithReason` carries
    /// the session's authoritative queue; resync the queued list from it.
    /// Transcript replay (`record_chunk`) never resyncs: journal-replayed
    /// interrupts carry no queue and may be historic.
    pub fn apply_live_interrupt_queue(&mut self, chunk: &StreamChunk) {
        if let StreamChunk::Interrupted { queued_inputs }
        | StreamChunk::InterruptedWithReason { queued_inputs, .. } = chunk
        {
            self.busy_inputs.resync_queued(queued_inputs);
        }
    }

    /// Remember a steer so its echo is tagged `You (steer): `.
    pub fn record_steer(&mut self, text: String) {
        self.busy_inputs.steers.push_back(text);
    }

    /// Show `text` as the next `Queued #n: ` line and list it until its
    /// turn starts. Returns `n`.
    pub fn record_queued(&mut self, text: String) -> u32 {
        let n = self.busy_inputs.next_queued_n();
        flush_in_flight_drop_empty(self);
        self.push_chunk(ChunkSource {
            text: sanitize_for_terminal(&text),
            color: Color::Green,
            kind: ChunkKind::TaggedUserInput(UserInputTag::Queued(n)),
            is_streaming: false,
            full_text: None,
        });
        self.busy_inputs.queued.push(QueuedInput { n, text });
        n
    }
}
