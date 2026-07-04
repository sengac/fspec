# PROV-124 — `/model` selector swallows the first arrow press

## Symptom

When you open the full-screen model selector (`/model`) on a session whose
current model **does not match** any provider's model (or where there is **no
current model** at all), the first arrow-key press (Up / Down / PageUp /
PageDown) does **nothing visible**. You have to press the arrow **twice** before
the cursor moves. Every subsequent press then works normally.

## Root cause — traced end to end (from source, not inference)

The selector keeps **two pieces of state that can disagree**:

| Field | File | Meaning |
|-------|------|---------|
| `selected_index` | `state.rs:16` | the row the **render layer** highlights |
| `has_selection`  | `state.rs:17` | "has the user made an explicit selection yet?" flag |

### On open — `set_providers` (`state.rs:45-80`, RPC-342 / RPC-341 / PROV-101)

1. Every provider starts **collapsed**; only the section containing the current
   model is auto-expanded (`state.rs:48-57`).
2. `rows::index_of_model` looks for the current model's row:
   - **found** → `selected_index = idx; has_selection = true` (`state.rs:73-75`)
   - **not found / no current model** → `has_selection = false`, `selected_index`
     stays `0` (`state.rs:76-78`).
3. Because nothing is expanded in the not-found case, **every row in the list is
   a provider header** — there are **zero selectable model rows**
   (`rows.rs` hides all models for collapsed providers).

⇒ `has_selection == false` ⟺ all providers collapsed ⟺ the list is all headers.

### The render layer ignores `has_selection` (`render.rs:90-98` → `rows_render.rs:111`)

`render_body` highlights whichever row satisfies `abs_i == selected_index`
(`rows_render.rs:111`). So **row 0 (a provider header) is painted with the
cursor band** even though the state machine considers nothing selected. The user
sees a cursor and reasonably expects Down to move it.

### First Down press — `move_down` (`navigation.rs:34-47`)

```rust
pub(crate) fn move_down(&mut self) {
    if !self.has_selection {
        self.anchor_first_selectable();   // <-- and then...
        self.adjust_scroll();
        return;                           // <-- returns WITHOUT a clamped move
    }
    // clamped move only runs when has_selection is already true
    ...
}
```

`anchor_first_selectable()` (`state.rs:95-99`) calls `first_selectable(&rows)`.
In the all-collapsed state there are **no selectable rows**, so
`first_selectable` (`model_selector_dialog_rows.rs:79-81`) returns
`.position(|r| r.selectable).unwrap_or(0)` → **0**. So `selected_index` is set to
0 (already 0), `has_selection` flips to `true`, and the function **returns
early**. **Nothing moves visually.**

### Second Down press

Now `has_selection == true`, so the early return is skipped and
`move_down_clamped(0) = (0+1).min(len-1) = 1` — the cursor finally advances.

⇒ **The first press is silently consumed to flip a flag; only the second press
moves.** The same early-return exists in `move_up`, `page_up`, `page_down`
(`navigation.rs:16-89`), so Up / PageUp / PageDown are all swallowed on the
first press too.

## Why the previous fix (PROV-101) caused this

PROV-101 ("Remove all provider/model/profile selection fallbacks") had a
legitimate goal: pressing **Enter** on a fresh open must **not** silently pick
row 0. To achieve that it introduced `has_selection` and — this is the defect —
wired the flag into **movement** as well as Enter. The first arrow press
therefore "activates" the selection instead of moving it.

- **Correct scope:** `has_selection` should gate **Enter only**.
- **Incorrect scope (the bug):** `has_selection` also gates **movement**.

## The fix — movement moves on the first press (TS parity)

The TypeScript reference (`useModelSelectorState.ts` `navigateUp`/`navigateDown`)
has **no `has_selection` gate on movement**: down is always
`Math.min(currentIdx + 1, len - 1)`, up is always `Math.max(currentIdx - 1, 0)`.

Change `move_up`, `move_down`, `page_up`, `page_down` (`navigation.rs`) so that
an explicit navigation **sets `has_selection = true` and performs the clamped
move on the same press**, instead of early-returning:

```rust
pub(crate) fn move_down(&mut self) {
    // The first explicit navigation activates the selection AND moves on the
    // same press (TS parity). has_selection continues to gate Enter only.
    self.has_selection = true;
    if let Some(next) = move_down_clamped(&self.rows, self.selected_index) {
        self.selected_index = next;
        self.adjust_scroll();
    }
}
```

Apply the analogous change to `move_up`, `page_up`, `page_down`.

### Why this is correct and low-regression

- **Fixes the double-press:** first Down in the all-collapsed state now runs
  `move_down_clamped(0) = 1`, so the cursor visibly moves on press one.
- **Preserves PROV-101's Enter guarantee:** `has_selection` still starts `false`
  on a no-match open, so Enter on a model row remains a no-op *until the user
  navigates*. (On a fresh open every row is a header, and Enter on a header
  toggles expansion — so there is never a "highlighted model row + Enter no-op"
  contradiction.)
- **No render change needed:** keeping the cursor band visible on open is
  consistent because, in the `has_selection == false` state, all rows are
  headers and Enter on a header toggles expansion (a real action). The band is
  not misleading.
- **TS parity:** matches `navigateUp`/`navigateDown` exactly (clamp, no wrap,
  no anchor-to-first-selectable side effect).

### Behavioural delta to be aware of

The only change is that the **first** arrow press now performs a one-row clamped
move rather than anchoring to `first_selectable`. In every reachable state this
produces the same or a more-correct destination:

- All-collapsed open (the bug): press one moves header 0 → header 1. ✅
- Expanded-after-Right-toggle, cursor on header: Down press one moves header →
  first model row (same as the old anchor result). ✅
- Up at the top row: stays at row 0 (correct clamp; old code wrongly moved the
  cursor *down* to first_selectable). ✅

## Affected files

- `codelet/fspec-tui/src/views/model_selector/navigation.rs` — `move_up`,
  `move_down`, `page_up`, `page_down` (remove the `!has_selection` early-return;
  set `has_selection = true` then clamp-move).
- `codelet/fspec-tui/src/views/model_selector/state.rs` — `anchor_first_selectable`
  remains for `Home` / filter-change paths (still valid there).

## Existing specs to reconcile

`spec/features/model-selector-no-auto-select.feature` (PROV-101) — any scenario
that asserts "first arrow press only activates / does not move" must be updated
to the corrected single-press-moves behaviour. The **Enter no-op** scenarios
must remain unchanged and keep passing.

## Test plan (ACDD)

New/updated scenarios in a capability feature file (e.g.
`model-selector-first-press-navigation.feature`), tagged `@PROV-124`:

1. **Open with no current model, first Down moves the cursor one row.**
   Given the selector is opened with no matched current model
   And every provider is collapsed (all rows are headers)
   When I press Down once
   Then the cursor moves from row 0 to row 1
   And `has_active_selection()` is true.

2. **First Up at the top row is a clamped no-move but activates selection.**
   Given the selector is opened with no matched current model
   When I press Up once
   Then the cursor stays on row 0
   And `has_active_selection()` is true.

3. **First PageDown moves by a viewport step on the first press.**

4. **Enter before any navigation is still a no-op on a model row (PROV-101
   regression guard).**

5. **Open on a matched current model still seeds the cursor and Enter selects
   immediately (RPC-341 regression guard).**

All scenarios must have exact `@step` comments and be linked via
`fspec link-coverage`. Tests must FAIL before the navigation.rs change and PASS
after.
