# Epic Review: ChangedFilesView Interaction Fixes (RPC-357/358/359)

**Date:** 2026-06-27
**Reviewer:** Claude Code (fspec review skill) — 3 parallel subordinate reviewers
**Work Units Reviewed:** 3

## Summary
- 🔴 Critical: 0 issues
- 🟡 Warnings: 5 issues (mostly shared coverage line-range drift)
- 🟢 Observations: several (cosmetic)
- All three: **PASS**, feature `rust-changed-files-view.feature` 18/18 scenarios 100% covered, full crate lib suite 273 passed / 0 failed, render.rs 183 lines, mod.rs 298 lines, no unwrap/expect/panic in production paths, DRY reuse of shared `list_scrollbar` helper.

## Work Unit Results

### RPC-357 — Mouse-wheel selection reloads diff — PASS
- 🔴 None.
- 🟡 W1: Coverage line ranges off-by-one / stale across the feature (pre-existing drift from earlier cards — e.g. "Pressing F" → tests.rs:247-269 but actual test at 525-546; "Tab moves focus" → mod.rs:263-289 = scroll_focused, not the Tab handler). Re-anchor with `audit-coverage --fix`.
- 🟢 tests.rs:241 `let _ = MouseButton::Left; // keep import used` is a no-op code smell — drop the unused import instead.

### RPC-358 — Pane-aware arrow-key diff scroll — PASS
- 🔴 None.
- 🟡 W1: `diff_focused_view()` helper (where Tab-focus precondition is set) isn't included in the diff-scenario coverage links.
- 🟡 W2: Diff-scenario impl mapping is thin (`mod.rs:183,184` only) — extend to include `scroll_focused`/`apply_diff_scroll` so the clamp logic is traceably covered.
- 🟢 Footer hint + `move_selection` doc comment correctly updated; clean DRY clamp reuse.

### RPC-359 — Scrollbars for both panes — PASS
- 🔴 None.
- 🟡 W1: Coverage line ranges drift slightly (start on doc-comment, stop a few lines short) — re-anchor.
- 🟡 W2: The "fits / no scrollbar" scenario doesn't explicitly assert the gutter is reclaimed (full content width) when there's no overflow — rule [3] (40/60 split preserved) only implicitly verified.
- 🟢 Clean struct-update gutter reservation; robust glyph-presence assertions; production paths free of unwrap/expect/panic.

## Fix Plan (Phase 4)
1. Re-anchor all coverage line ranges for `rust-changed-files-view.feature` via `audit-coverage --fix` (RPC-357 W1, RPC-358 W1/W2, RPC-359 W1).
2. RPC-357: remove the `let _ = MouseButton::Left` no-op / unused import.
3. RPC-359: add an assertion (new or extended test) that, with no overflow, the file-list content occupies the full pane width (gutter reclaimed), pinning rule [3].
