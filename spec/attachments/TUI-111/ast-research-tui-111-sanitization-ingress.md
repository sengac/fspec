# AST Research — TUI-111: Centralized Terminal Output Sanitization at Ingress

Generated during discovery via `GraphSearch` (ast_search / ast_callers) + targeted
Reads. This is the AST analysis record required before entering the testing phase.

## 1. Canonical sanitizer (the entity being moved)

```
slug: fspec-tui-src-store-agent_view-sanitize-rs::sanitize_for_terminal
path: rust/fspec-tui/src/store/agent_view/sanitize.rs:34-51
signature: pub fn sanitize_for_terminal(text: &str) -> String
cyclomatic complexity: 8, params: (text)
deps: regex::Regex (static ANSI_ESCAPE_RE), is_control_char (same file)
```

AST caller graph (14 direct + indirect callers):
- **Production call sites (3 files, 5 physical calls):**
  - `store/agent_view/chunk_tool_result.rs` — 2 calls (ToolResult content, ToolProgress chunk)
  - `views/diff_common/diff_render.rs` — `diff_line()` (diff bodies)
  - `views/diff_common/row.rs` — `file_row()` (path + change_type, 2 calls)
  - `views/checkpoints/render.rs` — `checkpoint_line()` label
- **Test call sites (sanitize_tests.rs / sanitize_tests_streaming.rs, 9 test fns)** — these
  move with the module.

## 2. Ingress choke points (functions that receive external strings)

### 2.1 `SessionContext::record_chunk` — THE scrollback ingress
```
path: rust/fspec-tui/src/store/agent_view/session_context.rs:81-160
match arms with visible text payloads (target of the refactor):
  StreamChunk::Text { text }              → append_assistant_text(self, text)
  StreamChunk::UserInput { text }         → push_chunk (green UserInput chunk)
  StreamChunk::Thinking { thinking }      → append_thinking(self, thinking)
  StreamChunk::ToolCall { tool_call }     → handle_tool_call (name + extract_tool_args_display)
  StreamChunk::ToolResult { tool_result } → handle_tool_result (info.content, today sanitized inside)
  StreamChunk::ToolProgress { tool_progress } → handle_tool_progress (info.output_chunk, today sanitized inside)
  StreamChunk::Error { error }            → handle_error → "API Error: {error}"
  StreamChunk::UserNotification { message } → push_chunk (Notification)
  StreamChunk::IncomingMessage { text }   → parse_supervisor_envelope → "[W] role> body"
  StreamChunk::Interrupted                → static "⚠ Interrupted" (no external text)
state-only variants (no visible text — untouched):
  SessionStateChange, IsolationStateChange, DebugStateChange, FooterStateUpdate,
  FspecCommandRequest, FspecCommandResult, WorkUnitsUpdate, SupervisorPendingInjection,
  CompactionComplete, TokenUpdate, ContinueStateUpdate, ContextFillUpdate,
  ExecStdinRequest, ExecStdinRequestCleared
```
All downstream sinks (scrollback paint, rewrap on resize, turn modal via
`full_text_for_seq`, COPY selection/copy, mux agent panes) re-read
`ChunkSource::text` — so sanitizing here covers them structurally.

### 2.2 `BoardStore::replace_work_units`
```
slug: fspec-tui-src-store-board-rs::replace_work_units
path: rust/fspec-tui/src/store/board.rs:69-125, cyclomatic 13
input: Vec<WorkUnitInfo> (fs/backend-originated)
external fields: id, title, description, epic, attachments[] (work_type/status closed vocab)
downstream sinks: views/board/viewport.rs build_cell_label (card),
  views/board/details_strip.rs render + visible_strip_rows (COPY-009),
  components/work_unit_search_rows.rs, views/multiplex board pane
```

### 2.3 Prompt/chrome store setters (AgentViewStore)
```
set_hitl_prompt     store/agent_view/hitl_state.rs:142-145
                    → HitlPromptState::new(request); sanitize HitlRequest.questions[].{header,question,options[].{label,description}}
set_pause_state     store/agent_view/pause_state.rs:40
                    → PauseState.{prompt, tool_call_id}
set_exec_stdin      store/agent_view/exec_stdin_state.rs:40
                    → ExecStdinRequest.command
set_role            store/agent_view/role_state.rs:30
                    → role string (RoleBanner)
```

### 2.4 Notice handler
```
App::handle_emit_session_notice  app/dispatch_slash_clear.rs:30
  Action::EmitSessionNotice(sid, text) → ctx.push_line(text)  (Notification chunk)
```

### 2.5 Mode-view state setters
```
ChangedFilesView::set_files / set_diff_lines   views/changed_files/mod.rs:149,171
CheckpointsView::set_checkpoints / set_files / set_diff_lines   views/checkpoints/mod.rs:184,212,230
ResumeSessionView::set_sessions  views/agent/resume_session_view.rs:120
SearchHistoryView rows (HistoryMatch.text)     views/agent/search_history_view.rs
BlocklistView::set_rules   views/blocklist/mod.rs:97  (BlocklistRuleInfo.{id,pattern,reason,guidance,source})
ModelSelectorView::set_providers  views/model_selector/state.rs:47  (display_name fields)
```

### 2.6 Dialog constructors
```
ErrorDialog::new            components/error_dialog.rs:37        (message)
NotificationDialog::new     components/notification_dialog.rs:102 (message)
StatusDialog::new           components/status_dialog.rs          (message)
LoadingDialog::new          components/loading_dialog.rs:48      (title + label)
CheckpointRestoreDialog     components/checkpoint_restore_dialog.rs (checkpoint names)
```
The LoadingDialog labels are the cascade stage labels built in
`app/dispatch_changed_files.rs` / `app/dispatch_checkpoints.rs` /
`app/dispatch_checkpoint_diff.rs` via `LoadTracker::begin_stage(key, label)`
(load_state.rs:76-81 documents "views sanitize before passing" — today nothing does).

## 3. Diff path (separate lane — codec sentinel constraint)

```
pending_tool_diff::capture_pending_diff   store/agent_view/pending_tool_diff.rs:49
pending_tool_diff::produce_diff_strings   store/agent_view/pending_tool_diff.rs:85
  → diff_format::build_edit_diff_rows_with_context (diff_context.rs; reads post-edit file from disk)
  → diff_format::format_write_diff / format_diff_for_display
  → diff_codec::to_line  (introduces ELISION_SENTINEL = '\u{1}', a C0 control char,
                          diff_codec.rs:27 — MUST NOT be stripped post-encode)
  → chunk_tool_result::handle_tool_result splices collapsed/full into ChunkSource.text/full_text
Downstream: chunk_wrap::wrap_tool_call (diff branch parses via parse_line each wrap),
  turn_modal styled_rows (parse_line), copy.
Sanitization point for the refactor: sanitize each SOURCE line (tool input strings +
context file lines) BEFORE to_line encoding — never run to_line output through the sanitizer.
```

## 4. Stderr marker ordering (must preserve)

```
store/agent_view/stderr.rs:
  STDERR_MARKER = "⚠stderr⚠"
  maybe_mark(chunk, is_stderr) — applied AFTER sanitization today
  strip_marker(line) — applied at render (chunk_wrap.rs:182-183, stderr.rs::style_modal_raw_line)
Refactor keeps: sanitize FIRST → maybe_mark → store; strip_marker at render unchanged.
```

## 5. Existing test inventory (regression net)

- `store/agent_view/sanitize_tests.rs` + `sanitize_tests_streaming.rs` — 14 scenario tests
  with `@step` comments (TUI-100). MOVE unchanged.
- `fspec-tui/tests/chunkprocessor_parity_rpc091.rs`, `chunk_rendering_parity_rpc078.rs`,
  `chunk_rendering_parity_rpc071.rs`, `thinking_streaming_parity_rpc093.rs` —
  golden-string chunk accumulation (pin append_assistant_text/append_thinking/handle_* behavior).
- `fspec-tui/tests/scrollback_pty_rpc078.rs`, `scrollback_wrap_rpc078.rs`,
  `scrollback_scroll_rpc094.rs` — scrollback rendering.
- `fspec-tui/tests/app_dispatch_reorder_rpc017.rs` — pins replace_work_units re-anchoring
  (must keep passing after ingress sanitization).
- `fspec-tui/tests/view_board_unit_rpc012/013/014/015/016.rs` — board golden strings.
- `fspec-tui/tests/checkpoint_row_* / view_* / blocklist_view_rpc056.rs` — mode-view goldens.
- `fspec-tui/tests/common/harness.rs` — mock-backend App harness for new integration tests.

## 6. Width-math ordering audit (sanitize BEFORE truncate)

- `views/board/details_strip.rs` — `truncate_to`/`wrap_to_two_lines` on raw text today;
  after ingress sanitization the stored text is clean, so all downstream width math is
  automatically correct. No view-side changes needed.
- `views/checkpoints/render.rs:194-197` — "Sanitize before truncating" comment becomes
  redundant (already sanitized at ingress) — call site deleted.
- `views/diff_common/diff_render.rs` / `row.rs` — call sites deleted after ingress.

## 7. Module move mechanics

```
rust/fspec-tui/src/terminal.rs          → terminal/mod.rs (verbatim)
rust/fspec-tui/src/terminal/sanitize.rs (new, from store/agent_view/sanitize.rs)
rust/fspec-tui/src/terminal/sanitize_tests.rs          (moved; #[cfg(test)] in terminal/mod.rs)
rust/fspec-tui/src/terminal/sanitize_tests_streaming.rs (moved; #[cfg(test)] in terminal/mod.rs)
store/agent_view.rs (the agent_view module root): remove
  `pub mod sanitize;`, `#[cfg(test)] pub mod sanitize_tests;`,
  `#[cfg(test)] pub mod sanitize_tests_streaming;`
terminal/mod.rs gains:
  `pub mod sanitize;` + `#[cfg(test)] pub mod sanitize_tests;` +
  `#[cfg(test)] pub mod sanitize_tests_streaming;`
  (the test files' inner `use crate::store::agent_view::sanitize::sanitize_for_terminal;`
   imports migrate to `use crate::terminal::sanitize::sanitize_for_terminal;`)
lib.rs: `pub mod terminal;` unchanged; re-export becomes
       `pub use terminal::sanitize::sanitize_for_terminal;`
       (replaces `pub use store::agent_view::sanitize::sanitize_for_terminal;`)
```
