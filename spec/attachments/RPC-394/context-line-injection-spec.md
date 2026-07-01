# RPC-394 — Edit/Write Diffs Miss Surrounding File Context Lines

## Problem Statement

In the Rust fspec-tui agent view, when an **Edit** tool replaces lines in a file, the
rendered diff shows the removed lines (red bar) and added lines (green bar) but **no
surrounding unchanged context lines** from the file. The user expects to see a few
unchanged lines above and below the change (like a normal `git diff` / the TypeScript
"reference version" they have in mind), giving the change spatial context.

See `spec/attachments/RPC-394/problem.png` (the user's screenshot): two red removed lines
and two green added lines with nothing around them.

## Root-Cause Analysis (confirmed)

The current pipeline lives in
`codelet/fspec-tui/src/store/agent_view/diff_format.rs`:

1. `format_edit_diff(old_string, new_string)` runs
   `similar::TextDiff::from_lines(old_string, new_string)` over **only the two
   fragments** the model supplied. It does **not** read the file.
2. `build_diff_rows` collects `changed_indices` (Added/Removed), then
   `indices_to_show` expands each changed index to `[idx-3 ..= idx+3]` to pull in
   context. **But the only lines that exist in the diff are the fragment lines.**
3. When `old_string` and `new_string` share **no** unchanged lines — e.g. a 2-line
   block fully replaced by 2 different lines — `similar` produces **only**
   `Delete` + `Insert` changes and **zero** `Equal` changes. Verified empirically:

   ```
   old = "A\nB"   new = "C\nD"
   → REMOVED "A", REMOVED "B", ADDED "C", ADDED "D"   (no Equal/Context)
   ```

   So `indices_to_show` finds every neighbour is *itself* a changed index →
   **no Context rows are ever materialised** → the screenshot.

### What the TS reference actually does

DeepSearch of the TS reference (`AgentView.tsx` `formatEditDiff` / `formatDiffForDisplay`,
`diff-parser.ts` `computeLineDiff`) confirms the **inline Edit/Write path also diffs only
the fragments** and reads the file solely via `calculateStartLine` to compute the starting
line *number* — it never injects file context either. The visible "context" in TS only
appears when the **model itself** embeds unchanged lines inside `old_string`/`new_string`.

Therefore this is a **product/UX improvement over strict TS parity**, deliberately chosen
by the user: the Rust version should be **better** than the TS reference by reading the
file and injecting real surrounding context, so the diff always shows spatial context
regardless of how tightly the model scoped its `old_string`.

(The separate gitoxide-backed `FileDiffViewer` / changed-files/checkpoint path already has
real file context via unified-diff hunks — that path is NOT in scope here.)

## Desired Behaviour

For an **Edit** (not Write):

1. Read the file content (the file already exists post-edit; `calculate_start_line`
   already reads it).
2. Locate the changed region within the **post-edit file** (the `new_string` span).
3. Take up to `CONTEXT_LINES` (3) real unchanged file lines **before** the change and
   up to `CONTEXT_LINES` real unchanged file lines **after** the change.
4. Build a merged display sequence: `[before-context…] + [diff of old↔new] + [after-context…]`
   with correct 1-based file line numbers, then run the existing windowing / elision /
   collapse logic over it.
5. The before/after context lines render as `Context` rows (gray gutter, white content,
   no background) — exactly the RPC-393 styling.

### Edge cases

- **Edit at the top of the file** (line 1): no before-context (clamp to start).
- **Edit at the bottom of the file**: no after-context (clamp to end).
- **File unreadable / path missing**: fall back to current behaviour (diff fragments
  only, no injected context) — must never panic. Mirrors `calculate_start_line`'s
  graceful `return 1`.
- **Write** (new file): unchanged — the whole content is additions, no context concept.
- **old_string not found in file**: fall back gracefully (no context injection).
- Context windows must not duplicate lines already present inside the fragments
  (avoid double-printing). Simplest robust approach: derive the WHOLE displayed region
  from the post-edit file plus the old fragment, OR clearly bound the injected context to
  lines strictly outside the replaced span.

## Constraints

- Preserve RPC-392 full-width colored bars and RPC-393 structured-row model
  (`DiffDisplayRow`, single `style_row`, codec `to_line`/`parse_line`, uniform elision).
- Keep every touched file **< 300 LoC** (split a helper module if needed).
- No `unwrap()`/`expect()`/`panic!()` in production paths; graceful fallback on IO error.
- ACDD: feature file → failing tests (`@step` comments) → implementation.

## Touch Points

- `codelet/fspec-tui/src/store/agent_view/diff_format.rs` — context-aware row builder;
  extend or complement `format_edit_diff` / `build_diff_rows`. Consider a new
  `build_edit_diff_with_context(old, new, file_path, start_line)` or a dedicated helper
  module to stay under 300 LoC.
- `codelet/fspec-tui/src/store/agent_view/pending_tool_diff.rs` — `PendingDiffKind::Edit`
  must carry the `file_path`; `produce_diff_strings` must thread it through so the builder
  can read the file for context.
- Existing tests: `tests/diff_format_rpc390.rs`, `tests/edit_diff_structured_rows_rpc393.rs`,
  and the new RPC-394 feature's tests.

## Acceptance (high level)

- An Edit replacing 2 fully-different lines inside a larger file shows up to 3 unchanged
  file lines above and 3 below the change as Context rows.
- An Edit at the very top shows no before-context but still shows after-context.
- An Edit at the very bottom shows before-context but no after-context.
- A missing/unreadable file falls back to fragments-only with no panic.
- A Write diff is unchanged (all additions, no injected context).
- All existing RPC-390/391/392/393 diff tests still pass (no styling/collapse regression).
