//! MultiLineInput — tui-textarea-backed multi-line input widget for
//! AgentView (RPC-019).
//!
//! Feature: spec/features/rpc019-multiline-input.feature
//!
//! Wraps `tui_textarea::TextArea` with the AgentView-specific
//! contract:
//!
//!   - Plain Enter submits the buffer (Surface a [`InputEventOutcome::Submitted`]).
//!   - Any modifier-carrying Enter (Shift/Alt/Ctrl/…) inserts a literal
//!     newline (Continued) — RPC-402; routing lives in
//!     `multiline_input_enter.rs`.
//!   - Shift+Up/Down/Left/Right are returned as [`InputEventOutcome::Ignored`]
//!     so the AgentView can map them onto history / session navigation
//!     Actions.
//!   - Pasted text containing embedded newlines is inserted verbatim.
//!   - The widget auto-grows from 1 visible row up to a configurable
//!     `max_visible_rows` cap (default 6).
//!
//! Mirrors the consumer surface of `src/tui/components/MultiLineInput.tsx`
//! but stays small — history persistence + slash-command palette
//! integration lives in RPC-021 / RPC-020; Enter routing in
//! `multiline_input_enter.rs` (RPC-402).

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use tui_textarea::{Input, TextArea};

/// Outcome of routing one input event through the [`MultiLineInput`].
///
/// `Submitted(buffer)` fires only on plain Enter and is consumed by
/// the AgentView (which translates it into `Action::InputSubmitted`).
/// `Continued` means the textarea handled the event internally
/// (insert, backspace, cursor move). `Ignored` is reserved for chord
/// keys (Shift+arrows) that the AgentView wants to translate into
/// view-level navigation Actions.
#[derive(Debug, Clone)]
pub enum InputEventOutcome {
    Submitted(String),
    Continued,
    Ignored,
}

/// RPC-095: per-keystroke gate. See parent doc.
#[derive(Debug, Clone, Copy, Default)]
pub struct InputGate {
    pub block_edits: bool,
    pub suppress_enter: bool,
}

/// Multi-line input wrapper. Owns a `TextArea<'static>` and a fixed
/// visible-row cap. The widget itself renders into a 1+N-row region —
/// the AgentView is responsible for the surrounding border + the
/// optional placeholder hint when the buffer is empty.
pub struct MultiLineInput {
    textarea: TextArea<'static>,
    max_visible_rows: u16,
    /// RPC-405 — first visible VISUAL row of the wrap-aware viewport.
    /// Mutated by `sync_viewport` (cursor-follow) before each paint.
    scroll_top: usize,
}

impl Default for MultiLineInput {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiLineInput {
    /// Construct an empty input with the default 6-row visible cap.
    pub fn new() -> Self {
        Self::with_max_visible_rows(6)
    }

    /// Construct an empty input with a custom visible-row cap. The
    /// underlying textarea retains all logical lines — only the
    /// rendered viewport is capped.
    pub fn with_max_visible_rows(max: u16) -> Self {
        let mut textarea = TextArea::default();
        // RPC-019: hide the textarea's own visual cursor highlight on
        // the cursor LINE — we paint the cursor cell ourselves via
        // ratatui's `set_cursor_position` (cursor_position() helper on
        // AgentView).
        textarea.set_cursor_line_style(ratatui::style::Style::default());
        Self {
            textarea,
            max_visible_rows: max.max(1),
            scroll_top: 0,
        }
    }

    /// True when the underlying buffer has no characters.
    pub fn is_empty(&self) -> bool {
        self.textarea.is_empty()
    }

    /// Current buffer content, lines joined with '\n'.
    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Replace the buffer with `text` (newlines split into lines).
    /// Cursor lands at the end of the LAST line (RPC-405: TS parity —
    /// `setValue` scrolls to the end; `CursorMove::End` alone would
    /// park it at the end of line 0).
    pub fn set_value(&mut self, text: &str) {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n')
                .map(std::string::ToString::to_string)
                .collect()
        };
        let mut ta = TextArea::new(lines);
        ta.set_cursor_line_style(ratatui::style::Style::default());
        ta.move_cursor(tui_textarea::CursorMove::Bottom);
        ta.move_cursor(tui_textarea::CursorMove::End);
        self.textarea = ta;
    }

    /// Number of logical lines (including the trailing empty line if
    /// the buffer ends with '\n').
    pub fn line_count(&self) -> usize {
        self.textarea.lines().len()
    }

    /// LOGICAL-line count clamped to `[1, max_visible_rows]` — NOT
    /// wrap-aware. Retained for tests/back-compat; the AgentView
    /// layout uses the wrap-aware
    /// [`visible_rows_for_width`](Self::visible_rows_for_width)
    /// instead.
    pub fn visible_rows(&self) -> u16 {
        let n = self.line_count() as u16;
        n.clamp(1, self.max_visible_rows)
    }

    /// Reset to an empty buffer.
    pub fn reset(&mut self) {
        let mut ta = TextArea::default();
        ta.set_cursor_line_style(ratatui::style::Style::default());
        self.textarea = ta;
        self.scroll_top = 0;
    }

    /// The logical buffer lines (RPC-405: consumed by the wrap-aware
    /// renderer + geometry in `multiline_input_render.rs`).
    pub fn lines(&self) -> &[String] {
        self.textarea.lines()
    }

    /// RPC-405 — first visible visual row of the wrap viewport
    /// (exposed for RPC-404 hardware-cursor positioning).
    pub fn scroll_top(&self) -> usize {
        self.scroll_top
    }

    /// RPC-405 — viewport mutation hook for `sync_viewport` (lives in
    /// `multiline_input_render.rs` to keep this file under 300 LoC).
    pub(super) fn set_scroll_top(&mut self, top: usize) {
        self.scroll_top = top;
    }

    /// Visual-row cap (RPC-405: consumed by `visible_rows_for_width`).
    pub fn max_visible_rows(&self) -> u16 {
        self.max_visible_rows
    }

    /// 0-based (row, col) cursor position inside the buffer. Useful
    /// for the AgentView to compute the terminal-relative cursor
    /// location.
    pub fn cursor(&self) -> (usize, usize) {
        self.textarea.cursor()
    }

    /// Internal key-event router used by `handle_event` and exposed
    /// for unit tests. Returns the outcome the caller (AgentView)
    /// should translate into an EventResult + Action.
    pub fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) -> InputEventOutcome {
        self.handle_key_gated(code, mods, InputGate::default())
    }

    /// RPC-095 — Move cursor to column 0 of the current line. Used by
    /// the input-clear path of the Esc cascade so callers can position
    /// the cursor without sending a key event.
    pub fn move_cursor_home(&mut self) {
        self.textarea.move_cursor(tui_textarea::CursorMove::Head);
    }

    /// RPC-402 — newline at cursor; used by the Enter-key router.
    pub(super) fn insert_newline(&mut self) {
        self.textarea.insert_newline();
    }

    /// RPC-403 — insert `text` at the cursor (embedded `\n` preserved);
    /// used by the paste router in `multiline_input_paste.rs`.
    pub(super) fn insert_str(&mut self, text: &str) {
        let _ = self.textarea.insert_str(text);
    }

    /// RPC-095 — Gated variant of `handle_key`. The `gate` is
    /// computed by the AgentView orchestrator per-frame.
    pub fn handle_key_gated(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
        gate: InputGate,
    ) -> InputEventOutcome {
        // RPC-402: Enter dispositions (plain submit, Shift/Alt newline)
        // live in `multiline_input_enter.rs` to keep this file <300 LoC.
        if let Some(outcome) = super::multiline_input_enter::handle_enter(self, code, mods, gate) {
            return outcome;
        }
        // Shift+arrow chords → forwarded to caller (history / session
        // navigation). Reported as Ignored so AgentView can convert
        // them into Action::{HistoryPrev,HistoryNext,SessionPrev,SessionNext}.
        if mods.contains(KeyModifiers::SHIFT)
            && matches!(
                code,
                KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right
            )
        {
            return InputEventOutcome::Ignored;
        }
        // Up/Down on the boundary of the textarea (first/last line) →
        // Ignored so a future RPC can layer scrollback nav on top.
        if matches!(code, KeyCode::Up | KeyCode::Down) {
            let (row, _col) = self.cursor();
            let line_count = self.line_count();
            let at_top = code == KeyCode::Up && row == 0;
            let at_bottom = code == KeyCode::Down && row + 1 >= line_count;
            if at_top || at_bottom {
                return InputEventOutcome::Ignored;
            }
        }
        // RPC-073: Ctrl+D bubbles up so App::handle_app_shortcut
        // (Stage 4) fires the global quit fallback.
        if mods.contains(KeyModifiers::CONTROL)
            && matches!(code, KeyCode::Char('d') | KeyCode::Char('D'))
        {
            return InputEventOutcome::Ignored;
        }
        // RPC-095 — block edits gate. Swallow Backspace, Delete,
        // forward-delete, and printable character insertion
        // (`is_edit_keystroke` lives in `multiline_input_enter.rs`).
        if gate.block_edits && super::multiline_input_enter::is_edit_keystroke(code, mods) {
            return InputEventOutcome::Continued;
        }
        // Everything else → forward to the textarea.
        let input = Input::from(crossterm::event::KeyEvent::new(code, mods));
        let _ = self.textarea.input(input);
        InputEventOutcome::Continued
    }

    /// Handle an arbitrary crossterm Event (key OR bracketed paste).
    /// Ungated variant of [`Self::handle_event_gated`].
    pub fn handle_event(&mut self, event: &Event) -> InputEventOutcome {
        self.handle_event_gated(event, InputGate::default())
    }

    /// RPC-402: only `KeyEventKind::Press` key events are processed —
    /// under kitty enhancement flags the terminal may deliver
    /// `Release`/`Repeat` events which would double-type, so they are
    /// ignored. RPC-403: bracketed paste routes through
    /// `multiline_input_paste.rs` (CRLF→LF normalization + RPC-095
    /// `block_edits` gate).
    pub fn handle_event_gated(&mut self, event: &Event, gate: InputGate) -> InputEventOutcome {
        match event {
            Event::Key(key) => {
                if key.kind != KeyEventKind::Press {
                    return InputEventOutcome::Ignored;
                }
                self.handle_key_gated(key.code, key.modifiers, gate)
            }
            Event::Paste(s) => super::multiline_input_paste::handle_paste(self, s, gate),
            _ => InputEventOutcome::Ignored,
        }
    }
}
