# Review Findings — PROV-104 (Model view scroll/viewport parity)

**Reviewer:** spawned ACDD reviewer (session 741b5bb9), supervised + adjudicated.

## Functional verdict: PASS
The scroll/viewport bug fix is **genuinely correct, reachable at runtime, and well-tested**.
Reviewer verified: `render_body` no longer steals content rows for inline arrows; the dedicated
scrollbar column does not overwrite content; the live model view uses this render path; the new
`scroll_tests.rs` renders to a real ratatui `TestBackend` and asserts the SELECTED row's `▸ {id}`
glyph is actually painted at top edge, bottom edge, mid-list, after End, and after PageDown, plus an
overflow test asserting all visible rows paint content (none stolen). TS parity (navigate semantics,
page nav, reset-on-filter behavior) confirmed. 100% coverage (6/6); audit 12/12.

## Adjudication of reviewer's 🔴 "C" findings (test-module allow attributes) → DOWNGRADED to convention-conformant (NOT defects)

The reviewer marked the card FAIL solely because the test modules carry
`#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` and use unwrap/expect/panic in
`#[cfg(test)]` code. **Supervisor decision: this is the established project-wide convention, not a
defect.**

Evidence: `grep -rl 'allow(clippy::unwrap_used' codelet --include='*.rs'` → **1011 files**. The
workspace denies these lints in production (`codelet/Cargo.toml:234-237`) and every test module across
the entire codebase opts out via this exact allow-attribute. `scroll_tests.rs` following the same
convention is CONSISTENT and CORRECT. Ripping the allow out of 3–4 files while 1011 others keep it
would make the codebase inconsistent and is not the intent of "fix all issues". Panic-on-index in
test code (`v.rows[idx]`, `markers[0]`) is idiomatic Rust test practice (a panic = a located test
failure). These findings are **resolved as conformant**, not outstanding.

## Legitimate 🟡 applied: reset-on-filter parity clarity
TS resets scroll explicitly (`useModelSelectorState.ts:303 setScrollOffset(0)` on filter, `:288` on
open). Rust filter handlers (`mod.rs:535,546,553`) called `anchor_first_selectable()` but not
`adjust_scroll()`, relying on a render-time side-effect to reset offset. Behaviorally correct (tests
confirm offset→0) but made explicit for parity/clarity by calling `adjust_scroll()` in the filter
handlers.

## 🟡 Recorded (pre-existing, not introduced; no rewrite): 300-line megafiles
`mod.rs` 2124, `rows.rs` 819 over the 300 guideline. PROV-104 net: `rows.rs` shrank −14; tests
isolated in `scroll_tests.rs` (294, under budget). Dedicated refactor cards recommended (see
`file-size-notes.md`). Out of PROV-104 scope (no git safety net for wholesale rewrite of huge files).
