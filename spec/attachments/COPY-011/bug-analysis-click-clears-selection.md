# COPY-011 — A click does not clear the active text selection

**Type:** Bug · **Epic:** tui-text-selection-copy · **Depends on:** COPY-010 · **Surfaces:** AgentView scrollback, input composer, turn-content modal, board details strip

---

## 1. Symptom (user-reported)

> "If I click again and I am not selecting something else, it should also clear the selection, like a normal selection system does."

With a text selection already active, a plain click (press + release, no drag) leaves the old selection and its REVERSED highlight on screen. It should be cleared.

## 2. Root cause

1. **Recognizer emits nothing for a quick click.** In `src/mouse/gesture.rs` `on_mouse`:
   - `Down(Left)` → `state = Pressed`, returns empty (`gesture.rs:74–77`).
   - `Up(Left)` → `committing = matches!(state, Selecting)` is **false** (state is `Pressed`), returns empty (`gesture.rs:91–98`).
   - So a quick click yields **no gesture at all** — not even the already-defined `SelectionGesture::Cancel` (which `on_mouse` never emits).
   - Confirmed by the recognizer's own unit test `a_quick_click_produces_no_selection_gesture` (`gesture.rs:176–192`) asserting `up == vec![]`.
2. **No wiring site clears on a bare press/click.** The `Down` dispatch entry points feed the recognizer without first clearing:
   - Scrollback `handle_scrollback_mouse` (`mouse_dispatch.rs:136–141`) → `feed_selection_recognizer`.
   - Composer `handle_composer_mouse` (`mouse_dispatch.rs:152–177`) → `self.input.handle_mouse`.
   - Modal `handle_turn_modal_mouse` (`mouse_dispatch.rs:82–103`) → `feed_turn_modal_selection`.
   - Board `handle_mouse` (`src/views/board/mouse.rs:56–78`) sets `details_press_active=true` and calls `feed_details_selection`; on `Up` it clears `details_press_active` (line 74) but **not** the selection.
   - Because the recognizer returns an empty gesture slice, every `apply_*_gestures` loop iterates over nothing and leaves the prior `Selection` untouched.
3. **Clearing is wired only to other events:** wheel-scroll (`mouse_dispatch.rs:121,128,92,97`; `views/board/mouse.rs:88–103`), `Esc` (`dispatch.rs:137–159`, `dispatch_select.rs:69–87`), and board card-change (`views/board/mouse.rs:88–115`, `details_select.rs:145–154`). None fire for a fresh click over an existing selection in the same surface.

## 3. Why the existing tests didn't catch it

Both "quick click" tests (`agentview_text_selection_copy_copy006.rs:291`, `turn_content_modal_..._copy008.rs:323–354`, `board_details_strip_..._copy009.rs:359–388`) start from a **clean** state with **no** prior selection and only assert a click creates *nothing*. **None first establishes an active selection and then clicks to verify it clears.**

## 4. Required behavior (acceptance criteria)

For **each** of the four surfaces:

1. **Given** an active text selection (with a visible highlight), **when** the user does a quick click (Down then Up, no drag, no long-press) in the same surface, **then** the selection is cleared, the highlight is removed, and **nothing** is written to the clipboard.
2. Starting a **new drag** (Down → Drag → Up) still replaces the old selection with the new one and copies the new text (a click that becomes a drag is "selecting something else").
3. A quick click when there is **no** active selection remains inert — nothing selected, nothing copied (unchanged from today).
4. The clear on a bare click must **not** emit a clipboard write and must **not** interfere with unrelated click behavior (e.g. board grid focus/select on a click **outside** the details strip stays as-is; RPC-102/098 Esc-exit-confirmation is untouched).

## 5. Recommended architecture (recognizer emits `Cancel` on a click)

The cleanest, minimal fix reuses the **existing** `SelectionGesture::Cancel` variant, which all four `apply_*_gestures` loops already handle by clearing the selection:

- Scrollback: `apply_selection_gestures` `Cancel` → `text_selection_active=false` + `Action::SelectionClear` (`mouse_dispatch.rs:211–214`).
- Composer: `apply_gestures` `Cancel` → `self.selection = None` (`multiline_input_select.rs:146–148`).
- Modal: `apply_turn_modal_gestures` `Cancel` → `turn_modal_selection = None` (`turn_modal_select.rs:119`).
- Board: `apply_details_gestures` `Cancel` → clears (`details_select.rs`, verify + also reset `details_press_active`).

**Change:** in `src/mouse/gesture.rs` `on_mouse`, when a left `Up` arrives while `state == Pressed` (i.e. a click with no drag and no elapsed long-press), transition to `Idle` and **emit `SelectionGesture::Cancel`** (instead of the current empty vec). Long-press (`tick` → `Selecting`) and drag (`Drag` → `Selecting`) still reach `Up` in the `Selecting` state and emit `Commit` as before — only the pure `Pressed→Up` click path changes.

Consequences to handle in the SAME card (ACDD — update specs/tests, don't leave them stale):
- Update the recognizer unit test `a_quick_click_produces_no_selection_gesture` to expect `vec![SelectionGesture::Cancel]` (rename/retitle as appropriate, e.g. "a quick click cancels any active selection").
- Verify each surface's `Cancel` arm fully clears BOTH the `Selection` and the highlight spans (scrollback: `selection_clear` empties `selection_highlight_spans`; composer/modal/board: highlight is derived from the selection so clearing the selection suffices — confirm the next render paints no REVERSED cells).
- The board `Up` handler must clear `details_press_active` regardless (already does at `views/board/mouse.rs:74`); ensure the `Cancel` path doesn't leave that latch set.

> Alternative (rejected as default): clear the prior selection on every left `Down` in each wiring site. This works but duplicates the clear across four sites and clears slightly earlier (on press rather than on the completed click); the single recognizer change is DRYer and keeps the gesture semantics in one place. If the worker finds a concrete reason the `Cancel`-on-Up approach breaks a scenario, the Down-clear approach is an acceptable fallback — document the reason.

## 6. Interaction with COPY-010

COPY-010 changes the `Begin`/`BeginLine` anchoring in the same four `apply_*_gestures` handlers and may add a `BeginLine` gesture. Do COPY-011 **after** COPY-010 so both sets of edits land on the post-COPY-010 handler shape. COPY-011 does not change `Begin`/`Extend`; it only adds the `Cancel`-on-click emission and its wiring/tests.

## 7. Files in scope

- `src/mouse/gesture.rs` (+ its unit tests) — the core change.
- Verify (and if needed adjust) the four `Cancel` arms:
  - `src/views/agent/mouse_dispatch.rs` (`apply_selection_gestures`) + scrollback `SelectionClear` reducer.
  - `src/views/agent/multiline_input_select.rs`
  - `src/views/agent/turn_modal_select.rs`
  - `src/views/board/details_select.rs` / `src/views/board/mouse.rs`
- Keep every touched `src/` file **< 300 lines**.

## 8. ACDD / testing notes

- New feature file: `spec/features/click-clears-active-text-selection.feature`, tag `@COPY-011`.
- Write **failing** tests first (red): for each surface — establish an active selection (drag or long-press), then a quick click, then assert (a) selection inactive, (b) highlight span count == 0, (c) clipboard unchanged since the original copy. Plus a regression test that a click-then-drag makes a fresh selection, and that a quick click with no prior selection is still inert.
- Reuse existing harnesses (`copy006` / `copy008` / `copy009` helpers) and their `selection_active` / `highlight_spans` / injected OSC 52 writer seams.
- Every Gherkin step needs a verbatim `// @step` comment. No `unwrap/expect/panic` in production. Full suite green + clippy clean.
