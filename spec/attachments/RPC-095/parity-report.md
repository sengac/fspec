# RPC-095 — AgentView MultiLineInput Parity Report

**Scope:** Bring the new Rust ratatui AgentView input row up to byte-for-byte behavioural parity with the original TypeScript Ink reference implementation across spinner/busy display, placeholder text, blocking state, and the Esc-key cascade.

**Source-of-truth references (TypeScript):**
- `src/tui/components/MultiLineInput.tsx`
- `src/tui/components/InputTransition.tsx`
- `src/tui/components/ThinkingIndicator.tsx`
- `src/tui/components/AgentView.tsx`
- `src/tui/components/multiline-input-compaction-logic.ts`

**Target files (Rust):**
- `codelet/fspec-tui/src/views/agent.rs`
- `codelet/fspec-tui/src/views/agent/multiline_input.rs`
- `codelet/fspec-tui/src/views/agent/dispatch.rs`
- `codelet/fspec-tui/src/views/agent/header.rs` + `header_build.rs`
- `codelet/fspec-tui/src/views/agent/footer.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc051.rs`
- (new) `codelet/fspec-tui/src/views/agent/spinner.rs`
- (new) `codelet/fspec-tui/src/views/agent/input_transition.rs`

---

## 1. Spinner display while busy

### TypeScript behaviour

**`ThinkingIndicator.tsx`** (lines 17–48) defines 6 spinner styles. The AgentView/InputTransition use the `'dots'` braille spinner:

```
['⠋','⠙','⠹','⠸','⠼','⠴','⠦','⠧','⠇','⠏']   // 10 frames, 80 ms interval
```

**Format:** `"{spinnerChar} {message}... {hint}"` — rendered as one dim `<Text>` line (`ThinkingIndicator.tsx:132-136`).

**Where it renders:** Inside the input row, *replacing* the `MultiLineInput`. `InputTransition.tsx:545-566` swaps in the spinner during `animationPhase === 'loading'`.

**Variants observed:**
- Loading: `"⠋ Thinking... (Esc to stop | 'Shift+←/→' sessions | 'Tab' select turn)"` (`InputTransition.tsx:155`)
- Compacting: `"⠋ Compacting... (Esc to stop)"` (`InputTransition.tsx:548-555`)

**Animation lifecycle:** When `isLoading || isCompacting` flips false, `InputTransition` runs a character-by-character "hide" → "show" reveal of the placeholder (`InputTransition.tsx:343-383`). This is polish, not behaviour-critical for parity.

**Header signal:** `SessionHeader` shows magenta `tokens/sec` chip when `isLoading=true` (via `displayedTokPerSec`, `AgentView.tsx:5226`).

### Current Rust state

- **No spinner anywhere** in `codelet/fspec-tui/src/`. No `ThinkingIndicator`, no `InputTransition`, no braille-frame animator. (Only matches for `ThinkingLevel` config enum and `ChunkKind::Thinking` content.)
- The `is_loading` flag is plumbed into `SessionHeader` (`header.rs:73`, `header_build.rs:138-144`) but `agent.rs:258` hard-codes `is_loading: false, tokens_per_second: None`. Header tok/s chip is **dead-wired**.
- Input row unconditionally renders `multiline_input::render_with_prompt` (the static placeholder) — `agent.rs:289-290`.

### Required Rust changes

1. **New module** `views/agent/spinner.rs` exposing:
   - `pub const DOTS_FRAMES: [&str; 10] = ["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"];`
   - `pub const DOTS_INTERVAL_MS: u64 = 80;`
   - A pure function `current_frame(elapsed_ms: u64) -> &'static str` returning `DOTS_FRAMES[(elapsed_ms / DOTS_INTERVAL_MS) as usize % 10]`.

2. **New module** `views/agent/input_transition.rs` exposing `render_input_or_spinner(area, buf, state)` that picks one of:
   - Spinner+message line when `is_loading || is_compacting`.
   - Pause/HITL block when applicable (out of scope for this card if it pulls in too much — see §7).
   - Otherwise delegate to `multiline_input::render_with_prompt`.

3. **agent.rs:** kick a 80 ms tick action while `is_loading || is_compacting`. Pattern: piggyback on the existing AgentView render-loop tick used elsewhere; if none, emit an `Action::SpinnerTick` with elapsed monotonic time.

4. **agent.rs:253-259:** stop hard-coding `is_loading: false, tokens_per_second: None`. Compute `is_loading = matches!(session.status, SessionStatus::Running)` and read tok/s from the existing `tokens_per_second_for(sid)` (see RPC-086 surface; if unavailable, leave `None` and at least wire `is_loading`).

### Strings (verbatim from TS)

- Loading message: `Thinking`
- Loading hint: `(Esc to stop | 'Shift+←/↓' sessions | 'Tab' select turn)` *(see TS `InputTransition.tsx:155` — copy exact glyphs and order)*
- Compacting message: `Compacting`
- Compacting hint: `(Esc to stop)`
- Style: dim (`Modifier::DIM`), default fg.

---

## 2. Placeholder text

### TypeScript behaviour

Three distinct placeholders routed via `getDisplayPlaceholderForMultiLineInput()` (`multiline-input-compaction-logic.ts:65-85`):

| State | Placeholder text | Source |
|---|---|---|
| Idle (default) | `Type a message... ('Shift+↑/↓' history \| 'Shift+←/→' sessions \| 'Tab' select turn)` | `AgentView.tsx:5446`, `InputTransition.tsx:156` |
| HITL freeform | `Type your answer...` | `InputTransition.tsx:418` |
| Compacting | `formatCompactionPlaceholder(progress)` — phase + counts string | `MultiLineInput.tsx:351-358` |

Rendered as dim text with an inverse-block cursor (`MultiLineInput.tsx:362-371`).

### Current Rust state

- Single hard-coded constant `INPUT_PLACEHOLDER_HINT` (`agent.rs:75-76`) — matches the **idle** TS string verbatim. ✅
- No HITL placeholder. ❌
- No compaction placeholder swap — compaction info lives in the **footer chip** (`footer.rs:104-118`) rather than the placeholder. ⚠️ (design divergence — see §7).
- Dangling unused constant `PLACEHOLDER_FOOTER_HINTS = "Enter=send  Ctrl+C=interrupt  ESC=back"` at `agent.rs:72`. Delete.

### Required Rust changes

1. Delete unused `PLACEHOLDER_FOOTER_HINTS` constant.
2. Add `pub const HITL_PLACEHOLDER: &str = "Type your answer...";` for §7 follow-up.
3. **Decision required (red card):** for compaction, keep the footer chip (current Rust) or duplicate text into the placeholder to match TS? Recommendation: **keep footer chip**, do *not* paint compaction text inside the placeholder — Rust's footer treatment is arguably better, just document this.

---

## 3. Blocking state (input lock while agent is busy)

### TypeScript behaviour

`MultiLineInput.tsx` uses `shouldBlockInputForMultiLineInput()` (`multiline-input-compaction-logic.ts`) to gate:

- Backspace / Delete (lines 150–166): `if (shouldBlock) return true;` consumes the key without editing.
- Forward delete `\x1b[3~` (lines 169–177): same pattern.
- Printable character insertion (lines 308–316): same pattern.

`suppressEnter` flag (`AgentView.tsx:5452-5457`) blocks Enter submission when slash palette, file palette, HITL options, or turn-select is active. `MultiLineInput.tsx:141-147` consults it.

Pause / action prompt (`InputTransition.tsx:467-543`) short-circuits the *entire* render — `MultiLineInput` is never mounted, so nothing can be typed.

Loading (running stream) is **not** blocked. The spinner replaces the input view but the buffer underneath stays editable, and the post-load animation reveals whatever was pending. (Simplest implementation: when `is_loading`, also block submission via `suppressEnter` semantics if not already covered — actually TS does *not* do this; verify by reading `InputTransition.tsx:545-566` carefully. The view replacement is sufficient because the user can't see what they're typing.)

### Current Rust state

- **No blocking logic.** `multiline_input::handle_key` (`multiline_input.rs:137-177`) accepts all printable input. `dispatch.rs:206-247` always forwards to `self.input.handle_event(event)`.
- During `Compacting` the user can type and submit — direct race condition with the backend.
- `suppressEnter` semantics: slash popup intercepts Enter via `dispatch.rs:52-94` before the input sees it, so slash-palette parity is OK. Turn-select, HITL: missing entirely.

### Required Rust changes

1. New helper in `views/agent/multiline_input.rs`:
   ```rust
   pub struct InputGate {
       pub block_edits: bool,    // backspace/delete/typing
       pub suppress_enter: bool, // Enter submit
   }
   ```
2. Compute the gate in `agent.rs` per render based on session status + popup state:
   - `block_edits = is_compacting`
   - `suppress_enter = is_compacting || slash_popup_open || file_popup_open || turn_select_active || hitl_options_active`
3. Threaded through `multiline_input::handle_key(key, gate)` — drop edit/insert ops when `block_edits`, swallow Enter when `suppress_enter`. Esc/cursor moves/Shift-arrows still flow.

---

## 4. Esc-key cascade

### TypeScript behaviour (`AgentView.tsx:4731-4773`)

Exactly one branch fires per Esc press, priority order:

1. Close exit-confirmation modal (`showExitConfirmation`).
2. Close turn modal (`showTurnModal`).
3. Close other dialogs (handled elsewhere at `:4577, :4649, :4677`).
4. Disable select mode if `isTurnSelectMode`.
5. Interrupt streaming/compaction: `(displayIsLoading || rustSnapshot.isCompacting) && currentSessionId` → `sessionInterrupt(currentSessionId); refreshRustState();`
6. Clear input text: `inputValue.trim() !== ''` → `setInputValue('')`.
7. Show exit confirmation (or `onExit()` if no session).

`MultiLineInput.tsx:294-296` returns `false` for `key.escape | key.tab | key.pageUp | key.pageDown` so they propagate to AgentView's handler unconsumed.

### Current Rust state

Esc cascade split between `views/agent/dispatch.rs:176-181` (emits `Action::AgentEscPressed` after popup/mode-view) and `app/dispatch_rpc051.rs:37-63` (handles bubbled action):

- No current session → `Action::BackToBoard`.
- Running or Compacting → spawn `backend.interrupt(session)`, stay on AgentView.
- Otherwise → `Action::BackToBoard`.

Compositor handles dialog-level Esc before AgentView sees it (parity for L1).

**Gaps versus TS:**
| TS priority | Rust status |
|---|---|
| L1 close exit-confirmation | ✅ Compositor handles modals |
| L2 close turn modal | N/A — feature absent (RPC-088/turn-select work) |
| L4 disable turn-select | N/A — feature absent |
| L5 interrupt loading/compaction | ✅ `dispatch_rpc051.rs:46-58` |
| L6 clear non-empty input | ❌ **Missing** — Esc on idle non-empty input drops user to Board, discarding text |
| L7 exit confirmation | ❌ **Missing** — straight to `BackToBoard` with no confirmation |

### Required Rust changes

1. Extend `app/dispatch_rpc051.rs::handle_agent_esc()` to check the input buffer **before** falling through to `BackToBoard`:
   ```rust
   if session_running_or_compacting { interrupt; return; }
   if !input.value().trim().is_empty() { input.clear(); return; }
   // L7: emit Action::ShowExitConfirmation (new) — or pop BackToBoard
   ```
2. `MultiLineInput` needs a `clear()` method (or equivalent) — verify it exists; if not, add one (`multiline_input.rs`).
3. Plumb the input buffer (or a snapshot of "is input non-empty") through to the action dispatcher. Simplest: query store/AgentView state for `input_is_nonempty` at action-handle time.
4. **Exit confirmation dialog (L7):** depends on whether a confirmation dialog exists. If not, this card may stop at L6 and split L7 to a separate card. **Recommendation:** scope L7 out unless an existing dialog primitive can be reused (`dialog_theme::render_dialog` from RPC-079 should make this cheap).

---

## 5. Per-behaviour Rust file map (summary)

| TS behaviour | Rust target | Status |
|---|---|---|
| Spinner frames (braille dots, 80 ms) | NEW `views/agent/spinner.rs` | ❌ Implement |
| `"⠋ Thinking... (Esc to stop)"` rendering | NEW `views/agent/input_transition.rs` | ❌ Implement |
| `"⠋ Compacting... (Esc to stop)"` rendering | NEW `views/agent/input_transition.rs` | ❌ Implement |
| Tokens/sec chip (header) | `header_build.rs:138-144`; wire `agent.rs:258` | ⚠️ Wire only |
| Idle placeholder | `agent.rs:75-76`, `multiline_input.rs:236-238` | ✅ |
| HITL placeholder `"Type your answer..."` | NEW constant + HITL branch | ❌ (or split to follow-up) |
| Block edits while compacting | `multiline_input.rs:137-177` via `InputGate` | ❌ Implement |
| `suppressEnter` for palettes/turn-select | Extend `InputGate` | ⚠️ Partial |
| Esc L5 interrupt | `app/dispatch_rpc051.rs:46-58` | ✅ |
| Esc L6 clear non-empty input | `app/dispatch_rpc051.rs` | ❌ Implement |
| Esc L7 exit confirmation | New dialog + dispatcher action | ❌ Decide scope |
| MultiLineInput ignores Esc | `multiline_input.rs:137-177` | ✅ |
| Footer right `cwd [⎇ branch]` | `footer.rs:91-102` | ✅ |
| Footer compaction chip | `footer.rs:104-118` | ✅ (Rust-original) |
| Footer supervisor chip | `footer.rs:82-87` (RPC-061) | ✅ (Rust-original) |
| Pause/HITL input replacement | — | ❌ Out of scope (separate card) |

---

## 6. Acceptance criteria (preliminary — refine in Example Mapping)

1. While a session is `Running`, the AgentView input row replaces the placeholder with a braille-dot spinner + the exact TS message `"Thinking... (Esc to stop | 'Shift+←/↓' sessions | 'Tab' select turn)"`, dim-styled, advancing one frame per 80 ms.
2. While a session is `Compacting`, the input row replaces the placeholder with `"⠋ Compacting... (Esc to stop)"`, advancing the same way.
3. The `SessionHeader` `tok/s` chip appears in magenta when the session is `Running` and `tokens_per_second` is `Some(_)`. (If RPC-086 hasn't wired tokens_per_second, this scenario is gated.)
4. While `Compacting`, typing printable characters, Backspace, Delete, and forward-delete in the input is **swallowed** — the buffer does not change.
5. While `Compacting`, pressing Enter does **not** submit; the input remains.
6. Cursor moves (←/→/↑/↓), Shift-arrows, and Esc still function normally during `Compacting`.
7. Pressing Esc when the session is `Running` or `Compacting` cancels the operation (existing behaviour preserved).
8. Pressing Esc when idle and the input buffer contains non-whitespace clears the buffer; the AgentView stays on the input row (no navigation).
9. Pressing Esc when idle and the input buffer is empty triggers exit confirmation (or, if L7 is descoped, navigates to Board — document the decision).
10. The footer right-side `cwd [⎇ branch]` parity remains unchanged.
11. The footer left-side compaction chip + 10-cell progress bar continues to render unchanged (Rust-original behaviour preserved).

## 7. Out of scope (split to follow-up cards)

- Full HITL freeform-vs-options UI (`Type your answer...` placeholder, options chips). Touch only the placeholder constant; full UI is its own card.
- Pause / action-prompt input replacement (`InputTransition.tsx:467-543`).
- Animation hide/show transition (cosmetic only).
- `validateMultiLineInputCompactionState` warnings (debug-only).

---

## 8. Test plan outline

- **Unit tests in `views/agent/spinner.rs`:** frame indexing, modulus, exact glyph set.
- **Unit tests in `views/agent/input_transition.rs`:** snapshot render of loading/compacting lines (TextBuffer comparison, full glyph & DIM assertions).
- **Unit tests in `multiline_input.rs`:** `InputGate { block_edits: true }` swallows backspace/delete/printable; `suppress_enter: true` swallows Enter; both preserve cursor moves + Esc bubbling.
- **Integration test:** simulate `Running` → render → assert spinner present; flip to `Idle` → assert placeholder present.
- **Integration test (Esc cascade):** populate buffer with text; press Esc; assert buffer cleared and view unchanged.
- **300-LoC source-shape guard:** confirm new modules stay under 300 lines; if `input_transition.rs` grows, split spinner glyph table into the spinner module.

## 9. Risks / open questions

- **R1:** `tokens_per_second_for(sid)` may not exist yet (depends on RPC-086). If absent, scope this scenario down to "is_loading wired but tokens_per_second remains None".
- **R2:** Esc L7 exit confirmation introduces a new dialog. If dialog infrastructure for confirmation is not present, defer L7 to a follow-up card.
- **R3:** The current Rust render-loop tick cadence — confirm it can drive an 80 ms spinner without a dedicated timer. If the loop is event-driven only, need to schedule a `tokio::time::sleep` and emit `Action::SpinnerTick`.
- **Q1:** Should compaction status appear in **both** the input placeholder and the footer chip (TS does placeholder, Rust does footer), or keep the current Rust design? Recommendation: keep footer.
