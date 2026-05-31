# Review: RPC-048 — /thinking off|low|med|high inline-arg parsing

**Date:** 2026-05-22
**Reviewer:** Claude Code (fspec review skill)
**Work Units Reviewed:** 1 (single story, no children)

## Status: PASS (after fixes)

## Summary
- 🔴 Critical: 1 issue (coverage line ranges) → ✅ Fixed
- 🟡 Warnings: 1 issue (over-broad impl mapping) → ✅ Fixed
- 🟢 Observations: 1 (minor gap in scenario coverage for trailing-whitespace bare `/thinking`)

---

## Files Reviewed

- `spec/features/slash-command-thinking-inline.feature`
- `spec/features/slash-command-thinking-inline.feature.coverage`
- `codelet/fspec-tui/src/app/slash_parser.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc020.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc022.rs`
- `codelet/fspec-tui/tests/slash_thinking_rpc048.rs`

---

## 🔴 Critical Issues (Fixed)

### 1. Three dispatch-wiring scenarios had completely wrong test line ranges

The coverage file pointed at lines that did **not contain the relevant tests** — they pointed at the parser tests / helper functions:

| Scenario | Recorded `testLines` | Actual location |
|---|---|---|
| `Submitting "/thinking high" sets the level via the backend and does NOT open the dialog` | `62-110` | `129-184` |
| `Submitting "/thinking gibberish" emits an error notice and does NOT call the backend` | `112-150` | `186-222` |
| `Submitting bare "/thinking" still opens the ThinkingLevelDialog (RPC-022 parity)` | `152-188` | `224-259` |

Likely cause: the integration tests were placed below newly added helpers (`fresh_app`, `drain_pending`, `submit_input`, `session_scrollback_text`) but the original coverage links were never refreshed after the helpers were inserted.

**Fix applied:** Unlinked the three scenarios via `fspec unlink-coverage --all`, then re-linked with correct ranges using `fspec link-coverage`.

---

## 🟡 Warnings (Fixed)

### 2. `Bare /thinking continues to open the ThinkingLevelDialog` had an over-broad impl mapping

Was mapped to `codelet/fspec-tui/src/app/slash_parser.rs:62-75`, which includes the `/thinking <arg>` `strip_prefix` branch (lines 66–84). For bare `/thinking`, only the early-return at lines 63–64 (`if trimmed == "/thinking" { return OpenThinkingDialog; }`) is reached.

**Fix applied:** Re-linked to `slash_parser.rs:63-64`.

---

## 🟢 Observations (Not Fixed — Not in Scope)

### 3. No explicit scenario for `/thinking ` (trailing-whitespace-only) bare command

Rule [2] explicitly says "Bare `/thinking` (no arg, with or without trailing whitespace-only suffix) MUST continue to return `SlashCommandParse::OpenThinkingDialog`". The parser DOES handle this (lines 73–75: `if arg.is_empty() { return OpenThinkingDialog; }`), but there's no dedicated scenario in the feature file or test case exercising the trailing-whitespace path. The inline `mod tests` in `slash_parser.rs` does not cover it either. This is a minor gap, not a defect; flagging as an observation since RPC-048's scope is the inline-arg parsing branches, not exhaustive whitespace coverage.

---

## Coverage Verification (Post-Fix)

- Feature file: `spec/features/slash-command-thinking-inline.feature` — OK (passes `fspec validate`)
- Test file: `codelet/fspec-tui/tests/slash_thinking_rpc048.rs` — OK (all 6 line ranges now point to real tests)
- Impl files: `slash_parser.rs`, `dispatch_rpc020.rs`, `dispatch_rpc022.rs` — OK (line ranges point to the actual branches)
- Scenario coverage: **6 / 6 scenarios covered (100%)**
- `fspec audit-coverage slash-command-thinking-inline` — **All files found (12/12). All mappings valid.**

---

## Per-Section Findings

### A. Feature File Compliance — PASS

- All scenarios have correct Given/When/Then ordering.
- No placeholder text (`[role]`, `[action]`, `[benefit]`) remains — the Background user story is concrete.
- Architecture doc-string is present (lines 11–15) and accurate.
- `@RPC-048` tag is present on the feature.
- `Background: User Story` block is present.

### B. Example Map Alignment — PASS

- All 5 rules are reflected in scenarios:
  - Rule [0] → "parse_slash_command recognises /thinking <level> inline arg" (scenario outline)
  - Rule [1] → "parse_slash_command returns InvalidThinkingLevel for an unknown arg"
  - Rule [2] → "Bare /thinking continues to open the ThinkingLevelDialog" + "Submitting bare /thinking still opens the ThinkingLevelDialog"
  - Rule [3] → "Submitting /thinking high sets the level via the backend and does NOT open the dialog"
  - Rule [4] → "Submitting /thinking gibberish emits an error notice and does NOT call the backend"
- All 11 examples map to scenarios (the scenario outline collapses examples [0]–[5]).
- No unanswered questions (red cards) remain on the work unit.
- Architecture notes match the actual implementation (parser branch in `slash_parser.rs`, dispatch wiring in `dispatch_rpc020.rs::handle_input_submitted`).

### C. Test Coverage Compliance — PASS (after fix)

- Every Gherkin scenario has a corresponding test in `slash_thinking_rpc048.rs`.
- Every test has `@step` comments and the `@step` text matches the feature step text exactly (including the Examples-table substitution comment for the scenario outline).
- Tests verify real behavior (mock backend assertions on `set_thinking_level_calls`, `send_input_calls`, `last_set_thinking_level`, scrollback content, Compositor `contains` / `topmost_priority`).
- Header comment at the top of the test file references the feature: `//! Feature: spec/features/slash-command-thinking-inline.feature`.
- After the fix, all 6 test line ranges in the coverage file point to the actual test functions.

### D. Implementation Quality — PASS

- **SOLID:** `parse_slash_command` is a pure function with a single responsibility (returning the `SlashCommandParse` variant). Dispatch arms in `handle_input_submitted` each route to a single helper.
- **DRY:** The `SetThinkingLevel` arm reuses the existing `handle_thinking_level_selected` (RPC-022 helper) for backend round-trip. No duplicated logic.
- **No shortcuts:** No `TODO`, `FIXME`, `HACK`, `todo!()`, `unimplemented!()`, or `unwrap()` in production code (test `#[allow]` only applies to `cfg(test)`).
- **No half-written code:** All match arms are complete.
- **Wired up end-to-end:** `SetThinkingLevel` and `InvalidThinkingLevel` are dispatched in `handle_input_submitted` (the production entry-point for submitted text). Integration tests prove the wiring works against a `MockBackend`.
- **Error handling:** Async backend round-trip uses `let _ =` for fire-and-forget per the RPC-022 pattern; invalid-arg path surfaces an `[error] unknown thinking level: {other}` scrollback line.
- **File size:** `slash_parser.rs` 138 lines, `dispatch_rpc020.rs` 282 lines, `dispatch_rpc022.rs` 236 lines — all under the 300-LoC ceiling per the RPC-024 source-shape constraint.
- **Idiomatic Rust:** Uses `if let Some(rest) = trimmed.strip_prefix("/thinking ")` and `match arg.as_str()` — no awkward branching.

### E. Build & Test Verification — PASS

- `cargo build` in `codelet/fspec-tui/` succeeds with no warnings.
- `cargo test --test slash_thinking_rpc048` — **6 passed; 0 failed**.
- `cargo test slash_parser` (inline tests) — **1 passed; 0 failed**.
- `fspec validate spec/features/slash-command-thinking-inline.feature` — PASS.
- `fspec audit-coverage slash-command-thinking-inline` — All 12/12 files found, all mappings valid.

### F. Cross-Cutting Concerns — PASS

- Implementation matches architecture notes (parser branch added in `slash_parser.rs`; dispatch wiring added in `dispatch_rpc020.rs`; `dispatch_rpc022.rs` untouched as planned; helper `handle_thinking_level_selected` reused).
- No security concerns: the arg is trimmed + lowercased to a small alphabet, so the formatted scrollback line cannot be hijacked.
- No performance concerns: parser is O(input length); the dispatch arms spawn at most one tokio task per invocation (same as the rest of the slash commands).

---

## Fix Results

### RPC-048: /thinking off|low|med|high inline-arg parsing

- 🔴 Issue 1 (wrong test line ranges for 3 dispatch scenarios) → ✅ Fixed by `unlink-coverage --all` + `link-coverage` with correct ranges (`129-184`, `186-222`, `224-259`).
- 🟡 Issue 2 (over-broad impl mapping for bare `/thinking` parser scenario) → ✅ Fixed by re-linking to `slash_parser.rs:63-64`.

## Final Verification

- All tests pass: ✅ (`cargo test --test slash_thinking_rpc048` — 6/6)
- Build succeeds: ✅ (`cargo build` in `codelet/fspec-tui/`)
- Coverage complete: ✅ (6/6 scenarios, 100%, `audit-coverage` clean)
- Feature file valid: ✅ (`fspec validate`)
- Tags valid: ✅ (all 8 tags on the feature are registered)
- Work unit status: ✅ Returned to `done`

---

## Summary Table

| Work Unit | Title                                          | Status  | Issues  |
|-----------|------------------------------------------------|---------|---------|
| RPC-048   | /thinking off|low|med|high inline-arg parsing  | ✅ PASS | 2 fixed |
