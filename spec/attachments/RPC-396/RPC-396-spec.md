# RPC-396 — Scrollable, space-filling help dialog with scrollbar

## Problem
The shared `HelpDialog` (`codelet/fspec-tui/src/components/help_dialog.rs`) is:
- **Fixed shrink-to-content size** — does not use available screen space.
- **Non-scrollable** — if content exceeds body height it is **truncated**
  (`dialog_theme.rs:235-238` breaks the row loop), losing content.
- Handles only `ESC` (`help_dialog.rs:63-74`).

This blocks RPC-397 (accurate, longer per-view help content), which will exceed
one screen on small terminals.

## Goal
Make `HelpDialog`:
1. **Expand** vertically AND horizontally to fill the terminal (minus a small margin).
2. **Scroll** via ↑/↓ (line), PageUp/PageDown (page), Home (top), End (bottom).
3. **Mouse wheel** scroll using the shared `WheelVelocity` acceleration.
4. Render the **shared scrollbar** (`render_list_scrollbar`) in a reserved 1-col gutter
   whenever content overflows.
5. Clamp scroll_offset to `[0, total - visible_rows]`; ESC still dismisses.

> Scope boundary: RPC-396 delivers ONLY the scroll/sizing mechanics. The `HELP_LINES`
> content stays as-is here; correcting/splitting content into board vs agent variants
> is **RPC-397** (which depends on this card).

## Reusable infrastructure (no new shared infra)
| Need | Reuse |
|------|-------|
| scroll-offset math | `components::scroll_viewport::ensure_visible` |
| wheel acceleration | `components::scroll_viewport::{WheelVelocity, WheelDirection}` |
| scrollbar painter | `components::list_scrollbar::render_list_scrollbar` |
| explicit-rect render | `components::dialog_theme::render_dialog_at` (used by TurnContentModal) |
| wheel-in-Component pattern | `components::thinking_level_dialog.rs:167-179` |
| key-scroll reference | `views::agent::slash_command_popup.rs:188-233` |

## Implementation plan
1. Add fields to `HelpDialog`: `scroll_offset: usize`, `visible_rows: usize`, `wheel: WheelVelocity`.
2. `render(&mut self, area, buf)`:
   - Compute a **space-filling rect** (area minus ~2-col / ~2-row margin, clamped).
   - Compute body height → set `self.visible_rows`.
   - Clamp `self.scroll_offset` to `total.saturating_sub(visible_rows)`.
   - Slice `HELP_LINES[scroll_offset .. scroll_offset+visible_rows]` into `DialogRow`s.
   - Call `render_dialog_at(rect, ...)`.
   - If `total > visible_rows`, paint `render_list_scrollbar` in a reserved 1-col gutter
     inside the body.
3. `handle_event`:
   - `Esc` → dismiss (unchanged).
   - `Up/Down` → ±1; `PageUp/PageDown` → ±visible_rows; `Home` → 0; `End` → max.
     Clamp via `ensure_visible` / direct max clamp.
   - `Event::Mouse(m)` `ScrollUp/ScrollDown` → step `wheel`, apply, clamp.
4. Keep `help_dialog.rs` under 300 LoC — extract a `help_dialog_scroll.rs` submodule
   for the rect math / scroll helpers if needed.

## Acceptance Criteria (Gherkin)
Feature: `spec/features/scrollable-help-dialog.feature`
1. 40 lines on a 24-row terminal → first page + scrollbar; PageDown shows next page.
2. Down-to-bottom then End → last line visible, thumb at bottom.
3. Wheel down scrolls down; wheel up past top is a no-op.
4. Large terminal where all content fits → no scrollbar, paging keys are no-ops.

## Business Rules
- R1: Dialog fills available area (minus small margin), not shrink-to-content.
- R2: Overflow scrolls, never truncates.
- R3: ↑/↓ line, PageUp/PageDown page, Home top, End bottom.
- R4: Mouse wheel scrolls with shared WheelVelocity.
- R5: Overflow → proportional scrollbar via `render_list_scrollbar` in a reserved gutter.
- R6: scroll_offset clamped `[0, total-visible_rows]`; ESC dismisses.

## ACDD Workflow
1. Failing tests first (drive `handle_event` scroll + render slice + scrollbar presence).
2. Implement; keep files <300 LoC.
3. `cargo test/clippy/fmt` clean; regenerate the help_dialog snapshot for the new size.
4. Link coverage per scenario.
