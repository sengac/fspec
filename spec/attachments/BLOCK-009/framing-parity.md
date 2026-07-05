# BLOCK-009 — BlocklistView reference-parity framing / chrome

## Problem

`codelet/fspec-tui/src/views/blocklist/mod.rs::render` hand-rolls its chrome:

```rust
let outer = Block::default().borders(Borders::ALL).title(" Blocklist ");
```

It does **not** use the shared `full_screen_shell` scaffold
(`codelet/fspec-tui/src/views/full_screen_shell.rs`) that the sibling mode-views
(`model_selector`, `changed_files`, `provider_settings`, `resume_session`,
`search_history`) all use. As a result the view diverges both from the TS
reference `src/tui/components/BlocklistListView.tsx` and from the other Rust
views:

1. **No rules-count header.** TS shows `Blocklist Rules (N rules)`. Rust puts
   `" Blocklist "` in the block border with no count.
2. **No vertical divider** between the left list pane and the right details
   pane. TS uses a `borderLeft` separator with `paddingLeft`. `changed_files`
   uses `render_vertical_divider` from `diff_common`.
3. **No `Showing X-Y of N` scroll indicator** (depends on BLOCK-008 scrolling).
4. **Footer wording differs.** TS:
   `↑↓/jk: Navigate | Enter/Space: Toggle Rule | Esc: Close`. Rust:
   ` j/k: navigate  |  Enter/Space: toggle  |  Esc: back `.

## Required changes

1. Replace the hand-rolled `Block`/`Layout` chrome with the shared
   `full_screen_shell` scaffold (use `render_full_screen_scaffold_raw_title` or
   the count-title variant `render_full_screen_scaffold` — the count variant
   naturally yields `Blocklist Rules (N rules)`), matching `changed_files`
   /`model_selector`.
2. Add the rules-count to the title (`Blocklist Rules (N rules)`), parity with
   TS + the count-title scaffold used by the other views.
3. Draw a vertical divider between the two panes using the shared
   `diff_common::render_vertical_divider` (as `changed_files` does), rather than
   a bare 50/50 split with no separator.
4. Add the `Showing X-Y of N` scroll indicator (this depends on the
   `scroll_offset` / `visible_rows` introduced by BLOCK-008 — hence the
   dependency).
5. Update the footer hint text to match the TS reference wording:
   `↑↓/jk: Navigate | Enter/Space: Toggle Rule | Esc: Close`.

## Explicitly OUT of scope (deliberate Rust-side designs — do NOT revert)

- The **category system** (`derive_category`, `[file_path]`/`[bash]` tag, the
  `Category:` detail field, the source tag in the list) — mandated by RPC-056
  rule 8 and covered by existing scenarios. Keep it.
- The **store-backed session-disabled persistence** on
  `AgentViewStore.blocklist_disabled_by_session` — RPC-056 rule 11. Keep it.
- The **event-dispatcher architecture** (`BlocklistEvent::Emit(Action)`) — this
  is the Rust idiom, not a defect.

## Constraints

- File under 300 lines — split into `blocklist/render.rs` etc. if needed.
- Clippy clean, no `unwrap`/`expect`/`todo` in non-test code.
- DRY: reuse `full_screen_shell` + `diff_common::render_vertical_divider` +
  the shared scrollbar/`mode_view_render` helpers rather than duplicating.
- Do NOT regress the RPC-056 scenarios
  (`spec/features/rpc056-blocklist-view-dispatch.feature`): the rendered text
  must still contain the rule ids, `system`/`project` source tags, `file_path`
  /`bash` categories, `○`/`●` glyphs, `(disabled)` suffix, and
  `No blocklist rules configured`.

## Test expectations

- Render of a 2-rule view shows a header containing `Blocklist Rules (2 rules)`.
- Render shows a vertical divider column between the panes.
- Render with an overflowing list shows `Showing X-Y of N`.
- Render footer contains `Enter/Space: Toggle Rule` and `Esc: Close`.
- Existing RPC-056 render assertions still pass (regression guard).
- Every test carries `// @step` comments matching the generated Gherkin.
