# Review: RPC-019 — Multi-line input + VirtualList-style scrollback in AgentView

**Date:** 2026-05-15
**Reviewer:** Claude Code (fspec review-skill, fresh review)
**Status:** ✅ PASS (after fixes)

This is an independent, fresh review of RPC-019 performed against the requirements
captured in the work unit's example map (rules + examples + architecture notes)
and the linked feature files. Previous review markdown files were intentionally
ignored — the findings below are formed by re-checking every file against the
RPC-019 contract.

---

## Scope

RPC-019 is a single-card story (no children under `parent: RPC-019` queried; its
own parent is the RPC-002 epic story). The review therefore targets RPC-019
itself across three feature slices:

1. `spec/features/rpc019-multiline-input.feature` — input widget behaviour
2. `spec/features/rpc019-scrollback.feature` — windowed scrollback widget
3. `spec/features/rpc019-source-shape.feature` — file-layout regression

Files reviewed:

- `codelet/fspec-tui/src/views/agent.rs` (orchestrator, 247 lines)
- `codelet/fspec-tui/src/views/agent/multiline_input.rs` (294 lines)
- `codelet/fspec-tui/src/views/agent/scrollback.rs` (275 lines)
- `codelet/fspec-tui/src/views/mod.rs`
- `codelet/fspec-tui/src/components/mod.rs` (Action enum tail)
- `codelet/fspec-tui/tests/view_agent_multiline_input_rpc019.rs`
- `codelet/fspec-tui/tests/view_agent_scrollback_rpc019.rs`
- `codelet/fspec-tui/tests/source_shape_rpc019.rs`
- `codelet/Cargo.toml` (workspace dep block)
- `codelet/fspec-tui/Cargo.toml` (crate dep block)
- `spec/features/rpc019-multiline-input.feature`
- `spec/features/rpc019-scrollback.feature`
- `spec/features/rpc019-source-shape.feature`

---

## 🔴 Critical Issues (Must Fix)

None.

- Build succeeds (`cargo build -p codelet-fspec-tui` passes).
- All 30 RPC-019 tests pass (9 source-shape + 13 multiline-input + 8 scrollback).
- All 26 Gherkin scenarios are coverage-linked at 100 %.
- All three feature files pass `fspec validate`.
- Every file under `views/agent/` and `views/agent.rs` is under 300 lines.
- No forbidden imports (`codelet_core::`, `codelet_napi::`, `tarpc::`,
  `tokio_tungstenite::`) in `views/`.
- The TypeScript reference files (`src/tui/components/MultiLineInput.tsx`,
  `VirtualList.tsx`, `ConversationInputArea.tsx`) are untouched.
- The four new `Action` variants (`HistoryPrev`, `HistoryNext`, `SessionPrev`,
  `SessionNext`) are present in `components/mod.rs` and emitted by
  `AgentView::handle_event`. App::dispatch routing is correctly deferred to
  RPC-021 (as the work unit description and architecture notes require).

---

## 🟡 Warnings (Should Fix)

### W-1 — Test file header referenced a non-existent feature path *(FIXED)*

- **File:** `codelet/fspec-tui/tests/view_agent_multiline_input_rpc019.rs:3`
- **Before:** `//! Feature: spec/features/rpc019-agent-input-and-scrollback.feature`
- **Problem:** That file does NOT exist. The actual feature is
  `spec/features/rpc019-multiline-input.feature`. The CLAUDE.md test-file
  header requirement is "Feature: <real feature path>".
- **Fix applied:** Updated the header doc-comment to reference
  `spec/features/rpc019-multiline-input.feature`.

### W-2 — Scrollback test file header referenced a non-existent feature path *(FIXED)*

- **File:** `codelet/fspec-tui/tests/view_agent_scrollback_rpc019.rs:3`
- **Before:** `//! Feature: spec/features/rpc019-agent-input-and-scrollback.feature`
- **Problem:** Same as W-1 — file does not exist.
- **Fix applied:** Updated the header doc-comment to reference
  `spec/features/rpc019-scrollback.feature`.

---

## 🟢 Observations (Nice to Have, No Action)

### O-1 — Rule [2] has no explicit Gherkin scenario

The work unit's rule `[2]` (Up/Down arrows on the first/last visual line are
forwarded as `Ignored` so a future RPC can layer scrollback / overlay
navigation on top) is implemented in `multiline_input.rs:159–167` and is
exercised indirectly by `input_event_outcome_distinguishes_submitted_continued_ignored`,
but no dedicated scenario asserts the "Up at top → Ignored" / "Down at bottom →
Ignored" boundary case. This is informational — the behaviour is documented in
the feature file's prose doc-string and pinned by the unit test, and the rule
itself describes an internal contract (`Ignored` outcome) rather than a
user-visible behaviour that needs a Gherkin step. **Not in scope for RPC-019
to fix.**

### O-2 — `components/mod.rs:203–211` parenthetical "(RPC-019)"

The doc-comment for the `ReEnableMouseTracking` variant (an RPC-023 / TUI-078
concern) contains an inline "(RPC-019)". This is unrelated to the RPC-019
contract and belongs to the RPC-023 / TUI-078 lineage. **Out of scope —
RPC-019 does not own this comment.**

### O-3 — Empty-Enter is silently dropped in `AgentView::handle_event`

`agent.rs:166–172` filters out an `InputEventOutcome::Submitted("")` and emits
`EventResult::ignored()` instead of `Action::InputSubmitted("")`. This is a
reasonable defensive guard and does NOT contradict rule [0] ("Plain Enter
submits the current input"). No scenario tests it. **Informational.**

---

## Coverage Verification

| Feature file                                              | Scenarios | Covered | Status |
| --------------------------------------------------------- | --------: | ------: | ------ |
| `spec/features/rpc019-multiline-input.feature`            | 12        | 12      | ✅ 100 % |
| `spec/features/rpc019-scrollback.feature`                 | 5         | 5       | ✅ 100 % |
| `spec/features/rpc019-source-shape.feature`               | 9         | 9       | ✅ 100 % |
| **Total**                                                 | **26**    | **26**  | ✅      |

All `@step` comments inside the test files match the Gherkin step text
verbatim. Every scenario has both a `test-file` and an `impl-file` link in the
`.coverage` siblings.

---

## Build & Test Verification

```
cargo build -p codelet-fspec-tui  → Finished `dev` profile [unoptimized + debuginfo]
cargo test  -p codelet-fspec-tui \
    --test view_agent_multiline_input_rpc019 \
    --test view_agent_scrollback_rpc019 \
    --test source_shape_rpc019
  → source_shape_rpc019:           9 passed, 0 failed
  → view_agent_multiline_input:   13 passed, 0 failed
  → view_agent_scrollback:         8 passed, 0 failed
  TOTAL                          → 30 passed, 0 failed
```

---

## Fix Results

| ID  | Issue                                                              | Status   |
| --- | ------------------------------------------------------------------ | -------- |
| W-1 | `view_agent_multiline_input_rpc019.rs` header path wrong           | ✅ Fixed |
| W-2 | `view_agent_scrollback_rpc019.rs` header path wrong                | ✅ Fixed |

## Final Verification

- All RPC-019 tests pass: ✅ 30 / 30
- Build succeeds: ✅
- Coverage complete: ✅ 26 / 26 scenarios
- Feature files valid: ✅ all three pass `fspec validate`
- Tags present: ✅ each feature carries `@RPC-019` plus required
  `@rust @tui` + component / feature-group tags
