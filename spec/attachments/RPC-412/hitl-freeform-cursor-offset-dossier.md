# RPC-412 — Inline HITL Freeform Cursor Off-By-Header-Rows Dossier

**Type:** Bug (RPC-411 regression / incompleteness)
**Epic:** rust-frontend
**Crate:** `codelet/fspec-tui`

---

## 1. Symptom (user-reported, screenshot-confirmed)

When the user answers a HITL question with **freeform text** — either a pure-freeform
question (no options) or the **"Other..."** sub-mode of an options question — the terminal
**hardware block cursor is painted one (or more) rows too HIGH**: it lands on the inline
HITL prompt **header line**

```
⏸ Demo: Another demo run — pick an option, type your own, or cancel to see what comes back: (Enter Submit | Esc Back to options)
> testing the demo.. see the cursor is above where I am typing            ← text is here, but the cursor block is on the line ABOVE
```

The typed characters render in the correct place on the `> ` composer line; only the
**cursor Y coordinate** is wrong. Column (X) is correct.

## 2. Why ONLY HITL freeform mode is affected

The hardware cursor is only ever drawn in one of these input states, gated by
`is_cursor_visible_with_prompts` (`views/agent/hitl_keys.rs:54-68`):

| State | Cursor shown? | Header above input line? | Buggy? |
|-------|--------------|--------------------------|--------|
| Normal composer | yes | no — `> ` paints at `area.y` | no |
| RPC-406 pause prompt | **no** (gated off) | — | no |
| HITL **options** mode | **no** (gated off) | — | no |
| HITL **freeform / Other** mode | **yes** | **yes** — header (+hint) stacked above `> ` | **YES** |

Freeform mode is the *only* combination that both shows a hardware cursor **and** paints a
header region above the live `> ` input line. That mismatch is the bug.

## 3. Root-cause code trace

### 3.1 Where the cursor is finally set
- `app/events.rs:211` and `app/events.rs:254` — `frame.set_cursor_position((x, y));`
  - `(x, y)` comes from `self.navigator.agent.cursor_position()` (events.rs:210 / :253),
    gated by `is_cursor_visible(...)`.

### 3.2 How `(x, y)` is computed
- `views/agent.rs:151-154` — `AgentView::cursor_position()`:
  ```rust
  pub fn cursor_position(&self) -> Option<(u16, u16)> {
      let area = self.last_input_area?;          // FULL input region, top = header line
      Some(self.input.hardware_cursor_in(area))
  }
  ```
- `views/agent.rs:266` — `self.last_input_area = Some(areas.input);` — this is `split[4]`, the
  **whole** input region whose `.y` is the top of the HITL header, NOT the `> ` line.
- `views/agent/multiline_input_render.rs:66-80` — `hardware_cursor_in(area)`:
  ```rust
  let rel_row = vrow.saturating_sub(self.scroll_top());  // 0 for a one-line answer
  let y = area.y.saturating_add(rel_row);                // = header top + 0  ← WRONG
  ```
  No header offset is ever added.

### 3.3 Where the extra header row(s) come from
- `views/agent/hitl_prompt.rs:205-223` — `render_hitl_prompt`:
  ```rust
  let mut next = paint_header(area, buf, state);   // wrapped header row count (>=1)
  if state.freeform_active() {
      if state.show_empty_hint { paint_line(...); next += 1; }   // optional hint row
      let input_area = Rect { y: area.y.saturating_add(next), .. }; // input pushed DOWN
      input.render_with_prompt(input_area, buf, HITL_PLACEHOLDER);
      return Some(next);                            // ← offset RETURNED
  }
  ```
- `views/agent/input_area.rs:96` — the call site **throws the offset away**:
  ```rust
  let _ = hitl_prompt::render_hitl_prompt(padded, buf, state, &self.input);
  ```

### 3.4 Height is reserved correctly (NOT the bug)
- `input_area.rs:57-59` → `hitl_prompt::prompt_height` (`hitl_prompt.rs:113-119`) reserves
  `header + hint + input_rows`. The region is tall enough. **Only the cursor Y anchor is wrong.**

## 4. Important subtlety: the offset is NOT a constant +1

`next` = `header_rows(state, width)` (`hitl_prompt.rs:106-108`, via `wrap_styled`) **plus** the
empty-hint row when `show_empty_hint` is set. The header wraps at the padded body width, so on a
narrow terminal or a long question the header can occupy **2+ rows**. A fix that hardcodes `+1`
is WRONG. The fix must propagate the **actual** `Some(next)` value that `render_hitl_prompt`
already returns.

## 5. Required fix (design — implement via ACDD)

Capture the freeform header offset during paint and add it to the cursor Y anchor. Suggested seam
(minimal, mirrors how the existing `last_pause` / `last_hitl` snapshots are cached at paint time):

1. Add a field to `AgentView` e.g. `pub(crate) last_hitl_input_offset: Option<u16>` (or fold the
   offset into a richer `last_hitl` payload). Reset to `None` on every paint.
2. In `input_area.rs::paint_input_area`, capture the return of `render_hitl_prompt`:
   ```rust
   let offset = hitl_prompt::render_hitl_prompt(padded, buf, state, &self.input);
   self.last_hitl_input_offset = offset; // Some(next) in freeform, None in options
   ```
   Note: `padded` is inset by the 1-col X pad from `input_area`. The offset is a **row** delta
   (Y), so it is independent of the X pad and can be added directly to `last_input_area.y`.
   Do NOT confuse padded.x vs input_area.x — only Y changes.
3. In `cursor_position()`, when a freeform offset is present, anchor at
   `last_input_area.y + offset` instead of `last_input_area.y`. Keep the existing
   `hardware_cursor_in` X/column math and the viewport clamp (the clamp `max_y` must be
   recomputed against the shifted region OR the offset applied before clamping so the cursor can
   still reach the true input line).

### Feature-parity anchor (TS reference — `src/tui/components/MultiLineInput.tsx`)
The TS composer renders the `> ` input as a normal flex child directly below the prompt header,
so Ink positions the inverse-style cursor *inside* the input line automatically
(`renderLineWithCursor`, MultiLineInput.tsx:326-349). The Rust port drives a raw
`frame.set_cursor_position`, so it must reproduce that "cursor sits on the input line, below the
header" behaviour explicitly. Parity requirement: in freeform mode the block cursor must sit on
the same visual row as the first character the user types, exactly as Ink does in the TS UI.

### tui-textarea reference (`/tmp/tui-textarea`)
`tui-textarea` stores the viewport (`scroll_top` + rect) DURING render (`widget.rs:168`
`self.viewport.store(...)`) and derives the cursor from that stored state on the next tick — i.e.
it treats "where the input was actually painted" as the source of truth for cursor placement. Take
the same principle: the cursor anchor must be derived from **where the input line was actually
painted** (`area.y + next`), not from the top of the enclosing region. Also note tui-textarea's
`next_scroll_top` (`widget.rs:83-92`) keeps the cursor row inside the viewport — our
`scroll_top()`/`sync_viewport` already mirror this; the fix must not break that clamp.

## 6. Files in scope

- `codelet/fspec-tui/src/views/agent.rs` — `cursor_position()` (:151-154), `AgentView` struct
  (:91-130), `last_input_area` set (:266)
- `codelet/fspec-tui/src/views/agent/input_area.rs` — `paint_input_area` (:70-129), discarded
  offset (:96)
- `codelet/fspec-tui/src/views/agent/hitl_prompt.rs` — `render_hitl_prompt` returns `Some(next)`
  (:196-223), `prompt_height` (:113-130)
- `codelet/fspec-tui/src/views/agent/multiline_input_render.rs` — `hardware_cursor_in` (:66-80)
- `codelet/fspec-tui/src/views/agent/hitl_keys.rs` — cursor gate (:54-68) — explains why only
  freeform is affected (do not regress the gate)

## 7. Out of scope
- Options-mode / pause-prompt cursor behaviour (no hardware cursor — unchanged).
- Any HITL UX/wording change (RPC-411 territory).
- The wire protocol / mapping (RPC-410 — done).

## 8. Testing notes (ACDD)
- Test crate: `codelet/fspec-tui/tests/` (integration render tests — see existing
  `inline_hitl_prompt_rpc411.rs` for the render-into-Buffer + cursor-assert pattern).
- Assert the cursor ROW equals the painted `> ` input row (header rows + hint), NOT the region
  top. Cover: (a) single-row header freeform, (b) freeform with `show_empty_hint` set (offset +1),
  (c) "Other..." sub-mode, and ideally (d) a wrapped multi-row header (offset > 1) at a narrow
  width to lock in "not a constant +1".
- Regression guard: options mode and pause mode still report NO hardware cursor
  (`is_cursor_visible_with_prompts` returns false).
- HANG-SAFETY: these are pure synchronous render/geometry tests (no blocked waiters) — no special
  join bounding needed, but follow the crate's existing render-test helpers.
