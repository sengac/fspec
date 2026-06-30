# RPC-391 AST Research — Colored Edit/Write Diff Rendering

## Goal
Wire the RPC-390 `diff_format` module into the live agent view and decode the
`[R]-`/`[A]+` markers into colored ratatui spans. Identify the exact touch
points and the existing `ChunkKind::ToolCall` shape.

## ChunkKind::ToolCall shape (the flag we extend)
AstGrep `pattern = "matches!(source.kind, ChunkKind::ToolCall { .. })"`:
- `chunk_wrap.rs:70` — the wrap layer branches into `wrap_tool_call` for tool
  cards. This is where the marker-decode + 8-line-collapse-bypass must hook.

Definition (`views/agent/rendered_chunk.rs:30`):
```rust
ToolCall { tool_call_id: String, is_error: bool }
```
RPC-391 adds `is_diff: bool` → `ToolCall { tool_call_id, is_error, is_diff }`.
Every constructor/match site must be updated:
- `chunk_processor.rs:118` (construct), `:134`, `:151`, `:176` (match)
- `chunk_wrap.rs:47`, `:52`, `:70` (match)
- `turn_modal.rs:168-169` (match)

## chunk_processor touch points
- `handle_tool_call` (`chunk_processor.rs:108`) — capture Edit/Write input here.
- `handle_tool_result` (`chunk_processor.rs:128`) — produce the diff body here,
  currently appends raw `info.content` (`:140-146`).

## SessionContext (where pending diffs live)
`session_context.rs:29` — `struct SessionContext`. Add
`pending_tool_diffs: HashMap<String, PendingToolDiff>` + reset in
`reset_scrollback` (`:158`). Watch the 300-LoC ceiling — extract a
`pending_tool_diff.rs` module for the struct + capture/produce helpers, and a
`diff_decode.rs` module for the marker→span decode used by both `chunk_wrap`
and the modal.

## TurnContentModal
`views/agent/turn_modal.rs` renders `ChunkSource::text` as plain wrapped rows
(`wrap_all` `:148`, `render` `:96`). It does NOT decode markers today — so a
diff card's full text would show literal `[R]`/`[A]`. RPC-391 must decode there
too (build colored `DialogRow` spans). `render_turn_modal` (`:181`) pulls
`full_text_for_seq` — we must retain the FULL (uncollapsed) diff so the modal
shows everything; store it parallel to the collapsed body.

## tool_args parity (tool-name classification)
`tool_args.rs:34` already classifies the Edit/Write family
(`edit|replace|write|write_file`) — reuse the same lowercase match for the
capture branch in `handle_tool_call`.

## RPC-390 consumed surface
`store::agent_view::diff_format::{format_edit_diff, format_write_diff,
format_diff_for_display, calculate_start_line, DIFF_COLLAPSED_LINES,
DiffOutputLine, DiffOutputKind}`.

## Decision
- `is_diff: bool` flag on `ChunkKind::ToolCall` (no string sniffing).
- New module `pending_tool_diff.rs` (PendingToolDiff struct + capture + produce).
- New module `diff_decode.rs` (decode_diff_line → styled spans; shared by
  chunk_wrap + turn_modal). Keeps both touched files < 300 LoC.
- Colors: `DIFF_BG_REMOVED = Color::Rgb(139,0,0)`, `DIFF_BG_ADDED =
  Color::Rgb(0,100,0)`, fg White; context gutter `Color::Gray`.
