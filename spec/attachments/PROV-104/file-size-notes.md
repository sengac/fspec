# PROV-104 — file-size residual overage notes

Per the project <300-line source-shape convention. PROV-104 added the new test
module `scroll_tests.rs` (294 lines — under budget) and minimal production code.

## Final line counts (touched files)

| File | Lines | Status | PROV-104 impact |
|------|------:|--------|-----------------|
| `codelet/fspec-tui/src/views/model_selector/scroll_tests.rs` | 294 | ✅ under 300 | NEW (PROV-104 tests) |
| `codelet/fspec-tui/src/views/model_selector/rows.rs` | 819 | ⚠️ over 300 (pre-existing) | net **−14** (removed inline-arrow row-stealing block) |
| `codelet/fspec-tui/src/views/model_selector/mod.rs` | 2124 | ⚠️ over 300 (pre-existing) | +55 (page_up/page_down + 2 key arms; tests via #[path]) |

## Assessment

`rows.rs` and `mod.rs` were already far over the 300-line budget before PROV-104
(they are large, test-dense view modules). PROV-104 did NOT push either over the
threshold — `rows.rs` actually shrank. No wholesale rewrite was performed (per the
PROV-101 review guidance). New PROV-104 tests were placed in a dedicated
`scroll_tests.rs` (included via `#[cfg(test)] #[path]`) specifically to avoid
growing `mod.rs` further.

## Recommended follow-up refactor cards (not done here)

1. Split `mod.rs` (2124) — extract the `#[cfg(test)] mod tests` block into a
   sibling `*_tests.rs` via `#[path]` (mirrors the new `scroll_tests.rs`), which
   would drop the production portion well under budget.
2. Split `rows.rs` (819) — move the `#[cfg(test)] mod tests` block out similarly.
