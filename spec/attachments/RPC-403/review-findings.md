# Review: RPC-403 — Bracketed paste never reaches agent input — compositor stub drops multi-line pastes

**Date:** 2026-07-01
**Reviewer:** ACDD compliance review (automated)
**Work unit:** RPC-403 (bug, 3 pts, epic: rust-frontend, status: done)

## Status: WARN

## 🔴 Critical Issues (Must Fix)

None. All 6 scenario tests pass, clippy is clean (zero warnings across all targets), coverage is 100% (6/6 scenarios), the old char-splitting stub is fully removed, and the end-to-end routing chain is correctly wired.

## 🟡 Warnings (Should Fix)

1. **Hardware cursor escapes the input viewport after a large paste** — `AgentView::cursor_position()` (`codelet/fspec-tui/src/views/agent.rs:135-145`) computes `y = input_area.y + logical_cursor_row` with **no clamp to the viewport and no accounting for tui-textarea's internal scroll offset**. After the 10-line paste in scenario 6, `input.cursor()` returns row 9 while the input area is capped at 6 rows (`visible_rows()`, `multiline_input.rs:125-128`), so `frame.set_cursor_position` (`app/events.rs:210-211, 253-254`) is called with a y up to 4+ rows below the input area (past the terminal bottom, since the input is the last layout chunk). This is exactly the risk flagged in `spec/attachments/RPC-403/investigation.md` ("verify cursor stays sane after a 100-line paste … flag as question during Example Mapping if scope creep") — but **the hotspot never became a red-card question in the example map**; it was silently dropped. Pre-existing since RPC-402 (typed Shift+Enter newlines could already exceed 6 rows), materially exacerbated by RPC-403 (multi-line paste is the easy trigger). Recommend a follow-up work unit: clamp the cursor row/col to the input viewport using the textarea's viewport offset.

2. **Paste leaks through Critical modals without paste handlers** — `PauseDialog::handle_event` (`components/pause_dialog.rs:206-244`) returns `ignored()` for `Event::Paste`, so a paste while a Critical pause dialog is on screen falls through `App::handle_paste` into the agent input hidden behind the modal. `HitlDialog` deliberately guards against exactly this ("always consume so a paste can never leak into the agent input hidden behind this Critical modal", `hitl_dialog.rs:292-303`), but the same policy was not applied to `PauseDialog` / other Critical dialogs. It is consistent with the pre-existing key-event fallthrough (unmatched chars also leak), but the two Critical modals now have inconsistent paste policies. Architecture note [3] said "each must handle Event::Paste **or safely ignore**" — for a Critical modal, ignoring is arguably not safe.

3. **DRY: CRLF normalization re-implemented inline in HitlDialog** — `hitl_dialog.rs:300` does `s.replace("\r\n", "\n").replace('\r', "\n")` instead of reusing `multiline_input_paste::normalize_line_endings` (`multiline_input_paste.rs:16-18`). The helper is `pub(super)` scoped to `views::agent`, so it can't be reused from `components/` without a visibility bump — but that's the fix: promote it to a shared `pub(crate)` util and use it in both places (and in role_dialog, see #4).

4. **RoleDialog paste skips lone-`\r` normalization** — `role_dialog.rs:155-158` passes the raw paste string to `textarea.insert_str(s)`. tui-textarea's `insert_str` normalizes `\r\n` (verified per the test file's own comment, `agent_input_paste_routing_rpc403.rs:112-115`), but a **lone `\r`** may enter the role draft as literal text — the very defect rule [3]/[4] eliminates for the agent input. Low likelihood, but inconsistent with the fix's own normalization contract.

5. **`hitl_dialog.rs` remains over the 300-LoC ceiling** — 427 total lines (~331 production before the `#[cfg(test)]` module). It was already over before RPC-403; the paste branch added only ~12 lines, so it did **not get materially worse** — noted per instruction, but it stays on the refactor debt list.

## 🟢 Observations (Nice to Have)

1. **Architecture note [0] vs implementation** — the note says "forward to the **top modal layer's** handle_event"; the implementation (`compositor.rs:197-199`) forwards through the **entire priority chain** via `Compositor::handle_event` until consumed. This is actually better (matches key-event dispatch semantics and satisfies rule [2] naturally) — just a doc/note drift, and the code's own doc comment describes the real behavior accurately.

2. **Scenario 3 is tested at unit level** (`MultiLineInput` direct, test lines 142-154) rather than through `App::handle_paste`. Acceptable — scenarios 1, 2, 5, 6 cover the full routed path; no coverage gap.

3. **Coverage off-by-one** — scenario 1 impl range lists `events.rs:162-178`; `handle_paste` ends at 177 (178 is blank). Cosmetic.

4. **Gating parity confirmed** — paste gate (`multiline_input_paste.rs:29-31`) returns `Continued` when `block_edits`, identical to the typed-edit swallow (`multiline_input.rs:214-216` + `is_edit_keystroke` in `multiline_input_enter.rs:62-67`). The test pins **both** entry paths (`handle_paste` and `handle_event(&Event::Paste)`), which is thorough.

5. **CRLF ordering is correct** — `replace("\r\n","\n")` before `replace('\r',"\n")` cannot double-convert (`\r\n` never becomes `\n\n`).

6. **Perf fine** — two `String::replace` passes + `insert_str` are O(n); no quadratic behavior on large pastes.

7. **300-LoC extraction discipline followed well** — `is_edit_keystroke` moved to `multiline_input_enter.rs` and paste logic to a new `multiline_input_paste.rs`, keeping `multiline_input.rs` at 297 and `dispatch.rs` at 299 (both at, but under, the limit).

8. **`PendingInputChanged` fires correctly on paste** — the `Continued` outcome path in `dispatch.rs:260-267` compares before/after buffer values, so a routed paste emits `Action::PendingInputChanged` exactly when text changed (RPC-052 parity preserved), and `sync_popups()` re-classifies after paste (a pasted leading `/` opens the slash popup — consistent with typed behavior).

9. **Suppressed-paste returns `Consumed`** — during Compacting, the gated paste returns `Continued` → `EventResult::consumed()` even though the buffer is unchanged. Correct choice: the paste should not fall through to Stage-4 app shortcuts, but worth knowing that "consumed" ≠ "inserted" on this path.

## Coverage Verification

- Feature file: `spec/features/agent-input-bracketed-paste-routing.feature` — **OK**
  - Correct Given/When/Then ordering in all 6 scenarios; preconditions are proper `Given`/`And`-after-Given steps (no And-after-Then preconditions).
  - No prefill placeholders; architecture doc string present (mirrors the 4 architecture notes); tags `@done @rust @agent-view @tui @RPC-403` present; capability-based filename.
  - Example map: 6 rules ↔ 6 examples ↔ 6 scenarios, fully traceable; no unanswered questions. (Caveat: the investigation's cursor-viewport hotspot never became a red-card question — see Warning #1.)
- Test file(s): `codelet/fspec-tui/tests/agent_input_paste_routing_rpc403.rs` — **OK**
  - Header references the feature file (line 1); every scenario has a test; all `@step` comments verified as EXACT matches against the Gherkin step text (including quoted strings `"prefix"`, `"pasted"`, `"prefixpasted"`, `"draft"`, `"more text"`).
  - Assertions are behavioral and specific (buffer content, visible-row counts, cursor position, exactly-one-Paste-event delivery, no synthetic key events, modal non-leakage) with diagnostic failure messages. `unwrap`/`panic!` confined to test code under an explicit `#![allow]`.
  - Test run: 6 passed, 0 failed (`/tmp/review403_tests.log`).
- Impl file(s) — **OK** (all coverage line ranges read and verified against source):
  - `codelet/fspec-tui/src/compositor.rs:197-199` — `handle_paste` forwards one real `Event::Paste` through the layer chain; char-splitting stub fully gone (no synthetic-key paste code remains anywhere in `src/`).
  - `codelet/fspec-tui/src/app/events.rs:162-177` — compositor-first, Navigator fallback; deferred-callback handling mirrors `handle_event`; run loop entry at 224-226.
  - `codelet/fspec-tui/src/views/agent/dispatch.rs:243-250` — `Event::Paste` routed through `handle_event_gated` with the same `InputGate` (block_edits/suppress_enter from `last_is_compacting`) as typed keys.
  - `codelet/fspec-tui/src/views/agent/multiline_input.rs:235-246, 125-128, 163-167` — gated event router delegates `Event::Paste` to the paste module; `insert_str` is `pub(super)`; `visible_rows` clamps [1, 6].
  - `codelet/fspec-tui/src/views/agent/multiline_input_paste.rs` (34 LoC, new) — `normalize_line_endings` + gated `handle_paste`.
  - `codelet/fspec-tui/src/views/agent/multiline_input_enter.rs:62-67` — `is_edit_keystroke` relocated here (still referenced from `handle_key_gated`; not dead).
  - `codelet/fspec-tui/src/components/hitl_dialog.rs:292-303` — paste appends to focused free-text row, always consumes under the Critical modal (see Warnings #3, #5).
  - `codelet/fspec-tui/src/components/role_dialog.rs:155-158` — pre-existing paste branch, now reachable via the fixed compositor path (see Warning #4).
  - No `unwrap()`/`expect()`/`panic!()`/`todo!()`/`unimplemented!()` in any production change; no unused imports (clippy clean, `/tmp/review403_clippy.log`).
  - File sizes: compositor.rs 200, app/events.rs 272, dispatch.rs 299, multiline_input.rs 297, multiline_input_paste.rs 34, multiline_input_enter.rs 67, role_dialog.rs 210 — all ≤ 300 except hitl_dialog.rs (427, pre-existing; not materially worsened).
- Scenario coverage: **6/6 scenarios covered** (fspec show-coverage reports 100%; each range independently verified by reading the cited lines).

## Build & Test Verification

- `cargo test -p codelet-fspec-tui --test agent_input_paste_routing_rpc403` → **6 passed / 0 failed** (`/tmp/review403_tests.log`)
- Adjacent modal regression suites: `cargo test -p codelet-fspec-tui --test pause_hitl_rpc053 --test role_dialog_rpc063` → **28 + 9 passed / 0 failed** (`/tmp/review403_modal.log`) — HITL free-text typing/submit and role-dialog save/clear/cancel unaffected.
- `cargo clippy -p codelet-fspec-tui --all-targets` → **clean, zero warnings** (`/tmp/review403_clippy.log`)

## Files Reviewed

- `spec/features/agent-input-bracketed-paste-routing.feature`
- `spec/attachments/RPC-403/investigation.md`
- `codelet/fspec-tui/tests/agent_input_paste_routing_rpc403.rs`
- `codelet/fspec-tui/src/compositor.rs`
- `codelet/fspec-tui/src/app/events.rs`
- `codelet/fspec-tui/src/views/navigator.rs` (routing verification)
- `codelet/fspec-tui/src/views/agent.rs` (cursor_position / render layout, lines 100-280)
- `codelet/fspec-tui/src/views/agent/dispatch.rs`
- `codelet/fspec-tui/src/views/agent/multiline_input.rs`
- `codelet/fspec-tui/src/views/agent/multiline_input_paste.rs`
- `codelet/fspec-tui/src/views/agent/multiline_input_enter.rs`
- `codelet/fspec-tui/src/components/hitl_dialog.rs`
- `codelet/fspec-tui/src/components/role_dialog.rs`
- `codelet/fspec-tui/src/components/pause_dialog.rs` (paste fallthrough audit)
- `codelet/fspec-tui/src/components/create_session_dialog.rs` (selection-only layer audit — safely ignores paste)
- fspec: `show-work-unit RPC-403`, `show-coverage agent-input-bracketed-paste-routing`

## Fix Results (2026-07-01, post-review remediation)

- 🟡 Warning 1 (cursor escapes input viewport after large paste) → ➡️ Deferred by design: follow-up bug RPC-404 created (relates-to RPC-403). Pre-existing geometry issue, out of scope for paste routing.
- 🟡 Warning 2 (paste leaks through Critical modals) → ✅ Fixed: all 9 Critical-priority layers now consume Event::Paste (pause_dialog.rs:243-248, error_dialog, exit_confirmation_dialog, status_dialog, disconnect_dialog, board_exit_confirmation_dialog, help_dialog, notification_dialog; hitl_dialog already consumed). Supplementary test paste_while_pause_dialog_is_open_is_swallowed_and_never_reaches_the_agent_input (tests/agent_input_paste_routing_rpc403.rs:275-322) drives the real App::handle_paste path.
- 🟡 Warning 3 (DRY CRLF normalization) → ✅ Fixed: shared pub(crate) util src/text_normalize.rs (41 LoC, unit-tested); multiline_input_paste.rs and hitl_dialog.rs now use it.
- 🟡 Warning 4 (role_dialog lone-\r) → ✅ Fixed: role_dialog.rs:155-163 normalizes via shared util before insert_str.
- 🟡 Warning 5 (hitl_dialog.rs >300 LoC pre-existing) → ➡️ Accepted as pre-existing debt; +6 lines only, noted on refactor list.
- 🟢 Observation 3 (coverage off-by-one) → ✅ Fixed: events.rs impl range re-linked as 162-177; CRLF scenario re-linked to text_normalize.rs:18-20 + multiline_input_paste.rs:21-31.

## Final Verification
- Full crate: cargo test -p codelet-fspec-tui → 2049 passed / 0 failed (rpc403 suite 7 passed incl. supplementary; pause_hitl_rpc053 28 passed; role_dialog_rpc063 9 passed)
- clippy --all-targets: clean; cargo fmt --check: clean
- Coverage: 6/6 scenarios, audit-coverage all mappings valid
- Feature file valid; tags valid
