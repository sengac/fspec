# Review: RPC-047 — `/compact` slash command + compaction progress footer

**Date:** 2026-05-22
**Reviewer:** Claude Code (fspec review skill)
**Scope:** Single leaf work unit (parent: RPC-030, no children).

## Status: ✅ PASS (after one minor fix)

---

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 1 (fixed during review)
- 🟢 Observations: 2 (informational only)

---

## A. Feature File Compliance — ✅ PASS

`spec/features/slash-command-compact.feature`

- Every scenario has correct Given → When → Then ordering.
- Scenario 5 uses an "And" after the initial "Given" to extend preconditions ("And the MockBackend's compact_session returns Ok …"), which is valid Gherkin (continues the precondition block before the `When` step).
- Architecture doc string is present at the top of the feature, documenting wire-shape, slash-command wiring, the CompactionComplete handler location, the AgentViewStore extension, the SessionFooter widget shape, and the single-sourced notice formatter helper.
- `@WORK-UNIT-ID` tag present (`@RPC-047`), plus complete component / feature-group / phase tags.
- No prefill placeholders (`[role]`, `[action]`, `[benefit]`) anywhere.
- `fspec validate spec/features/slash-command-compact.feature` → OK.

## B. Example Map Alignment — ✅ PASS

Every rule (R0–R8) is reflected in at least one scenario:

| Rule | Covered by scenario |
| --- | --- |
| R0 (spawn task → compact_session) | "calls backend.compact_session for the focused session" |
| R1 (success notice format) | "emits a success notice … on Ok" |
| R2 (error notice format) | "emits an error notice … on Err" |
| R3 (silent no-op) | "with no current session is a silent no-op" |
| R4 (store accessors) | covered indirectly via R5 + footer scenarios |
| R5 (CompactionComplete clears + emits) | "CompactionComplete chunk clears progress and emits a completion notice" |
| R6 (footer renders progress) | "SessionFooter renders the compaction progress segment when progress is Some" |
| R7 (footer omits when None) | "SessionFooter omits the compaction segment when progress is None" |
| R8 (spawn + EmitSessionNotice routing) | "only affects the focused session — background sessions are untouched" |

Every example (E0–E7) maps 1:1 to a scenario. No unanswered red-card questions remain on the work unit. Architecture notes align with the actual implementation file locations (see Coverage Verification below).

## C. Test Coverage Compliance — ✅ PASS

Test file: `codelet/fspec-tui/tests/slash_compact_rpc047.rs`

- File header references the feature file (line 3).
- 8 Gherkin scenarios → 8 `#[tokio::test]` / `#[test]` functions.
- Every `@step` comment matches the corresponding Gherkin step text verbatim (verified line-by-line against the feature file).
- Tests assert real observable behaviour (call counts, exact scrollback strings, buffer substrings, glyph presence/absence). No trivial assertions.
- Coverage report (`fspec show-coverage slash-command-compact`) shows **100% (8/8 scenarios)** with both test and impl line ranges linked.
- `cargo test --test slash_compact_rpc047` → 8 passed; 0 failed.

## D. Implementation Quality — ✅ PASS (with 1 fix)

Reviewed files:
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (258 LoC — slash handler + `format_compaction_notice` helper)
- `codelet/fspec-tui/src/app/dispatch_rpc045.rs` (281 LoC — CompactionComplete chunk handler reuses `format_compaction_notice`)
- `codelet/fspec-tui/src/views/agent/footer.rs` (244 LoC — SessionFooter widget + `compaction_bar` helper)
- `codelet/fspec-tui/src/views/agent.rs` (295 LoC — `render_with_store` wires `compaction_progress_for(current_session)` into SessionFooter)
- `codelet/fspec-tui/src/store/agent_view.rs` (299 LoC — store field declaration)
- `codelet/fspec-tui/src/store/agent_view/isolation_state.rs` (126 LoC — get / set / clear accessors)

**SOLID + DRY:**
- ✅ Single-responsibility — slash routing, chunk routing, store accessors, footer rendering, and notice formatting live in separate modules.
- ✅ DRY — `format_compaction_notice(&CompactionResult) -> String` in `dispatch_rpc020.rs` is the **single source of the `[compaction] X.X% reduction …` format string**, imported and reused by `dispatch_rpc045.rs::handle_stream_chunk_state_updates`. Architecture note `[5]` matches the actual code shape.

**No shortcuts:** no `TODO`, `FIXME`, `HACK`, `unimplemented!()`, `todo!()`, or `unwrap()` in production paths for the new code.

**Wired up end-to-end:**
- `SlashCommandSelected(Compact)` → `handle_slash_command` → `tokio::spawn` → `backend.compact_session` → `Action::EmitSessionNotice` → `handle_emit_session_notice` → originating session's scrollback. ✅
- `StreamChunk::CompactionComplete` → `handle_stream_chunk_state_updates` → `clear_compaction_progress` + `Action::EmitSessionNotice`. ✅
- `render_with_store` reads `store.compaction_progress_for(current_session)` and threads it into the `SessionFooter` widget. ✅

**Rust quality:**
- No `unwrap()` / `expect()` / `panic!()` in production source files for the new code paths (only in test helpers, which is allowed by `#![allow(clippy::unwrap_used, …)]`).
- Proper error propagation via `Result<CompactionResult, String>` from `backend.compact_session`.
- `tokio::runtime::Handle::try_current()` guard prevents panics in synchronous unit tests.
- File sizes: all six files ≤ 299 LoC — well under the 300-LoC ceiling and consistent with the RPC-025 source-shape invariant.

**Single-task invariant:** Mutations to `compaction_progress_by_session` go through the `AgentViewStore` accessors, which run on the App task per the project's per-task ownership rule.

## E. Build & Test Verification — ✅ PASS

- `cargo build` in `codelet/fspec-tui/` → clean.
- `cargo build --workspace` in `codelet/` → clean.
- `cargo test --test slash_compact_rpc047` → 8 passed; 0 failed.
- `cargo clippy --tests -- -D warnings` → clean **after one fix**.

## F. Cross-Cutting Concerns — ✅ PASS

- No security concerns (no untrusted input, no secret handling).
- No unbounded loops or missing pagination.
- The compaction notice is emitted via `EmitSessionNotice` so it correctly lands on the originating session even after a focus switch (mirrors the RPC-046 pattern).
- Double-emission consideration: when the backend implementation returns `CompactionComplete` as a `StreamChunk` AND `compact_session` returns `Ok(CompactionResult)`, two `[compaction] …` lines can appear. This is **intentional and documented** in `dispatch_rpc045.rs` (lines 108–115) as parity with the TS Ink original. Not a defect.

---

## 🔴 Critical Issues
None.

## 🟡 Warnings (Fixed)

1. **`footer.rs:44` — clippy `doc_lazy_continuation` warning on the `compaction_progress` field doc comment.**
   - `cargo clippy --tests -- -D warnings` failed with `doc list item without indentation` on the third doc-comment line.
   - **Fix applied during review:** indented the continuation line by two spaces (`/// ` → `///   `) so clippy treats it as a list-item continuation rather than a new paragraph.

## 🟢 Observations (Informational)

1. **`compact_with_no_current_session_is_a_silent_no_op` test** asserts `app.agent_view_store().open_sessions().len() == 0` to prove "no scrollback chunk is appended to any session". This is logically sufficient (no sessions ⇒ no chunks), but the assertion implicitly relies on the lack of sessions rather than directly inspecting scrollbacks. Acceptable — strengthening it would require seeding a background session, which would violate the scenario's `Given an App with NO current session` precondition.
2. **Tag-registry violations across the wider repo** (`fspec validate-tags` shows 311 files with violations project-wide), but `slash-command-compact.feature` itself is fully compliant (`@tui-component`, `@tui`, `@agent-view`, `@rpc`, `@slash-command`, `@multi-session`, `@rust`, `@session-management`, `@RPC-047`, `@done`). Project-wide cleanup is out of scope for this review.

---

## Coverage Verification

- **Feature file:** `spec/features/slash-command-compact.feature` — OK (validates, 8 scenarios, all tags registered).
- **Test file(s):** `codelet/fspec-tui/tests/slash_compact_rpc047.rs` — OK (8 tests, 1:1 with scenarios, every `@step` matches).
- **Impl file(s):**
  - `codelet/fspec-tui/src/app/dispatch_rpc020.rs` (slash handler + `format_compaction_notice`) — OK
  - `codelet/fspec-tui/src/app/dispatch_rpc045.rs` (CompactionComplete arm) — OK
  - `codelet/fspec-tui/src/views/agent/footer.rs` (SessionFooter widget) — OK
  - Plus supporting wiring in `views/agent.rs` and store glue in `store/agent_view.rs` + `store/agent_view/isolation_state.rs`.
- **Scenario coverage:** 8/8 covered (100%).

## Files Reviewed

1. `spec/features/slash-command-compact.feature`
2. `spec/attachments/RPC-047/slash-compact.md`
3. `spec/attachments/RPC-047/ast-research-compaction-wiring.md`
4. `codelet/fspec-tui/tests/slash_compact_rpc047.rs`
5. `codelet/fspec-tui/src/app/dispatch_rpc020.rs`
6. `codelet/fspec-tui/src/app/dispatch_rpc045.rs`
7. `codelet/fspec-tui/src/app/dispatch_rpc046.rs` (`handle_emit_session_notice` — verified routing)
8. `codelet/fspec-tui/src/views/agent.rs` (`render_with_store` wiring)
9. `codelet/fspec-tui/src/views/agent/footer.rs`
10. `codelet/fspec-tui/src/store/agent_view.rs`
11. `codelet/fspec-tui/src/store/agent_view/isolation_state.rs`

---

## Fix Results

### RPC-047: /compact slash command + compaction progress footer

- 🟡 Warning 1 — clippy `doc_lazy_continuation` on `footer.rs:44` → ✅ Fixed (indented continuation line).

## Final Verification

- All RPC-047 tests pass: ✅ (`cargo test --test slash_compact_rpc047` → 8 passed)
- Workspace build succeeds: ✅ (`cargo build --workspace` clean)
- Clippy clean: ✅ (`cargo clippy --tests -- -D warnings` clean)
- Coverage complete: ✅ (8/8 scenarios linked to tests + impl)
- Feature file valid: ✅ (`fspec validate spec/features/slash-command-compact.feature`)
- Tags valid: ✅ (feature carries 10 registered tags including `@RPC-047`)

---

## Verdict

RPC-047 is in a **clean done state** after the single clippy fix applied during this review. All acceptance criteria are testable, tested, and aligned with the actual implementation. The implementation respects the project's source-shape invariants (≤ 300 LoC per file), the single-task mutation rule for `AgentViewStore`, and the single-sourced notice-formatter helper called out in architecture note `[5]`. No scope creep — every change strictly serves a documented rule or example.
