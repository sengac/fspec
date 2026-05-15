# RPC-019 AST Research Findings

Date: 2026-05-15
Performed during: Example Mapping / specifying phase
Scope: codelet/fspec-tui/src/ (Rust ratatui crate)

## Goal
Identify the existing widget surface that RPC-019 must replace + the
call-sites that depend on it, so that the new MultiLineInput +
ScrollbackList land additively without breaking RPC-018 invariants.

## 1. Existing AgentView struct + impl

Result of: `pub struct AgentView { $$$FIELDS }`

```
codelet/fspec-tui/src/views/agent.rs:67  pub struct AgentView {
    pub scrollback: Vec<RenderedChunk>,
    pub input: Input,                    // tui_input::Input — replaced by MultiLineInput
    pub scroll_offset: u16,
    pub stick_to_bottom: bool,
    pub action_tx: Option<UnboundedSender<Action>>,
    pub next_seq: u64,
    pub last_input_area: Option<Rect>,
}
```

Result of: `impl AgentView { $$$METHODS }`
- `views/agent.rs:91 impl AgentView { ... }` — single impl block.
- Methods called from outside this file:
  - `AgentView::new(action_tx)` — constructor used by Navigator + tests.
  - `chunk_count()` — tests.
  - `cursor_position() -> Option<(u16, u16)>` — called by App in
    `app/events.rs:156, 194` when active_view == Agent.
  - `push_line<S>(line)` — called by App::dispatch in
    `app/dispatch.rs:279, 284` to record user echo + notice.
  - `record_chunk(&StreamChunk)` — called by App::dispatch:38 when
    Action::ChunkReceived arrives.
  - `handle_event(&Event) -> EventResult` — called by Compositor.
  - `render_with_store(area, buf, &AgentViewStore)` — called by
    Navigator.

## 2. RenderedChunk

```
codelet/fspec-tui/src/views/agent.rs:59  pub struct RenderedChunk {
    pub seq: u64,
    pub lines: Vec<Line<'static>>,
}
```

→ Pre-renders Gherkin chunks into `ratatui::text::Line` vectors. The
ScrollbackList in RPC-019 KEEPS this representation — it just iterates
only the visible window instead of flattening into a giant `Paragraph`.

## 3. Action enum surface to extend

```
codelet/fspec-tui/src/components/mod.rs:86  pub enum Action { ... }
```

Variants relevant to RPC-019 already exist:
- `Quit`, `BackToBoard`, `InputSubmitted(String)`, `Interrupt`,
  `ChunkReceived(SessionId, StreamChunk)`.

NEW variants RPC-019 must add (App::dispatch routing deferred to RPC-021):
- `HistoryPrev`
- `HistoryNext`
- `SessionPrev`
- `SessionNext`

## 4. Call-sites that touch the soon-to-be-replaced `view.input` field

Pattern: `view\.input|\.input =|\.input\.`

ONLY two production references (both inside agent.rs itself, behind
push_line/cursor_position/etc — covered by the methods listed in §1).

External test references (must be migrated when the struct changes):
- `codelet/fspec-tui/tests/view_agent_unit_rpc013.rs:143`
  `view.input = view.input.clone().with_value("hi".to_string());`
- `codelet/fspec-tui/tests/view_agent_unit_rpc013.rs:151`
  `assert_eq!(view.input.value(), "");`

→ Either expose `set_input_text(&str)` + `input_value()` on AgentView,
  or migrate the RPC-013 test to a behaviour-level assertion (drive
  `handle_event` with KeyCode::Char keystrokes and read the resulting
  Action::InputSubmitted from the rx). The latter is preferable so the
  RPC-013 test stops poking internals.

## 5. tui_input::Input import audit

Pattern: `tui_input::Input` (glob *.rs in codelet/)

Only ONE production hit:
- `codelet/fspec-tui/src/views/agent.rs:36 use tui_input::Input;`

→ Removing this import is sufficient for the
  `rpc019-source-shape.feature` scenario:
  "AgentView orchestrator now wires the new widgets — the file does
   NOT contain the substring `tui_input::Input`".

## 6. Existing crate-level deps

`codelet/Cargo.toml:78  tui-input = "0.10"`

`codelet/fspec-tui/Cargo.toml:28  tui-input.workspace = true`

→ tui-input STAYS as a workspace dep (other future widgets may use it).
  RPC-019 ADDS `tui-textarea = "0.7"` next to it, also workspace-shared
  so future cards can reuse it (e.g. RPC-020 slash command palette's
  search input).

## 7. New file budget (per RPC-002 invariant "every new module file
   under 300 LoC" — RPC-012 rule [10])

Planned new files:
- `codelet/fspec-tui/src/views/agent/multiline_input.rs` — wraps
  `tui_textarea::TextArea<'static>`. Expected ~200 LoC.
- `codelet/fspec-tui/src/views/agent/scrollback.rs` — owns
  `Vec<RenderedChunk>`, ScrollState, viewport-window render. Expected
  ~180 LoC.

Modified files (must stay < 300 LoC):
- `codelet/fspec-tui/src/views/agent.rs` — currently 281 lines. The
  refactor SHRINKS it because the input handling moves into
  multiline_input.rs and the scrollback walk moves into scrollback.rs.

## 8. RPC/NAPI surface impact

NONE. Per master plan (`spec/attachments/RPC-002/rust-tui-parity-master-plan-2026-05-13.md`)
RPC-019 row reads "Touches RPC/NAPI? No". Verified by checking
`codelet/rpc/src/lib.rs::FspecService` — no new method needed.

## 9. Downstream cards depending on this surface

Confirmed via the master plan card map:
- **RPC-020** "Slash command palette + @file search popup" — needs the
  MultiLineInput's `/` and `@` event hooks. RPC-020 will add a third
  variant to InputEventOutcome (or layer a popup on top) — RPC-019
  leaves InputEventOutcome additive-friendly (non-#[non_exhaustive]
  but the enum is private to the module, so adding a variant is a
  local change).
- **RPC-021** "Multi-session + Shift navigation + command history" —
  consumes the four new Action variants `HistoryPrev/Next/SessionPrev/Next`.
  RPC-019 only needs to EMIT them; routing through App::dispatch is
  RPC-021's job. RPC-021 also wires command history persistence via
  `persistence_get/add/search_history` (new RPC methods).
- **RPC-022** "Modal dialogs" — orthogonal; not blocked by RPC-019.

## 10. Invariants to preserve (from RPC-002 master plan)

1. Single-task mutation — all store mutations via `App::dispatch`. The
   ScrollbackList lives INSIDE `AgentView` (which is owned by the App
   task — Navigator.agent), so its mutating methods (`push`, `scroll_*`)
   stay on the App task. ✓
2. Host-supplied tokio runtime — only `tokio::spawn`. RPC-019 widgets
   do not spawn anything. ✓
3. Loopback-only WebSocket bind — unaffected.
4. `codelet-napi` is NOT a dep — unaffected.
5. File-size discipline (< 300 LoC) — see §7. ✓
6. Source-shape regression — covered by new `rpc019-source-shape.feature`.
7. Cross-transport parity — N/A, no new backend methods.
