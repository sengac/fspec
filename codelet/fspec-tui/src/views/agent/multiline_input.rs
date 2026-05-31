//! MultiLineInput — tui-textarea-backed multi-line input widget for
//! AgentView (RPC-019).
//!
//! Feature: spec/features/rpc019-multiline-input.feature
//!
//! Wraps `tui_textarea::TextArea` with the AgentView-specific
//! contract:
//!
//!   - Plain Enter submits the buffer (Surface a [`InputEventOutcome::Submitted`]).
//!   - Shift+Enter inserts a literal newline (Continued).
//!   - Shift+Up/Down/Left/Right are returned as [`InputEventOutcome::Ignored`]
//!     so the AgentView can map them onto history / session navigation
//!     Actions.
//!   - Pasted text containing embedded newlines is inserted verbatim
//!     (no `\n` → space substitution).
//!   - The widget auto-grows from 1 visible row up to a configurable
//!     `max_visible_rows` cap (default 6).
//!
//! Mirrors the consumer surface of `src/tui/components/MultiLineInput.tsx`
//! but stays small — history persistence + slash-command palette
//! integration is deferred to RPC-021 / RPC-020.

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
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
    /// Cursor lands at the end of the last line.
    pub fn set_value(&mut self, text: &str) {
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.split('\n').map(std::string::ToString::to_string).collect()
        };
        let mut ta = TextArea::new(lines);
        ta.set_cursor_line_style(ratatui::style::Style::default());
        ta.move_cursor(tui_textarea::CursorMove::End);
        self.textarea = ta;
    }

    /// Number of logical lines (including the trailing empty line if
    /// the buffer ends with '\n').
    pub fn line_count(&self) -> usize {
        self.textarea.lines().len()
    }

    /// Current visible-row count = `line_count` clamped to
    /// `[1, max_visible_rows]`.
    pub fn visible_rows(&self) -> u16 {
        let n = self.line_count() as u16;
        n.clamp(1, self.max_visible_rows)
    }

    /// Reset to an empty buffer.
    pub fn reset(&mut self) {
        let mut ta = TextArea::default();
        ta.set_cursor_line_style(ratatui::style::Style::default());
        self.textarea = ta;
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

    /// RPC-095 — Gated variant of `handle_key`. The `gate` is
    /// computed by the AgentView orchestrator per-frame.
    pub fn handle_key_gated(
        &mut self,
        code: KeyCode,
        mods: KeyModifiers,
        gate: InputGate,
    ) -> InputEventOutcome {
        // Plain Enter → submit, unless suppressed.
        if code == KeyCode::Enter && mods.is_empty() {
            if gate.suppress_enter {
                return InputEventOutcome::Continued;
            }
            let buf = self.value();
            self.reset();
            return InputEventOutcome::Submitted(buf);
        }
        // Shift+Enter → literal newline (Continued). Treated as an
        // edit, so block_edits swallows it too.
        if code == KeyCode::Enter && mods.contains(KeyModifiers::SHIFT) {
            if gate.block_edits {
                return InputEventOutcome::Continued;
            }
            self.textarea.insert_newline();
            return InputEventOutcome::Continued;
        }
        // Shift+arrow chords → forwarded to caller (history / session
        // navigation). Reported as Ignored so AgentView can convert
        // them into Action::{HistoryPrev,HistoryNext,SessionPrev,SessionNext}.
        if mods.contains(KeyModifiers::SHIFT)
            && matches!(code, KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right)
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
        if mods.contains(KeyModifiers::CONTROL) && matches!(code, KeyCode::Char('d') | KeyCode::Char('D')) {
            return InputEventOutcome::Ignored;
        }
        // RPC-095 — block edits gate. Swallow Backspace, Delete,
        // forward-delete, and printable character insertion.
        if gate.block_edits && is_edit_keystroke(code, mods) {
            return InputEventOutcome::Continued;
        }
        // Everything else → forward to the textarea.
        let input = Input::from(crossterm::event::KeyEvent::new(code, mods));
        let _ = self.textarea.input(input);
        InputEventOutcome::Continued
    }

    /// Handle an arbitrary crossterm Event (key OR bracketed paste).
    pub fn handle_event(&mut self, event: &Event) -> InputEventOutcome {
        match event {
            Event::Key(key) => self.handle_key(key.code, key.modifiers),
            Event::Paste(s) => {
                // Bracketed paste — insert verbatim, preserving '\n'.
                // tui-textarea's insert_str DOES preserve embedded
                // newlines; verified via its `insert_str` test.
                let _ = self.textarea.insert_str(s);
                InputEventOutcome::Continued
            }
            _ => InputEventOutcome::Ignored,
        }
    }

    /// Render the textarea content into `area`. The caller is
    /// responsible for the surrounding border + placeholder hint.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // ratatui 0.29 / tui-textarea 0.7: the textarea itself
        // implements Widget for `&TextArea`.
        (&self.textarea).render(area, buf);
    }

    /// Paint the input box body: green "> " prompt prefix on the top
    /// row, then either the textarea content or a dim placeholder
    /// hint when the buffer is empty. Used by AgentView so the
    /// orchestrator stays under its 300-LoC ceiling.
    pub fn render_with_prompt(&self, area: Rect, buf: &mut Buffer, placeholder: &str) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let prompt_area = Rect {
            x: area.x,
            y: area.y,
            width: 2.min(area.width),
            height: 1,
        };
        Paragraph::new(Line::from(Span::styled(
            "> ",
            Style::default().fg(Color::Green),
        )))
        .render(prompt_area, buf);

        let body_x = area.x.saturating_add(2);
        let body_width = area.width.saturating_sub(2);
        if body_width == 0 {
            return;
        }
        let body_area = Rect {
            x: body_x,
            y: area.y,
            width: body_width,
            height: area.height,
        };
        if self.is_empty() {
            let hint = Span::styled(placeholder, Style::default().fg(Color::DarkGray));
            Paragraph::new(Line::from(hint)).render(body_area, buf);
        } else {
            self.render(body_area, buf);
        }
    }
}

/// RPC-095 — returns true if the keystroke would otherwise insert
/// text or delete characters. Used by `handle_key_gated` to swallow
/// input while the session is Compacting (`gate.block_edits = true`).
fn is_edit_keystroke(code: KeyCode, _mods: KeyModifiers) -> bool {
    matches!(
        code,
        KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete | KeyCode::Tab
    )
}
