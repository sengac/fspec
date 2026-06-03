# AST Research — RPC-154: Detail::Summary `t` keybind removal

## Goal
Prove the `t` / `T` keybind currently exists in `handle_summary_key` and identify the
exact AST nodes / source-byte ranges to remove (or leave silently ignored) so the
Rust ProviderSettingsView matches the TS canonical surface, where no `t` keybind is
bound to TestProviderConnection on any Detail screen.

## Current Rust source location
`codelet/fspec-tui/src/views/provider_settings/detail.rs`

### `fn handle_summary_key`
- Definition site: `detail.rs:46`
- Signature: `fn handle_summary_key(view: &mut ProviderSettingsView, key: KeyEvent, provider_id: String, last_status: Option<DetailStatus>) -> ProviderSettingsEvent`
- The function's body is a `match key.code { … }` block.
- `t` / `T` arm: `detail.rs:58-67`:

```rust
KeyCode::Char('t') | KeyCode::Char('T') => {
    view.mode = ProviderSettingsMode::Detail {
        provider_id: provider_id.clone(),
        sub: DetailSub::Summary {
            last_status: Some(DetailStatus::Testing),
        },
    };
    view.status = "Testing…".to_string();
    ProviderSettingsEvent::Emit(Action::TestProviderConnection(provider_id))
}
```

### Adjacent arms (KEPT — out of scope for RPC-154)
- `Esc`        : `detail.rs:53-57` — closes Summary back to List.
- `r / R`      : `detail.rs:68-77` — emits `Action::RefreshProviderModels` (RPC-154 description names only `t`; `r` stays).
- `Enter`      : `detail.rs:78-89` — opens EditApiKey form.
- catch-all `_`: `detail.rs:90-98` — re-enters Summary, preserving last_status.

## AST-pattern observations
- `ast-grep` (rust language) does NOT match
  `KeyCode::Char('t') | KeyCode::Char('T') => { $$$BODY }` because the alternative-
  pattern in a match arm is a single `match_pattern` AST node that includes the entire
  `or_pattern`, and our pattern syntax can't bind that node when the arm body is a block
  expression — only `fn handle_summary_key(...) -> $RET { $$$BODY }` matches.
- This means our **structural** assertions in test code must fall back to source-string
  inspection: read `detail.rs`, slice from `fn handle_summary_key(` through the next
  top-level `fn ` byte offset, and assert `KeyCode::Char('t')` does not occur in that
  range. The byte-offset slicing pattern is identical to the one used in
  `codelet/fspec-tui/tests/rpc150_inline_test_result_render_shape.rs`.

## TS canonical reference
- Path (read from worktree mirror):
  `.fspec/worktrees/3ce722ec-0b61-4601-813b-023909a2a45a/src/tui/inputHandlers/listModeHandler.ts`
- `grep -n "key\\.t\\|'t'\\|\\.test("` → only one match at line 57, and that match is
  `if (key.tab) {` (tab, NOT `t`).
- No code path binds `t` to `TestProviderConnection`. Confirmed: TS has nothing to mirror.

## Impacted existing tests / specs (require migration after the arm is removed)

1. `codelet/fspec-tui/tests/provider_settings_view_rpc054.rs`
   - Lines 149-171: `Scenario: t inside Detail::Summary emits TestProviderConnection`
     This scenario IS the deviation. RPC-154 must invert it: assert that pressing `t`
     in Summary returns Consumed and emits no Action.

2. `codelet/fspec-tui/tests/provider_settings_api_key_delete_key_rpc163.rs`
   - Lines 287-322: `pressing_delete_in_summary_sub_mode_is_treated_as_unrelated`
     uses `view.handle_key(key(KeyCode::Char('t')))` (line 294) as a *precondition* to
     transition Summary{None} → Summary{Some(Testing)}. After RPC-154 the `t` arm is
     removed, so this transition will no longer happen via the keystroke. The test must
     be rewritten to construct the Summary{Some(Testing)} state directly via
     `view.mode = …` (the field is `pub`), or by introducing a `view.set_status` +
     manual `view.mode` mutation.

3. `spec/features/rpc054-provider-settings-view.feature`
   - The `t inside Detail::Summary emits TestProviderConnection` scenario in this
     feature file is superseded by RPC-154. The scenario should be deleted (or
     converted to a comment pointing at RPC-154's new feature file) once the
     implementation lands.

4. `codelet/fspec-tui/src/views/provider_settings/mod.rs:154`
   - The Summary footer hint string is `"t: test · r: refresh models · Esc: back"`.
     The `t: test ·` prefix is now lying to the user (the key is silently ignored).
     RPC-154 should also drop that prefix from the Summary branch of `footer_hint()`.
     (Strictly an alignment fix, not a key-handler change.)

## Plan summary

- Remove the `KeyCode::Char('t') | KeyCode::Char('T') => { … }` arm from
  `handle_summary_key`. The `_` catch-all already does the correct thing for an
  unrecognized key (re-enter Summary preserving last_status, return Consumed).
- Update the Summary footer hint string in `mod.rs` to drop `t: test ·`.
- Migrate `provider_settings_api_key_delete_key_rpc163.rs` to construct
  Summary{Some(Testing)} directly without pressing `t`.
- Rewrite the `provider_settings_view_rpc054.rs` test that asserts `t` emits
  TestProviderConnection — invert it to assert `t` is silently ignored.
- Delete / invert the matching scenario in
  `spec/features/rpc054-provider-settings-view.feature`.
- New RPC-154 feature file (already written) pins the absence-of-`t` invariant
  going forward, both behaviourally and structurally (source-byte audit).
