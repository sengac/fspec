# Epic Review: RPC-029 — AgentView structure alignment with TS Ink original

**Date:** 2026-05-19
**Reviewer:** Claude Code (`@spec/skills/review-skill.md`)
**Work Units Reviewed:** 1 (RPC-029, no children)

---

## Summary

- 🔴 Critical: 0
- 🟡 Warnings: 2 (1 DRY violation, 1 stale coverage line-ranges)
- 🟢 Observations: 2 (file-size headroom, pre-existing clippy warnings outside scope)

All issues found in RPC-029's own deliverables were fixed in this review pass. The
work unit returned to `done` after re-validation.

---

## Findings

### 🟡 Warning 1 — DRY violation: `horizontal_pad` and `line_width` duplicated

RPC-029 introduced **two byte-identical helper functions** in both
`codelet/fspec-tui/src/views/agent/header.rs` and
`codelet/fspec-tui/src/views/agent/footer.rs`:

- `fn horizontal_pad(area: Rect, pad: u16) -> Rect` (12 lines, identical)
- `fn line_width(line: &Line<'_>) -> usize` (3 lines, identical)

Both files are siblings under `views/agent/`. The card's architecture note [1]
already established the pattern of sharing 1-row-strip helpers (`paint_row_bg`)
from the parent module — these two helpers were missed.

**Fix applied:** Extracted both helpers to a new module
`codelet/fspec-tui/src/views/agent/chrome.rs` as `pub(crate)` functions
(73 lines including module doc + unit tests). `header.rs` and `footer.rs` now
import them via `use super::chrome::{horizontal_pad, line_width};`. Three new
unit tests in `chrome::tests` exercise the boundary cases (1-col padding, pad
larger than area, multi-span char count).

**Verification:**
- `cargo test --lib views::agent` → 41/41 pass (including 3 new chrome tests)
- `cargo test --test view_agent_unit_rpc029` → 13/13 pass
- `cargo build` → clean

---

### 🟡 Warning 2 — Stale `.feature.coverage` implementation line-ranges

The original coverage file pointed multiple scenarios at line ranges that:

1. **Pointed past EOF.** Six scenarios were linked to `views/agent/header.rs`
   when their target code actually lived in the sibling file
   `views/agent/header_build.rs`. Example: "Header [DEBUG] badge..." was
   mapped to `header.rs:182-190`, but `header.rs` is only 172 lines long —
   those lines never existed in `header.rs`. The DEBUG-badge code lives in
   `header_build.rs:78-83`.
2. **Pointed at unrelated code.** "Scrollback area has no border..." mapped
   to `agent.rs:280-285` which is the **input padding** code, not scrollback.
   Scrollback rendering lives at `agent.rs:272-277`.
3. **Pointed at moved code.** "Header and footer have horizontal padding..."
   mapped to `header.rs:100-113` which used to be the duplicated
   `horizontal_pad` function. After the DRY fix, that function lives at
   `chrome.rs:22-37`.

**Fix applied:** Unlinked and re-linked all 13 scenarios with accurate
file + line ranges:

| Scenario                                                    | New mapping                                     |
| ----------------------------------------------------------- | ----------------------------------------------- |
| Scrollback area has no border…                              | `agent.rs:272-277`                              |
| Input area has no border…                                   | `agent.rs:282-291`                              |
| Footer row appears strictly above…                          | `agent.rs:221-234`                              |
| Header inserts work-unit prefix…                            | `header_build.rs:32-50`                         |
| Header omits work-unit prefix…                              | `header_build.rs:32-50`                         |
| Header and footer rows paint dark grey…                     | `agent.rs:75-85`                                |
| Header and footer have horizontal padding…                  | `chrome.rs:22-37`                               |
| Footer left side is empty…                                  | `footer.rs:54-72`                               |
| Footer branch glyph uses ⎇ U+2387…                          | `footer.rs:80-87`                               |
| Footer cwd span is dark-grey…                               | `footer.rs:76-88`                               |
| Header [DEBUG] badge paints red-bold…                       | `header_build.rs:78-83`                         |
| Header [ISOLATED] badge paints green…                       | `header_build.rs:52-57`                         |
| Header prefix + work unit + model run paints cyan and bold… | `header_build.rs:42-50`                         |

**Verification:**
- `fspec show-coverage rpc029-agent-structure-alignment` → 13/13 (100%)
- `fspec audit-coverage rpc029-agent-structure-alignment` → all 26 file
  references resolve

---

### 🟢 Observation 1 — `views/agent.rs` is at 299 lines

After adding `pub mod chrome;`, the file now sits at 299 / 300 lines. Any
further additions to the orchestrator must extract into a sibling module
(precedent: `header_build.rs`, `mode_view_render.rs`, `chrome.rs`).

### 🟢 Observation 2 — Pre-existing clippy issues outside RPC-029 scope

`cargo clippy -- -D warnings` reports two warnings in
`src/components/dialog_theme.rs` (`explicit_counter_loop`) and one error in
`tests/rpc027_dialog_parity_ij.rs` (`redundant_closure_for_method_calls`).
**These files are NOT in RPC-029's scope** — `dialog_theme.rs` is a dialog
chrome helper (RPC-027 territory) and the failing test belongs to RPC-027.
Flagged here for traceability but explicitly not fixed (would be scope creep).

---

## Files Reviewed

- `spec/features/rpc029-agent-structure-alignment.feature`
- `spec/features/rpc029-agent-structure-alignment.feature.coverage`
- `codelet/fspec-tui/src/views/agent.rs` (298 → 299 lines)
- `codelet/fspec-tui/src/views/agent/header.rs` (188 → 172 lines)
- `codelet/fspec-tui/src/views/agent/header_build.rs` (181 lines, unchanged)
- `codelet/fspec-tui/src/views/agent/footer.rs` (201 → 185 lines)
- `codelet/fspec-tui/src/views/agent/chrome.rs` (73 lines, new)
- `codelet/fspec-tui/tests/view_agent_unit_rpc029.rs` (498 lines, unchanged)

---

## Fix Results

### RPC-029: AgentView structure alignment

- 🟡 DRY violation (horizontal_pad + line_width duplicated) → ✅ Fixed: extracted to `views/agent/chrome.rs` with 3 unit tests
- 🟡 Stale coverage line-ranges (13 scenarios) → ✅ Fixed: re-linked all to accurate file + line spans

### Final Verification

- `cargo test --test view_agent_unit_rpc029`: ✅ 13/13 pass
- `cargo test --lib views::agent`: ✅ 41/41 pass (94/94 across full lib)
- `cargo build`: ✅ clean
- `fspec validate spec/features/rpc029-agent-structure-alignment.feature`: ✅ valid
- `fspec show-coverage rpc029-agent-structure-alignment`: ✅ 13/13 (100%)
- `fspec audit-coverage rpc029-agent-structure-alignment`: ✅ all mappings valid
- Feature file tags: `@done`, `@RPC-029`, `@rust`, `@tui`, `@ui`, `@rpc`, `@agent-view`, `@header`, `@footer`, `@ui-enhancement` — all categorically required tags present

### Status Transition

`done` → `implementing` → `validating` → `done`

---

## Summary Table

| Work Unit | Title                                          | Status   | Issues   |
| --------- | ---------------------------------------------- | -------- | -------- |
| RPC-029   | AgentView structure alignment (TS Ink parity)  | ✅ PASS  | 2 fixed  |
