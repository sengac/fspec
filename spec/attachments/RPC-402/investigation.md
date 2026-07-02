# RPC-402 Investigation: Shift+Enter newline unreachable — keyboard enhancement flags never enabled

## Symptom

The Rust TUI agent-view input (`MultiLineInput`) only ever behaves as a single-line input. Pressing
Shift+Enter submits the message instead of inserting a newline, even though the widget has an
explicit Shift+Enter → `insert_newline()` branch and full multi-line rendering support (auto-grow
to 6 rows).

## Reference behavior (TS legacy TUI)

- `src/tui/components/MultiLineInput.tsx` + `src/tui/hooks/useMultiLineInput.ts`
- Buffer is `string[]` of logical lines; `insertNewline` exists at `useMultiLineInput.ts:314-330`.
- NOTE: the TS component *also* never wires Shift+Enter (doc comment at line 10 claims it, but
  `insertNewline` has zero call sites; `key.return` at MultiLineInput.tsx:141-147 always submits).
  So this is a gap in BOTH frontends — but the Rust widget already has the branch written; only
  the terminal protocol plumbing is missing. The desired UX (per the widget's own doc comment at
  `multiline_input.rs:1-21`) is: **Enter submits, Shift+Enter inserts a newline**.

## Rust architecture (current)

Pipeline:

```
crossterm EventStream → app/events.rs run loop → Navigator → AgentView::handle_event (dispatch.rs)
  → MultiLineInput::handle_key_gated (multiline_input.rs)
```

Key files:

| File | Role |
|---|---|
| `codelet/fspec-tui/src/terminal.rs` | `TerminalGuard::init` (:47-53) / `enable_terminal_modes` (:68-78) — raw mode + `EnterAlternateScreen` + `EnableMouseCapture` + `EnableBracketedPaste`. **No `PushKeyboardEnhancementFlags`.** |
| `codelet/fspec-tui/src/views/agent/multiline_input.rs` | 297 LoC. Wraps `tui_textarea::TextArea<'static>` (tui-textarea 0.7, `codelet/Cargo.toml:136`). Buffer = Vec<String> of lines. `handle_key_gated`: Enter+empty-mods submits (:165-173); **Shift+Enter → `insert_newline()` (:174-182)**; Up/Down at buffer boundary → Ignored (:194-204); everything else forwarded to `textarea.input()` (:217-220). `visible_rows()` = `line_count().clamp(1, 6)` (:122-127). |
| `codelet/fspec-tui/src/views/agent/dispatch.rs` | `Submitted(value)` → `Action::InputSubmitted` (:252-258); RPC-095 compacting gate (:243-246); Shift+arrows history/session (:25-33, :223-228); boundary Up/Down → scrollback (:231-239, :269-280). |
| `codelet/fspec-tui/src/app/events.rs` | Run loop; cursor positioned via `frame.set_cursor_position` (:198-204, :241-247) from `AgentView::cursor_position` (`agent.rs:133-143`). |
| `codelet/fspec-tui/src/views/agent.rs` | `input_height = self.input.visible_rows()` (:226); layout gives input band `Length(input_height)` (:233-242). |

## Root cause

Legacy terminal mode encodes **Enter and Shift+Enter as the identical byte `CR` (0x0D)**. Without
the kitty keyboard protocol (crossterm `PushKeyboardEnhancementFlags`), crossterm can never report
`KeyModifiers::SHIFT` on `KeyCode::Enter`. Grep for
`KeyboardEnhancement|PushKeyboard|kitty` across `codelet/fspec-tui/src/` → **zero matches**.

Therefore:
- The Shift+Enter branch at `multiline_input.rs:176` is **unreachable** in a real terminal.
- Every Enter variant (plain, Shift, Ctrl, Ctrl+J — crossterm normalizes LF to Enter) collapses
  into the submit branch at `multiline_input.rs:166`.
- Incidental escape hatch: Alt+Enter on terminals that send `ESC CR` is parsed as Enter+ALT,
  skips both branches, falls through to tui-textarea's default keymap which maps any-modifier
  Enter → `insert_newline()` (vendored `tui-textarea-0.7.0/src/textarea.rs:288-290`). Unintended
  and undocumented.

## Fix design

1. **terminal.rs**: after entering the alternate screen, attempt
   `PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)`
   (optionally `| REPORT_ALL_KEYS_AS_ESCAPE_CODES` is NOT needed; disambiguation suffices for
   modifier reporting on Enter). Must be **best-effort**: query support via
   `crossterm::terminal::supports_keyboard_enhancement()` and only push when supported (kitty,
   foot, WezTerm, recent Windows Terminal, iTerm2 do; many legacy terminals don't). Store a flag
   on `TerminalGuard` so cleanup issues `PopKeyboardEnhancementFlags` **before** leaving the
   alternate screen / disabling raw mode. Cleanup must be panic-safe and mirrored in the Drop
   path (match how existing modes are torn down in `terminal.rs`).
2. **multiline_input.rs**: no change needed for Shift+Enter itself (branch exists). BUT:
   - With enhancement flags pushed, crossterm may deliver `KeyEventKind::Release`/`Repeat`
     events. Verify the dispatch path filters to `KeyEventKind::Press` (check existing handling
     in app/events.rs / dispatch.rs); if not, add the filter or Release events will double-type.
   - Close the accidental Alt+Enter fallthrough OR make it intentional: recommended to treat
     Alt+Enter explicitly as insert_newline too (common convention, works on legacy terminals
     that send ESC CR — gives users on non-kitty terminals a working newline key). This makes
     multi-line input reachable even where enhancement flags are unsupported.
3. **Placeholder/help text**: input placeholder (`INPUT_PLACEHOLDER_HINT`, `agent.rs:74-75`) and/or
   help dialog should mention Shift+Enter (and Alt+Enter) for newline. Check
   `help_dialog.rs::for_agent()`.

## Risks / constraints

- `PushKeyboardEnhancementFlags` on unsupported terminals can garble input — MUST gate on
  `supports_keyboard_enhancement()` (which requires raw mode active; call after raw mode enable).
- Must Pop flags on ALL exit paths (normal quit, panic hook, suspend) — inspect `TerminalGuard`
  Drop and any panic-hook restore in `terminal.rs`.
- `KeyEventKind::Release` events: with DISAMBIGUATE_ESCAPE_CODES alone, release events are not
  reported, but be defensive — filter Press anyway if not already done.
- Files at 300-LoC edge: `multiline_input.rs` is 297 LoC — adding logic may require splitting.
- Testing: terminal-mode side effects can't be asserted in unit tests directly; test the
  key-handling logic (Shift+Enter and Alt+Enter produce newline, plain Enter submits, buffer
  grows visible_rows) via `MultiLineInput` unit-level tests, and test `TerminalGuard` flag
  bookkeeping via extracted pure functions where possible.

## Relationship to RPC-403

RPC-403 (paste routing) is the second half of "why the Rust input is single-line". They are
independent fixes: this card makes typed newlines possible; RPC-403 makes pasted newlines
possible. `relates-to` dependency recorded.
