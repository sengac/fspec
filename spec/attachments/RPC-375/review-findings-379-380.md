# Epic Review: RPC-375 — Markdown Rendering Parity (RPC-379, RPC-380)

**Date:** 2026-06-28
**Reviewer:** Claude Code (fspec review skill) + 2 parallel review subordinates
**Work Units Reviewed:** 2 (RPC-379, RPC-380)

## Summary
- 🔴 Critical: 1 (RPC-379 — proven runtime panic on non-ASCII input)
- 🟡 Warnings: 6 (RPC-379: 3, RPC-380: 3)
- 🟢 Observations: architecture, axum-untouched, file-size, escaping all sound

## Work Unit Results

### RPC-379: Bare-URL/email autolink parity — 🔴 FAIL → fixing
Functionally correct for ASCII, fully covered, passes test/clippy/fmt. BUT contains a proven
crash on multibyte input.

**🔴 C1 — `&text[i..]` panics on non-char-boundary (PROVEN).**
`autolink.rs` `match_scheme` slices `&text[i..]` where `i` is a byte index advanced one byte at
a time by `find_token`/`split_text`. When text before a URL contains a multibyte UTF-8 char,
`i` lands inside a char and the slice panics. Reproduced:
```
input: "café https://example.com"
panic: start byte index 4 is not a char boundary; it is inside 'é' (bytes 3..5)
```
Real-world trigger: any attachment markdown with accented words, em-dashes, emoji, or CJK
before a URL/email — common in docs. Denial-of-render on legitimate content.
**Fix:** iterate char-safe (`char_indices()` / guard with `is_char_boundary` / use
`text.get(i..)` instead of panicking `&text[i..]`).

**🟡 W1 — Spec/test gap for non-ASCII.** No scenario/example covers multibyte content; that gap
is why C1 shipped green. Add a rule + example + test ("URLs preceded by non-ASCII text still
autolink without crashing").

**🟡 W2 — URL run stops only at ASCII whitespace** (`is_ascii_whitespace`), so a Unicode
non-breaking space after a URL pulls trailing bytes into the href. Minor parity divergence.

**🟡 W3 — Email detection narrowing undocumented** (leading-dot local parts, TLD only checked
for "has a dot"). Acceptable narrowing but should be documented.

### RPC-380: GFM footnote-option alignment — ✅ PASS (with WARN) → tightening coverage
Change is correct, minimal, aligned with design + example map; full suite/clippy/fmt clean.
Negative footnote assertion verified meaningful against pulldown 0.12.2 (`<sup
class="footnote-reference">` / `<div class="footnote-definition">`).

**🟡 W1 — Coverage test line ranges stale.** Recorded 11-38 / 41-51 / 54-63; actual fn spans
11-35 / 37-47 / 49-58 (last overshoots EOF=59). Re-link with correct ranges.

**🟡 W2 — Impl line mappings shifted** to include import lines (18-19) rather than only the
Options block (24-29). Re-link to the behavioral lines.

**🟡 W3 — `!contains("<sup")` broader than needed.** The `footnote-reference` /
`footnote-definition` assertions are the precise ones; no change required.

## Fix Plan (ACDD, via implementer subordinate)
1. RPC-379 done → specifying: add rule + example for non-ASCII safety (W1). → testing: add a
   failing multibyte test. → implementing: char-safe scanning fix (C1), terminate run on any
   Unicode whitespace (W2), document email narrowing (W3). → validating → done.
2. RPC-380 done → validating: re-link coverage with correct test+impl line ranges (W1/W2).
   → done.

## Fix Results (verified by supervisor)

### RPC-379
- 🔴 C1 multibyte panic → ✅ Fixed: scanner rewritten to be UTF-8 safe — candidate positions
  now come from `str::char_indices()`, `match_scheme` uses non-panicking `text.get(i..)?`, all
  remaining slices are bounded by char-boundary indices. Reproduced the panic first
  (`café https://example.com`), then made it pass.
- 🟡 W1 spec/test gap → ✅ Fixed: added rule + example + 8th scenario "A URL preceded by
  non-ASCII text is autolinked without panicking" with a matching `@step` test (red→green).
- 🟡 W2 ASCII-only whitespace → ✅ Fixed: URL run now terminates on any `char::is_whitespace()`.
- 🟡 W3 email narrowing → ✅ Fixed: documented in the `autolink.rs` module doc.
- Result: 8/8 scenarios covered; autolink.rs 241 lines; cargo test/clippy/fmt clean.

### RPC-380
- 🟡 W1/W2 coverage line-range drift → ✅ Fixed: all 3 scenarios re-linked with true test fn
  spans (10-35 / 37-47 / 49-59) and the behavioral impl block (render.rs:24-31).
- 🟡 W3 `<sup>` assertion → no change required (precise `footnote-reference` /
  `footnote-definition` assertions retained).
- Result: 3/3 scenarios covered with accurate ranges.

## Final Verification (whole crate)
- `cargo test -p codelet-attachment-viewer`: ✅ 61 tests across 8 suites, 0 failures
- `cargo clippy --all-targets -- -D warnings`: ✅ clean
- `cargo fmt --check`: ✅ clean
- `fspec validate` (both features): ✅ valid
- Coverage: autolink 8/8, footnote 3/3 — ✅ 100% with accurate line ranges
- All files < 300 lines; axum HTTP layer untouched
- RPC-379 and RPC-380: ✅ done
