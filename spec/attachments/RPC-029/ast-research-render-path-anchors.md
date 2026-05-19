# RPC-029 — AST Research

Purpose: confirm structural anchors in the Rust ratatui AgentView before
refactoring per the canonical TS Ink layout.

## 1. AgentView render path — Block widgets to remove

`AstGrep --pattern 'Block::default().borders($BORDERS)' --path codelet/fspec-tui/src/views/agent.rs`

Matches:
- `codelet/fspec-tui/src/views/agent.rs:253:32 — Block::default().borders(Borders::ALL)` (scrollback wrapper)
- `codelet/fspec-tui/src/views/agent.rs:261:27 — Block::default().borders(Borders::ALL)` (input wrapper)

Both must be deleted in Phase A. No other `Block::default().borders(...)`
call sites exist under `views/agent/`.

## 2. SessionHeader struct location

`AstGrep --pattern "pub struct SessionHeader<$LT> { $$$FIELDS }" --path codelet/fspec-tui/src/views/agent/header.rs`

Matches:
- `codelet/fspec-tui/src/views/agent/header.rs:35:1 — pub struct SessionHeader<'a> { … }`

Only one definition. Phase C extends this struct with `work_unit_id`,
`work_unit_status`, `is_isolated`, `is_debug_enabled`,
`is_select_mode`, `tokens_per_second`, `reasoning_tokens`,
`compaction_reduction`, `is_loading`.

## 3. SessionFooter build_right_text signature

`AstGrep --pattern "fn build_right_text($$$ARGS) -> $RET { $$$BODY }" --path codelet/fspec-tui/src/views/agent/footer.rs`

Matches:
- `codelet/fspec-tui/src/views/agent/footer.rs:57:1 — fn build_right_text(workspace: &WorkspaceInfo) -> String`

Phase A reverts the branch glyph `⌥ → ⎇` here; Phase B splits the return
into two styled spans (dim cwd + cyan branch suffix).

## 4. Existing store accessors for work-unit prefix

`Grep current_work_unit_id|current_work_unit_status` against
`codelet/fspec-tui/src/store/agent_view.rs`:

- L191–192 `pub fn current_work_unit_id(&self) -> Option<&str>`
- L195–196 `pub fn current_work_unit_status(&self) -> Option<&str>`
- L199–202 `pub fn set_current_work_unit(...)`

→ No new store field plumbing needed for Phase C (4). The accessors
already exist.

## 5. Source-shape invariants that must stay green

`Grep PLACEHOLDER_FOOTER_HINTS` across `codelet/fspec-tui/tests/`:

- `tests/source_shape_rpc013.rs:118` — refers to the constant in a doc
  comment; the body of the test (L122–133) asserts `agent.rs`
  *contains* the literal substrings `Enter=send`, `Ctrl+C=interrupt`,
  `ESC=back`.

→ Phase A must KEEP the `PLACEHOLDER_FOOTER_HINTS` constant in
`views/agent.rs` (so the substrings remain present in the source) but
STOP using it from `footer.rs::build_left_hints`. The constant becomes
a vestige preserved purely for the RPC-013 source-shape pin.

## 6. Existing tests that will turn red

`Grep "Enter=send"|"\\[⌥"` across `codelet/fspec-tui/tests/`:

- `tests/view_agent_unit_rpc018.rs:69, 71, 229` — asserts `bottom`
  contains `Enter=send` / `ESC=back` and `row 9` contains `Enter=send`.
- `tests/agent_chrome_parity_rpc018.rs` (likely) — branch glyph assertions
  reference `[⌥ …]`.

→ These three rpc018 unit-test assertions must be revised in Phase D to
match the new footer (no left hints, `⎇` glyph). The corresponding
feature-file scenarios in `spec/features/rpc018-agent-chrome.feature`
must also be updated. The scenarios that test token state from
StreamChunk variants are untouched.
