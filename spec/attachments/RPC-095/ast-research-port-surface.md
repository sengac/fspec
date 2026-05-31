# RPC-095 — AST Research Findings

## Goal
Identify the existing Rust render-loop tick, Action enum, and surrounding plumbing that the spinner / busy-state / Esc-cascade extensions need to integrate with.

## Findings

### 1. Render tick already exists at 60 fps
- File: `codelet/fspec-tui/src/app/events.rs`
- `const RENDER_TICK: Duration = Duration::from_millis(16);` (line 28)
- `tokio::time::interval(RENDER_TICK)` (line 170) — already drives idle repaints
- **Implication**: a dedicated 80 ms `SpinnerTick` action/timer is NOT required. The existing 60 fps loop repaints often enough; the spinner just needs to compute its frame from `elapsed_ms / 80` at render time using a monotonic timestamp captured when the busy state began.

### 2. Action enum location
- `codelet/fspec-tui/src/components/mod.rs:105` — `pub enum Action { ... }`
- Recent additions (per RPC-094): 5 ScrollbackList* + MouseWheel* variants. New `Action::SpinnerTick` is NOT needed (see §1).

### 3. is_loading / tokens_per_second already plumbed in header
- `codelet/fspec-tui/src/views/agent/header_build.rs:128-144` already implements the chip logic conditional on `is_loading && tokens_per_second.is_some()`
- `codelet/fspec-tui/src/views/agent.rs:253-259` hard-codes both to `false`/`None`. Wiring this to session status is a one-line change once we have `store.session_status_for(&sid)`.

### 4. Existing public constants pattern in views/agent/
- `slash_commands.rs:87`: `pub const SLASH_COMMANDS: &[SlashCommand] = &[...]`
- `header.rs:40`: `pub(crate) const HEADER_BG: Color = ...`
- `footer.rs:33`: `pub(crate) const FOOTER_BG: Color = ...`
- → New `spinner.rs` should expose `pub const DOTS_FRAMES: [&str; 10] = [...]` and `pub const DOTS_INTERVAL_MS: u64 = 80;` in the same idiom.

### 5. Esc cascade entry point
- `codelet/fspec-tui/src/app/dispatch_rpc051.rs::handle_agent_esc_pressed` is THE place to insert the L6 (clear input) branch between the L4 interrupt and L5 BackToBoard arms.
- Plumbing needed: read `input_is_nonempty` either from a new `App` accessor or by exposing a method on `agent_view_store`. Simplest: query `self.agent_view_store` for current input snapshot OR call into `self.navigator.agent.input.value().trim().is_empty()` (already accessible via Navigator).

### 6. MultiLineInput edit surface
- `codelet/fspec-tui/src/views/agent/multiline_input.rs`:
  - `handle_key(code, mods)` is the single edit entry point.
  - To gate edits during compaction, add an `InputGate` parameter or a parallel `handle_key_gated(code, mods, gate)`.
  - Esc is NOT special-cased here — it's intercepted at the AgentView dispatch level (good for parity).

### 7. 300-LoC ceiling enforcement
- `codelet/fspec-tui/tests/source_shape_rpc013.rs:138` enforces all `views/agent/*.rs` modules < 300 LoC.
- New modules `spinner.rs` and `input_transition.rs` must respect this.

## File touch list (forecast)

| File | Action |
|---|---|
| `codelet/fspec-tui/src/views/agent/spinner.rs` | NEW — frames table + frame-picker + painter |
| `codelet/fspec-tui/src/views/agent/input_transition.rs` | NEW — render dispatcher (spinner-or-input) |
| `codelet/fspec-tui/src/views/agent.rs` | DELETE `PLACEHOLDER_FOOTER_HINTS`; wire `is_loading`; track `spinner_started_at: Option<Instant>`; call `input_transition::render` |
| `codelet/fspec-tui/src/views/agent/multiline_input.rs` | Add `InputGate { block_edits, suppress_enter }`; gate handle_key paths |
| `codelet/fspec-tui/src/views/agent/dispatch.rs` | Pass `InputGate` when forwarding key events |
| `codelet/fspec-tui/src/app/dispatch_rpc051.rs` | Insert L6 input-clear branch before L5 BackToBoard |

## Open verification items

- **V1:** confirm `agent_view_store.session_status_for(&id)` exists and returns `&SessionStatus` (used in `dispatch_rpc051.rs:47`). ✅ already there.
- **V2:** confirm `Navigator.agent.input` is reachable from `App` for L6 input-clear. ✅ already accessed for `cursor_position` at events.rs:184.

## Recommended implementation order (TDD)

1. spinner.rs — unit tests for frame-picker + glyph table.
2. spinner.rs — painter test (snapshot a 1-row paint).
3. input_transition.rs — render-dispatch tests (spinner shown when loading, placeholder shown otherwise).
4. multiline_input.rs — InputGate tests (block_edits swallows printable/backspace/delete/Enter; cursor moves preserved).
5. agent.rs — wire is_loading + spinner_started_at + call input_transition::render.
6. dispatch_rpc051.rs — add L6 clear-input branch + unit test.
7. Re-run the full `cargo test -p codelet-fspec-tui` suite.
