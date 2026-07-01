# RPC-400 Review — ACDD Compliance (review-skill.md)

**Date:** 2026-07-01
**Reviewer:** Claude Code (fspec review skill) via dedicated reviewer worker
**Work Unit:** RPC-400 — Stderr lines in tool cards must render red and strip the stderr sentinel (TS parity)

## Status: PASS

## 🔴 Critical Issues (Must Fix)
None. All checks A–F pass. Feature correctly specified, tested, implemented, and
wired end-to-end. Tests green (7/7), clippy clean, fmt clean, no regressions in
RPC-389/399 collapse or RPC-391/392/393 diff suites. No unwrap/expect/panic/
todo!/unimplemented! in any new/edited production code. All edited files ≤ 299 LoC.

## 🟡 Warnings (Should Fix)
None blocking.

## 🟢 Observations (Nice to Have)
1. **Whole-card `is_error` red not plumbed into the modal path.** The modal
   (`diff_decode::style_modal_lines` → `stderr::style_modal_raw_line`) reddens a
   line only when it *contains* the marker, whereas the scrollback path reddens
   on `is_error OR marker` (`chunk_wrap.rs:167`). Consequence: a failed command's
   *stdout* lines (no marker) are red in the scrollback but normal color in the
   TurnContentModal. Explicitly acknowledged and accepted in architecture note
   [3]; no scenario asserts whole-card-red in the modal (Scenario 6 uses an
   `is_error=false` success card). In-spec, but not byte-for-byte identical to
   the scrollback for the failed-stdout case. Candidate follow-up if full modal
   parity is later desired.
2. **`chunk_processor.rs` and `agent_view.rs` are both at exactly 299 LoC** — one
   line under the 300 ceiling. Any further edit will require refactoring first.
   Flagged proactively.
3. **Header line on failed cards stays normal color.** Rule 2 / Example 3 say a
   failed command renders "the entire body red"; the implementation reddens body
   lines but leaves the header (`● Bash(...)`) in normal style, consistent with
   architecture note [2] ("Prefix/header untouched") and the pre-existing
   whole-card error behavior. The test scopes its assertion to body lines only,
   so no test/spec contradiction. "Entire body" wording is a slight imprecision
   (header excluded by design).

## Coverage Verification
- Feature file: `spec/features/tool-card-stderr-line-coloring.feature` — OK.
  7 scenarios, `@RPC-400` present, architecture doc string present/accurate,
  Given/When/Then ordering correct, no placeholders.
- Test file: `codelet/fspec-tui/tests/tool_card_stderr_coloring_rpc400.rs` — OK.
  Header references the feature file; 7 tests / 7 scenarios; all @step comments
  match the feature step lines VERBATIM; assertions check real behavior
  (span fg == Color::Red, exact stripped text, marker absence, diff bg colors).
- Impl files — OK:
  * `stderr.rs` — `STDERR_MARKER = "⚠stderr⚠"` (line 18) matches
    `codelet/tools/src/bash_output.rs:13`; parity locked by `marker_value_is_exact`
    unit test; modal strips marker BEFORE wrapping.
  * `chunk_processor.rs:201-227` — live path prefixes only when is_stderr=true,
    verbatim otherwise.
  * `chunk_wrap.rs:160-178` — per-line `is_error || contains(marker)` → strip +
    red; diff branch (147-155) bypassed.
  * `diff_decode.rs:81-83` — non-diff branch delegates to stderr helper.
- Scenario coverage: 7/7 (100%), audit-coverage 14/14 files found, all mappings valid.

## End-to-end wiring verified
- `handle_tool_progress` dispatched from `session_context.rs:108` on ToolProgress.
- `wrap_tool_call` is the real scrollback render path via `wrap_source`.
- `style_modal_lines` called by `turn_modal::styled_rows:159` feeding the real modal render.

## Verification results
- `cargo test -p codelet-fspec-tui --test tool_card_stderr_coloring_rpc400`: 7 passed / 0 failed.
- `cargo test -p codelet-fspec-tui` (full): 216 ok blocks, 0 failures/regressions.
- `cargo clippy -p codelet-fspec-tui --all-targets`: clean.
- `cargo fmt -- --check`: clean.

## Fix Results
No critical or blocking issues → no fixes required. Observations 1–3 are all
in-spec (explicitly acknowledged in architecture notes) and are recorded as
optional follow-ups; no code change applied.

## Final Verification
- All tests pass: ✅
- Build/clippy/fmt: ✅
- Coverage complete: ✅ (7/7, audit clean)
- Feature file valid: ✅
- Tags valid: ✅
