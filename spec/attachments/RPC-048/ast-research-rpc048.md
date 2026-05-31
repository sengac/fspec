# AST Research for RPC-048 — `/thinking off|low|med|high` inline-arg parsing

## Goal
Confirm the existing code shape that RPC-048 extends, so the implementation slots into
the existing patterns without churn.

## Target sites (existing code we extend)

### 1. `SlashCommandParse` enum — extend with two new variants

Found at `codelet/fspec-tui/src/app/slash_parser.rs:10`:

```rust
pub enum SlashCommandParse {
    OpenModelDialog,
    OpenThinkingDialog,
    ClearRole,
    SetRole(String),
    NotASlashCommand,
}
```

AST query:
```
pattern: 'pub enum SlashCommandParse { $$$VARIANTS }'
language: rust
path: codelet/fspec-tui/src/app/slash_parser.rs
```

→ RPC-048 adds `SetThinkingLevel(ThinkingLevel)` and `InvalidThinkingLevel(String)`.

### 2. `parse_slash_command` match block — extend with `/thinking <arg>` branch

Lives in the same file. Currently:
- `"/thinking"` (exact) → `OpenThinkingDialog`
- `"/role ..."` uses `strip_prefix("/role ")` — we mirror that for `/thinking `.

### 3. `handle_input_submitted` slash-command match — extend with two new arms

AST query:
```
pattern: 'match parse_slash_command(&text) { $$$ARMS }'
language: rust
path: codelet/fspec-tui/src/app/dispatch_rpc020.rs
```

→ Match at `dispatch_rpc020.rs:196` currently handles `OpenModelDialog`,
   `OpenThinkingDialog`, `ClearRole`, `SetRole`, `NotASlashCommand`. RPC-048 adds:
   - `SetThinkingLevel(level)` → call existing `self.handle_thinking_level_selected(session, level)`.
   - `InvalidThinkingLevel(other)` → push `[error] unknown thinking level: {other}` via `navigator.agent.push_line`.

### 4. `handle_thinking_level_selected` — existing helper we reuse (no changes)

AST query:
```
pattern: 'pub(crate) fn handle_thinking_level_selected($$$ARGS) { $$$BODY }'
language: rust
path: codelet/fspec-tui/src/app/dispatch_rpc022.rs
```

→ Match at `dispatch_rpc022.rs:115`. Already spawns `backend.set_thinking_level`,
   re-fetches via `backend.get_thinking_level`, and dispatches
   `Action::ThinkingLevelLoaded(sid, fresh)`. This is the exact contract the
   acceptance criterion in the attachment requires for the store refresh.

## Out-of-file callers (none required to change)

- `codelet/fspec-tui/src/lib.rs:43` already re-exports
  `parse_slash_command` and `SlashCommandParse` — new variants flow through automatically.
- `codelet/fspec-tui/tests/slash_command_wiring_rpc022.rs` will be extended with the
  new RPC-048 integration scenarios (or a sibling file
  `tests/slash_command_thinking_rpc048.rs` if it grows too large).
- No changes required in `transport/embedded.rs` or `transport/websocket.rs`:
  the `set_thinking_level` FspecBackend trait method already exists (added in
  RPC-022 and widened in RPC-037).

## Source-shape ceiling check

- `slash_parser.rs` is currently 95 lines (well under the 300-LoC ceiling).
- `dispatch_rpc020.rs` is currently 258 lines (under, but tight — added two
  small match arms stay within budget; if it approaches 300 we factor the
  `[error]` push helper into a sibling submodule).

## Conclusion

The whole change is two new enum variants + two new match arms + a small
extension to the `/thinking` parser branch, plus the unit-test table in
`slash_parser.rs` and the integration tests in `tests/`. No new trait methods,
no transport changes, no store changes.
