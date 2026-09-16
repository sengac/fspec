# Research: Centralized Terminal Output Sanitization for ALL TUI Views

**Work unit**: TUI-111 (to be created)
**Date**: 2026-09-13
**Researcher**: fspec agent (research-only — no production code changed)

---

## 1. Problem Statement

Today `sanitize_for_terminal()` (TUI-100, card `TUI-100`) is applied **only to two
stream-chunk variants** of the AgentView scrollback:

- `StreamChunk::ToolResult` → `info.content` (`store/agent_view/chunk_tool_result.rs:94`)
- `StreamChunk::ToolProgress` → `info.output_chunk` (`chunk_tool_result.rs:146`)

Plus two **view-local, ad-hoc** call sites added by TUI-104 /
`sanitize-file-paths-and-labels-in-changed-files-and-checkpoint-views`:

- `views/diff_common/diff_render.rs:46` — `diff_line()` (diff bodies of
  Changed Files + Checkpoints panes)
- `views/diff_common/row.rs:59-60` — `file_row()` (file paths + change type)
- `views/checkpoints/render.rs:195` — checkpoint row labels

Everything else that carries **external (LLM / backend / filesystem / user)
strings** into the TUI is painted **unsanitized**: BoardView work-unit
titles/descriptions, assistant text, thinking, notifications, error text,
supervisor messages, HITL / pause / exec-stdin prompts, role banners,
error/notification/status/loading dialogs, blocklist rules, model-selector
rows, checkpoint labels (partially), the turn-content modal, the
full-screen-shell titles, and the mux status strip.

The user's goal: **cleanse ALL output that goes to the TUI before it is
displayed**, in a DRY / SOLID way — one shared sanitizer, one choke point
(or a small set of well-defined choke points), no per-view reimplementation.

---

## 2. Current State — Complete Sink Inventory

### 2.1 Where the existing sanitizer lives

```
rust/fspec-tui/src/store/agent_view/sanitize.rs
    pub fn sanitize_for_terminal(text: &str) -> String
```

Behaviour (pinned by `sanitize_tests.rs` / `sanitize_tests_streaming.rs`):

1. Strips ANSI escape sequences via a `LazyLock<Regex>` (CSI, OSC, SGR).
2. `\t` → two spaces.
3. `\r` → removed.
4. Control chars `0x00-0x08, 0x0B, 0x0C, 0x0E-0x1F, 0x7F` → removed.
5. `\n` preserved.

It is re-exported at the crate root:
`fspec-tui/src/lib.rs:60` — `pub use store::agent_view::sanitize::sanitize_for_terminal;`

Location smell: it sits under `store/agent_view/` although it is now used by
`views/diff_common` and `views/checkpoints` — it is **not** an
agent-view concern anymore.

### 2.2 Where it is applied today (exhaustive)

| # | File | Call site | Data flow |
|---|------|-----------|-----------|
| 1 | `store/agent_view/chunk_tool_result.rs:94` | `handle_tool_result` | ToolResult content → scrollback |
| 2 | `store/agent_view/chunk_tool_result.rs:146` | `handle_tool_progress` | ToolProgress streaming → scrollback |
| 3 | `views/diff_common/diff_render.rs:46` | `diff_line()` | changed-files / checkpoint diff panes |
| 4 | `views/diff_common/row.rs:59-60` | `file_row()` | file list rows |
| 5 | `views/checkpoints/render.rs:195` | `checkpoint_line()` | checkpoint labels |

### 2.3 What reaches the TUI **unsanitized** (the gap)

External-string sinks, grouped by view. "Sanitizing at store ingress"
would cover rows marked ✅-ingress; "at render" covers rows marked ✅-render.
Most rows are both (double coverage is safe because the function is
idempotent on already-clean text, but the refactor should pick ONE layer).

#### A. AgentView scrollback (`SessionContext` — `store/agent_view/`)

All of these push `ChunkSource { text, .. }` into scrollback:

| Sink | Source file | Function | Text origin |
|------|-------------|----------|-------------|
| Assistant text | `chunk_processor.rs:17` `append_assistant_text` | `StreamChunk::Text` | LLM |
| Thinking | `chunk_processor.rs:54` `append_thinking` | `StreamChunk::Thinking` | LLM |
| Error line | `chunk_processor.rs:125` `handle_error` | `StreamChunk::Error` | LLM provider |
| User input echo | `session_context.rs:91` | `StreamChunk::UserInput` | user (typed/pasted) |
| Interrupted marker | `session_context.rs:117` | `StreamChunk::Interrupted` | static `"⚠ Interrupted"` |
| Notification | `session_context.rs:126` / `push_line` (162) | `StreamChunk::UserNotification`, `Action::EmitSessionNotice` | backend (many `app/dispatch_*` producers) |
| Incoming supervisor msg | `session_context.rs:134` `IncomingMessage` | `[W] role> body` | supervisor session / LLM |
| Reconnect notices | `reconnect_notice.rs:23,39` `push_notice_line` / `replace_notice_by_seq` | `Action::Disconnected` etc. | static + attempt counters |
| Tool-call header (name + args) | `chunk_tool_result.rs:23` `handle_tool_call` via `tool_args.rs:22` `extract_tool_args_display` | tool input JSON (LLM-controlled strings, e.g. `file_path`, `command`) | LLM |
| Edit/Write diff rows | `pending_tool_diff.rs:85` `produce_diff_strings` → `diff_format.rs` / `diff_context.rs` (reads the **post-edit file from disk**) | tool input + filesystem | LLM + fs |
| ToolProgress / ToolResult | `chunk_tool_result.rs` | ✅ already sanitized (rows 1-2 above) | bash output |
| Turn-content modal (Enter on a turn) | `views/agent/turn_modal.rs` wraps `ChunkSource::text` / `full_text` directly | modal renders the stored text **verbatim** | inherits whatever scrollback stored |
| Scrollback copy (COPY-005/006/010) | `views/agent/scrollback_copy.rs`, `full_text_for_seq` (`scrollback_select.rs:113`) | clipboard payload | inherits scrollback content |

Note: the turn modal and copy paths mean that **sanitizing at the
scrollback ingress (ChunkSource.text) is sufficient for every
AgentView text sink** — the modal, copy, re-wrap on resize
(`ScrollbackList::rewrap_at` → `chunk_wrap::wrap_source`) and the
windowing/collapse logic all operate on the stored string.

#### B. AgentView chrome (header / footer / banners / prompts)

| Sink | Source file | Text origin |
|------|-------------|-------------|
| SessionHeader badges (model name, WU id/status, sub-labels) | `views/agent/header_build.rs` | `ModelInfo`, `WorkUnitInfo` (fs/backend) |
| SessionFooter cwd + branch | `views/agent/footer.rs` | `WorkspaceInfo` (fs) |
| Role banner | `views/agent/role_banner.rs` | `AgentViewStore::set_role` (user `/role`, `dispatch_role_dialog.rs`) |
| Inline pause prompt (tool approval) | `views/agent/pause_prompt.rs` (`header_spans`: `state.prompt`, `state.tool_call_id`) | `PauseState` (sessions layer — command / file path) |
| Inline HITL prompt | `views/agent/hitl_prompt.rs` (`header_spans`: `q.header`, `q.question`, options) | `HitlRequest` (LLM-generated `request_user_input`) |
| Exec-stdin prompt | `views/agent/exec_stdin_prompt.rs` | `ExecStdinRequest` (LLM) |
| Compaction progress chip | `views/agent/footer.rs` / `chrome_state.rs` | `CompactionProgress` (backend) |

#### C. BoardView (explicitly called out by the user)

`WorkUnitsLoaded` → `BoardStore::replace_work_units` (`app/dispatch.rs:71`,
`store/board.rs:69`) → painters:

| Sink | Source file | Text origin |
|------|-------------|-------------|
| Card cell labels (`id [N]`, ⏩/🟢 markers) | `views/board/viewport.rs:144` `build_cell_label` → `pad_to_width` → `buf.set_string` | `WorkUnitInfo.id` (fs) |
| Details strip: title, description (2-line wrap), attachments, metadata | `views/board/details_strip.rs:32` `render` (+ `visible_strip_rows` for COPY-009) | `WorkUnitInfo.{id,title,description,attachments,epic,status}` (fs) |
| Search dialog rows (work-unit search) | `components/work_unit_search_rows.rs` | `WorkUnitInfo` (fs) |
| Board header checkpoint counts | `views/board/header.rs` | `CheckpointCounts` (fs, numbers — low risk) |

`WorkUnitInfo.description` / `title` are free-text created via
`fspec create-story --description` — fully attacker/LLM controllable.
**This is the biggest single gap**: no sanitization anywhere in the
board path.

#### D. Changed Files view (`views/changed_files/`)

- `set_files` / `set_diff` (`views/changed_files/mod.rs:149,171`) — raw
  strings from the backend RPC land in the view's `files` / `diff_lines`.
- Diff pane rows go through `diff_line()` ✅ (TUI-104) and file rows through
  `file_row()` ✅.
- Titles: `format!("Changed Files ({count})")` — static, safe.
- Loading dialog label — see §4.

#### E. Checkpoints view (`views/checkpoints/`)

- Labels ✅ at `render.rs:195`.
- File rows ✅ via `file_row()`.
- Diff pane ✅ via `diff_line()`.
- Restore/delete dialog text (`components/checkpoint_restore_dialog.rs`) —
  renders checkpoint names → should sanitize.
- Loading-dialog stage labels — see §4.

#### F. Full-screen-shell mode views (shared chrome)

`views/full_screen_shell.rs` — every lazy mode view (ModelSelector,
ProviderSettings, Blocklist, ChangedFiles, Checkpoints, SearchHistory,
ResumeSession) paints its title + footer here. Titles are mostly static,
except:

- ModelSelector: `"Select Model (N models)"` + refresh state — static.
- SearchHistory: editable query echo `(search): {query}` — user text, typed
  into a real input; control chars impossible (raw-mode char events), low
  priority.
- ResumeSession rows: session summaries from `SessionInfo` (fs) — project
  paths / names.

#### G. Dialogs on the Compositor

| Dialog | File | Text origin |
|--------|------|-------------|
| ErrorDialog | `components/error_dialog.rs` (`maybe_push_error_dialog_for_chunk`, `app/dispatch_dialog_dismiss.rs:58`) | `StreamChunk::Error` — LLM provider error string |
| NotificationDialog / StatusDialog | `components/notification_dialog.rs` | backend notifications |
| LoadingDialog | `components/loading_dialog.rs` — `new(title, label)` | **cascade stage labels embed file paths / checkpoint names** (`LoadTracker::begin_stage` call sites in `app/dispatch_changed_files.rs:64,80`, `app/dispatch_checkpoints.rs:63,99,156`, `app/dispatch_checkpoint_diff.rs:28`). `load_state.rs:76` even documents "views sanitize before passing" — but today **nothing sanitizes**. |
| Create-session / role / work-unit-search / exit-confirmation dialogs | `components/*` | user input (typed) + `WorkUnitInfo` |

#### H. Mux mode

- `views/multiplex/render.rs:186-206` — status strip: pane titles +
  session state (numbers + static strings — low risk).
- Mux panes reuse BoardView / AgentView / ChangedFiles / Checkpoints
  renderers, so they inherit whatever those views do.

### 2.4 Why per-view sanitization is not enough (SOLID argument)

1. **DRY violation today**: the "how do I make this string terminal-safe"
   knowledge is duplicated across 3 feature families (TUI-100 tool output,
   TUI-104 diff output, TUI-104b file paths/labels) at **three different
   architectural layers** (store ingress, view render, view helper). Any new
   sink (a new view, a new dialog, a new stream-chunk variant) has to
   remember the rule again — which is exactly what the user is asking to
   eliminate.
2. **Open/closed**: adding a `StreamChunk` variant or a new view should
   require **zero** new sanitize calls. Only achievable if the choke points
   are structural (all ingress, or all render), not call-site.
3. **Single Responsibility**: `sanitize.rs` under `store/agent_view/` mixes
   a cross-cutting terminal-safety concern with agent-view state. It should
   be a crate-level `terminal`/`sanitize` module.
4. **Consistency (Liskov-ish)**: `sanitize_for_terminal` is not
   idempotence-documented, and each call site has subtly different
   pre/post handling (tabs, truncation-before-sanitize at
   `details_strip.rs` vs sanitize-before-truncate at
   `checkpoints/render.rs:194-197`). One canonical order
   (sanitize → then width math) must be enforced by structure.

---

## 3. Design Options

### Option A — Sanitize at every **store/ingress** boundary (RECOMMENDED)

Move the sanitizer to a crate-root module and call it at the **data ingress
points**, so every view paints data that was cleaned exactly once:

```
rust/fspec-tui/src/
  terminal/            (new module, or reuse existing terminal.rs namespace)
    sanitize.rs        (move from store/agent_view/sanitize.rs)
```

Choke points (exhaustive list — this is the whole refactor):

1. **`SessionContext::record_chunk`** (`store/agent_view/session_context.rs`)
   — sanitize the payload of EVERY chunk variant that becomes visible text:
   `Text`, `Thinking`, `UserInput`, `Error`, `UserNotification`,
   `IncomingMessage`, `ToolCall` (name+args display string), `ToolResult`,
   `ToolProgress`, `Interrupted`. This single match arm covers the whole
   scrollback, the turn modal, copy, re-wrap, and mux agent panes, because
   every one of them re-reads `ChunkSource::text`.
   - Implementation note: do the sanitization *inside* `record_chunk`'s
     arms (before `append_assistant_text` etc. see the text), or normalize
     at the top of `record_chunk` via a small `&StreamChunk → sanitized
     payload` helper. Keep `ChunkSource` construction unchanged so the
     existing RPC-091/093 accumulation tests keep passing.
   - The tool-diff path (`pending_tool_diff`) is the exception: diff *content*
     comes from tool input + the filesystem; sanitize each diff *line* at
     `produce_diff_strings` (or in `diff_format` row building) — see §5 R5.
2. **`BoardStore::replace_work_units`** (`store/board.rs:69`) — sanitize
   `id`, `title`, `description`, `epic`, and each attachment path on ingress.
   Covers board cards, details strip, search dialog, and the mux board pane.
   (Numbers/`status`/`work_type` are closed vocabularies — skip or
   cheaply include.)
   - Note: `WorkUnitInfo` is shared with `AgentViewStore`
     (`sync_work_unit_contexts`) — the header WU chip also gets covered if
     we sanitize at the shared ingress. Decision: sanitize inside
     `replace_work_units` (BoardStore owns the projection) and reuse the
     same sanitized snapshot for the agent header via
     `sync_work_unit_contexts` — or, simpler: sanitize in a tiny
     `sanitize_work_unit(&WorkUnitInfo) -> WorkUnitInfo` helper called from
     both call sites (DRY, single definition of "which fields").
3. **Prompt/pause exec-stdin ingress** — `AgentViewStore::set_hitl_prompt`
   (`hitl_state.rs:142`), `set_pause_state` (`pause_state.rs:40`),
   `set_exec_stdin` (`exec_stdin_state.rs:40`), `set_role`
   (`role_state.rs:30`) — sanitize the display strings at store write.
4. **`Action::EmitSessionNotice` handler**
   (`app/dispatch_slash_clear.rs:30` `handle_emit_session_notice`) — sanitize
   before `push_line`. (Alternatively covered by #1 if notices flow as
   chunks — they don't; this is the direct path.)
5. **ErrorDialog / NotificationDialog / StatusDialog / LoadingDialog
   constructors** (`components/*`) — sanitize the message/title/label in
   `new()`. This is the *last-mile* guarantee for any dialog text that
   originates from backend error strings, cascade labels (which embed file
   paths), or provider errors. Sanitizing in `new()` is safe (dialogs are
   leaf widgets) and makes the compositor stack structurally safe.
6. **ChangedFiles / Checkpoints / Resume / Search / Blocklist /
   ModelSelector view state setters** (`set_files`, `set_diff_lines`,
   `set_checkpoints`, `set_sessions`, `set_rules`, model rows) — sanitize
   the incoming rows. The existing TUI-104/TUI-104b call sites
   (`diff_line`, `file_row`, `checkpoint_line`) then become redundant and
   can be **deleted** (DRY payoff) — or kept as defense-in-depth.
   Recommended: delete them once ingress covers the same data, and rely on
   one layer.
7. **Reconnect notices** (`reconnect_notice.rs`) — text is static +
   counters; optional.

**Properties:**
- Views stay dumb: they paint store data; they never need to know terminal
  safety rules.
- New view/dialog/chunk-variant: if it routes through an existing store,
  zero new sanitize calls. If it introduces a *new* ingress (new RPC
  payload type), it is added to the ingress checklist — still one rule.
- Idempotent: running the sanitizer on already-clean text is a no-op
  (no ANSI/control chars present → the regex pass + char filter change
  nothing), so double sanitization at ingress + render is harmless during
  the migration.
- Cost: per-frame cost moves from N render call sites to N ingress events
  (ingress happens on RPC events, far less often than frames).
- Risk: stored scrollback text changes (already-sanitized instead of
  raw). Any test that asserts raw content in scrollback must be updated.
  COPY-005/006/008/009/010 copy payloads become clean text — which is the
  *desired* behaviour (today, copying tool output copies ANSI codes).

### Option B — Sanitize at every **render** boundary (rejected as primary)

A `SanitizedText` wrapper (newtype over `String` that lazily or eagerly
sanitizes) used at `Paragraph::new(...)` / `buf.set_string(...)` call sites,
or a `buf`-level pre-filter.

- Pros: truly "last mile", catches anything that missed ingress.
- Cons:
  - ~40+ paint call sites across 15+ view modules; every new span/line must
    remember the wrapper. This is the DRY failure we are trying to kill.
  - `buf.set_string` already *drops* control-char graphemes (ratatui 0.29
    `buffer.rs` filters `is_control()`), so partial safety already exists at
    paint time — but it is silent and does not fix width math, tabs, or
    ANSI sequences that *precede* a control char within a grapheme.
  - Wrapping at render time breaks width/truncation invariants in several
    places (sanitize can change display width; truncation must happen
    AFTER sanitize).
- Verdict: keep as an *invariant test* (see §6), not as the mechanism.

### Option C — Buffer-level pre-flight (rejected)

A final pass over the entire `Buffer` before `terminal.draw()` in
`app/run_loop.rs` / `terminal.rs`.

- Pros: literally "ALL output".
- Cons:
  - The buffer holds *styled grapheme cells*, not raw strings — ANSI
    sequences have by then been split into cell symbols; you cannot
    reliably re-assemble and strip them. Control chars are already dropped
    by ratatui. Tabs/width issues are already baked into cell offsets.
  - Per-frame O(width×height) scan of every cell on every frame —
    measurable CPU on 4K terminals.
  - Cannot fix scrollback *copy* (clipboard) or the *stored* text.
  - Hides rather than fixes: bad data still pollutes stores, widths, and
    copy.
- Verdict: rejected as mechanism. Can be a cheap *assertion* in debug
  builds (no `\x1b` in any cell symbol) — see §6.

**Recommendation: Option A** (ingress sanitization at a closed set of
store/action/dialog boundaries), with the sanitizer promoted to a
crate-level module and all existing per-view call sites deleted.

---

## 4. The Canonical Sanitizer

Move + rename (keep `sanitize_for_terminal` as the function name for test
stability; module move is the refactor):

```
rust/fspec-tui/src/store/agent_view/sanitize.rs
rust/fspec-tui/src/store/agent_view/sanitize_tests.rs
rust/fspec-tui/src/store/agent_view/sanitize_tests_streaming.rs
        ↓
rust/fspec-tui/src/terminal/sanitize.rs      (module `terminal::sanitize`)
```

Keep:
- The ANSI regex (`\x1b(?:\[[0-9;]*[A-Za-z]|\][^\x07]*\x07|[^\x1b])`) —
  pinned by TUI-100 tests.
- The control-char set `0x00-0x08, 0x0B, 0x0C, 0x0E-0x1F, 0x7F`.
- `\t` → 2 spaces, `\r` → drop, `\n` preserved.

Add (decisions to make during specifying — proposed as defaults):
- **`sanitize_lines(&str) -> Vec<String>` / `sanitize_in_place`-style
  helpers** for row-list ingress (diff lines, file rows, blocklist rules,
  model rows) so per-row loops don't re-allocate needlessly. A simple
  `iter.map(sanitize_for_terminal).collect()` wrapper is sufficient; a
  batch variant is an optimization, not a requirement.
- **Document idempotence** as an explicit contract + proptest:
  `sanitize(sanitize(x)) == sanitize(x)` for all x.
- Keep the function pure and allocation-honest (it already allocates a new
  `String`; fine).
- **Do NOT** change `STDERR_MARKER` handling: markers are added *after*
  sanitization today (`maybe_mark` on sanitized text) and stripped again at
  render (`strip_marker` in `chunk_wrap.rs`). Moving sanitization to
  `record_chunk` must preserve that order: sanitize → mark → store.
- **Do NOT** touch the diff codec (`ELISION_SENTINEL = \u{1}` — a C0
  control char deliberately used *inside* stored diff text). Consequence:
  diff-card bodies must NOT be run through `sanitize_for_terminal` after
  `to_line` encoding. Sanitize the *source lines* (tool input + context
  file lines) *before* `build_diff_rows`/`to_line` encoding instead — see
  §5 R5.

---

## 5. Refactor Plan (DRY/SOLID breakdown)

> Estimates assume the existing test suite (fspec-tui has ~100 integration
> test files; the chunk/scrollback/parity tests are strict golden-string
> tests that will catch regressions).

**R1 — Promote the sanitizer module.** (≈ 1 point)
Move `sanitize.rs` + both test files to `terminal/sanitize.rs`; update the
`pub use` in `lib.rs`; update all 5 existing call sites' imports. No
behaviour change. Pure move — trivially testable.

**R2 — Sanitize AgentView scrollback at ingress.** (≈ 3 points)
In `SessionContext::record_chunk`, sanitize the text payload of every
visible chunk variant (list in §3.1). Delete the now-redundant
`sanitize_for_terminal` calls in `chunk_tool_result.rs` (2 call sites) —
they move into the ingress. Keep stderr-marker ordering.
Update affected golden tests (chunkprocessor-parity,
chunk_rendering_parity_rpc078/071, scrollback_pty_rpc078,
thinking_streaming_parity_rpc093).

**R3 — Sanitize BoardView data at ingress.** (≈ 2 points)
New `fn sanitize_work_unit(&mut WorkUnitInfo)` (fields: `id`, `title`,
`description`, `epic`, attachment paths) called from
`BoardStore::replace_work_units` and (optionally)
`AgentViewStore::sync_work_unit_contexts` for the header chip.
Feature-file territory: board card labels + details strip now display
clean text; COPY-009 `visible_strip_rows` output becomes clean.

**R4 — Sanitize store-set prompt/chrome text.** (≈ 2 points)
`set_hitl_prompt`, `set_pause_state`, `set_exec_stdin`, `set_role`,
`push_line` / `handle_emit_session_notice`, reconnect-notice text:
sanitize at the store write (or the single `push_line` choke point).

**R5 — Sanitize tool-diff source lines.** (≈ 2 points, trickiest)
In `pending_tool_diff::produce_diff_strings` / `diff_format::build_diff_rows`
sanitize each source line (tool input strings + context file lines read
from disk) BEFORE the diff codec encodes them. This keeps the codec's
`\u{1}` elision sentinel untouched and covers the inline diff card +
`full_text` (turn modal) + copy in one move.

**R6 — Sanitize mode-view + dialog ingresses.** (≈ 3 points)
- `ChangedFilesView::set_files` / `set_diff_lines` (or the dispatch
  handlers feeding them), `CheckpointsView::set_checkpoints/set_files/
  set_diff_lines`, `ResumeSessionView::set_sessions`,
  `SearchHistoryView` rows, `BlocklistView::set_rules`,
  model-selector rows.
- Dialog `new()` sanitizers: `ErrorDialog`, `NotificationDialog`,
  `StatusDialog`, `LoadingDialog` (title + label — this catches the
  cascade labels that embed file paths), checkpoint-restore dialog.
- Delete the TUI-104/TUI-104b render-time call sites
  (`diff_render.rs`, `row.rs`, `checkpoints/render.rs:195`) once their
  ingress is covered — the DRY payoff. (Migration order: land R6 ingress
  first, run the suite, *then* delete render call sites.)

**R7 — Invariant tests (cheap, high value).** (≈ 2 points)
- Proptest: idempotence of `sanitize_for_terminal`.
- Golden integration tests per view: feed a hostile payload (ANSI + tabs +
  CR + NUL + lone `\x1b`) through the *ingress* and assert the rendered
  buffer contains none of it (buffer-level assertion — the "Option B"
  idea used as a *test*, not a mechanism). Cover: board details strip,
  board card, agent scrollback (all 10 chunk variants), error dialog,
  loading-dialog label, checkpoint label, changed-files file row + diff
  line, HITL/pause/exec-stdin prompt, role banner, turn modal.
- Optional debug-only assertion in the run loop: no `\x1b` char in any
  buffer cell symbol before `draw()` (guarded by `debug_assert!`, cheap
  enough in debug only).

**R8 — Feature files + tags (ACDD discipline).** (≈ 1-2 points)
One new capability feature file (e.g.
`sanitize-all-tui-output-at-ingress.feature`) with a scenario per sink
group; tag it (e.g. `@TUI-111` or the existing sanitize family tag);
link coverage for each scenario to its ingress test. Keep the legacy
TUI-100/104 features as historical record (their scenarios stay true —
sanitization still happens for those paths, just at a different layer;
if any scenario text says "at the TUI display layer" we should amend the
wording to "before display").

Total: ≈ 13-16 story points → at the upper edge of acceptable; split
candidate: (R1+R2+R7-agent) as TUI-111 and (R3+R4+R5+R6+R7-views) as
TUI-112, or deliver as one card with 2 waves.

---

## 6. Testing Strategy (per repo guidelines)

- All new code: unit tests at the ingress (`#[test]` in store modules) +
  integration tests in `fspec-tui/tests/*.rs` using the existing
  `tests/common/harness.rs` mock-backend harness (render the App, drive
  hostile chunks/RPC payloads, assert on `Buffer` contents).
- Use `proptest` for idempotence + "sanitizer never panics on arbitrary
  bytes" (the ANSI regex over arbitrary `\x1b`-laden strings is the
  classic foot-gun — property: `sanitize` output contains no `\x1b` and no
  control char from the forbidden set, for random inputs).
- Golden-string parity tests already exist for the chunk processor; they
  must be the regression net for R2/R5.
- Scope test runs: `cargo test -p codelet-fspec-tui --test <name>` per
  file (NEVER unscoped `cargo test --workspace` — see CLAUDE.md).

---

## 7. Risks & Open Questions

1. **Width-shifting**: sanitizing changes display width (tab→2 spaces,
   control removal). Anything computing width BEFORE sanitize is now
   wrong. Audit: `details_strip.rs` computes `truncate_to` on the raw
   string today; post-refactor it must truncate on the sanitized string.
   (The refactor standardizes to: sanitize at ingress → all downstream
   width math sees clean text. No per-view fixes needed.)
2. **Elision sentinel `\u{1}`** inside diff codec lines (see §4) — must
   not be stripped. R5's "sanitize before encode" design avoids this; a
   blanket "sanitize ChunkSource.text" would corrupt it. Keep the diff
   path on its own pre-encode lane.
3. **User-input echo**: `UserInput` text comes from the shared
   `MultiLineInput`; raw mode makes control chars near-impossible, but
   bracketed paste of binary-ish text is possible. Sanitizing it at ingress
   is harmless (idempotent on clean text) — include it.
4. **LLM raw-output contract (TUI-100 rule 6)**: the LLM must keep
   receiving RAW tool output. Ingress sanitization in `fspec-tui` does not
   affect `rust/tools` or the session engine — the contract holds. The
   existing integration scenario "LLM receives raw unsanitized output
   while TUI gets sanitized" still passes.
5. **OSC 52 clipboard writes** (`mouse/clipboard.rs`) copy
   *rendered* text — becomes clean automatically.
6. **Multiplex**: inherits from the underlying views — no extra work, but
   one integration test through the mux path is worth adding (R7).
7. **`sanitize_for_terminal` is `pub` at the crate root** — external
   consumers (tests, possibly `fspec` CLI) import it. The move must keep
   the re-export (or update the 3 in-repo importers + re-export for
   backward compat).

---

## 8. Source Map (files the refactor touches)

```
fspec-tui/src/terminal/sanitize.rs            (new — moved)
fspec-tui/src/terminal/sanitize_tests.rs      (new — moved)
fspec-tui/src/terminal/sanitize_tests_streaming.rs (new — moved)
fspec-tui/src/lib.rs                          (re-export)
fspec-tui/src/store/agent_view/session_context.rs     (R2 ingress)
fspec-tui/src/store/agent_view/chunk_tool_result.rs   (R2: delete 2 call sites)
fspec-tui/src/store/agent_view/chunk_processor.rs     (R2: arms already covered via record_chunk)
fspec-tui/src/store/agent_view/reconnect_notice.rs    (R4, optional)
fspec-tui/src/store/agent_view/hitl_state.rs          (R4)
fspec-tui/src/store/agent_view/pause_state.rs         (R4)
fspec-tui/src/store/agent_view/exec_stdin_state.rs    (R4)
fspec-tui/src/store/agent_view/role_state.rs          (R4)
fspec-tui/src/store/agent_view/pending_tool_diff.rs   (R5)
fspec-tui/src/store/agent_view/diff_format.rs         (R5)
fspec-tui/src/store/agent_view/diff_context.rs        (R5)
fspec-tui/src/store/board.rs                          (R3)
fspec-tui/src/store/agent_view/work_unit_state.rs     (R3, optional)
fspec-tui/src/app/dispatch_slash_clear.rs             (R4: notice handler)
fspec-tui/src/views/changed_files/mod.rs              (R6)
fspec-tui/src/views/checkpoints/mod.rs                (R6)
fspec-tui/src/views/agent/resume_session_view.rs      (R6)
fspec-tui/src/views/blocklist/mod.rs                  (R6)
fspec-tui/src/views/model_selector/*                  (R6, rows)
fspec-tui/src/components/error_dialog.rs              (R6)
fspec-tui/src/components/notification_dialog.rs      (R6)
fspec-tui/src/components/loading_dialog.rs           (R6)
fspec-tui/src/components/checkpoint_restore_dialog.rs(R6)
fspec-tui/src/views/diff_common/diff_render.rs        (R6: delete call site)
fspec-tui/src/views/diff_common/row.rs                (R6: delete call sites)
fspec-tui/src/views/checkpoints/render.rs            (R6: delete call site)
fspec-tui/tests/*                                     (R7 new + updated goldens)
spec/features/sanitize-all-tui-output-at-ingress.feature (R8)
```

---

## 9. Conclusion

The clean DRY/SOLID design is **Option A: ingress sanitization at a closed
set of boundaries** (stream-chunk recorder, board store, prompt/chrome
store setters, notice handler, view state setters, dialog constructors),
backed by a **promoted crate-level `terminal::sanitize` module** and
**buffer-level invariant tests**. It makes "all TUI output is clean" a
*structural* property rather than a per-call-site memory, keeps the LLM
raw-output contract, preserves the diff codec's sentinel design, and is
idempotent enough to migrate incrementally (R1 → R8) with the existing
golden-string test suite as the safety net.
