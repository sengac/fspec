# RPC-162 — AST Research: handle_edit_key Enter/Esc exit transitions

## Goal

Audit the current `handle_edit_key` function in
`codelet/fspec-tui/src/views/provider_settings/detail.rs` so we can pin
down the exact AST shapes that must change for RPC-162 (empty-Enter
silent cancel + Esc → List instead of Detail::Summary).

## Pattern 1: function declaration (location)

```
ast-grep --lang rust \
  --pattern 'fn handle_edit_key($$$ARGS) -> $RET { $$$BODY }' \
  codelet/fspec-tui/src/views/provider_settings/detail.rs
```

Match: `codelet/fspec-tui/src/views/provider_settings/detail.rs:102`.

Single hit — `handle_edit_key` is declared exactly once. The function
is `pub(super)`-visible and is dispatched from `handle_detail_key` at
line 41 of the same file.

## Pattern 2: Esc-arm transition target (current behavior)

Searching for `view.mode = ProviderSettingsMode::Detail { ... sub: DetailSub::Summary { last_status: None }, ... }`
inside `handle_edit_key` (lines 109-115) reveals the Esc branch
currently routes to `Detail::Summary` rather than `List`:

```rust
KeyCode::Esc => {
    view.mode = ProviderSettingsMode::Detail {
        provider_id,
        sub: DetailSub::Summary { last_status: None },
    };
    view.status.clear();
    ProviderSettingsEvent::Consumed
}
```

This must become:

```rust
KeyCode::Esc => {
    view.mode = ProviderSettingsMode::List;
    view.status.clear();
    ProviderSettingsEvent::Consumed
}
```

## Pattern 3: Enter-arm empty-draft validation branch (current behavior)

The Enter arm currently contains an empty-draft check that
SETS `view.status = "API key cannot be empty"` and STAYS in
`EditApiKey` (lines 117-125):

```rust
KeyCode::Enter => {
    if draft.is_empty() {
        view.status = "API key cannot be empty".to_string();
        view.mode = ProviderSettingsMode::Detail {
            provider_id,
            sub: DetailSub::EditApiKey { draft },
        };
        return ProviderSettingsEvent::Consumed;
    }
    // ... non-empty save branch follows
}
```

This must become silent cancel — return to `List` mode without
setting any status and without emitting an Action:

```rust
KeyCode::Enter => {
    if draft.is_empty() {
        view.mode = ProviderSettingsMode::List;
        view.status.clear();
        return ProviderSettingsEvent::Consumed;
    }
    // ... non-empty save branch follows
}
```

## Pattern 4: Enter-arm save branch (current non-empty transition)

The non-empty save branch transitions to `Detail::Summary { SavingCredentials }`
(lines 127-137):

```rust
let api_key = draft.clone();
view.mode = ProviderSettingsMode::Detail {
    provider_id: provider_id.clone(),
    sub: DetailSub::Summary {
        last_status: Some(DetailStatus::SavingCredentials),
    },
};
view.status = format!("Saving credentials for {provider_id}…");
ProviderSettingsEvent::Emit(Action::SaveProviderCredentials { provider_id, api_key })
```

After RPC-162 this must transition to `List` directly (no Summary
intermediate):

```rust
let api_key = draft.clone();
view.mode = ProviderSettingsMode::List;
view.status.clear();
ProviderSettingsEvent::Emit(Action::SaveProviderCredentials { provider_id, api_key })
```

## Pattern 5: Char-arm legacy clearing branch

The Char arm (lines 152-169) currently clears `view.status` when the
current status equals the legacy `"API key cannot be empty"` string:

```rust
if view.status == "API key cannot be empty" {
    view.status.clear();
}
```

Under the new flow the legacy string is never written by RPC-162's
Enter branch, so this clearing branch becomes dead code in practice
BUT is intentionally kept as a defensive no-op — any legacy caller
that sets `view.status` manually (e.g. older test fixtures or
external callers) will still see the field cleared on the next
accepted keystroke. Keeping the branch costs ~3 LoC and preserves
backwards compatibility.

## Pattern 6: render_detail Summary arm (NOT changed by RPC-162)

`render_detail` at lines 210-224 still renders the `Detail::Summary`
arm. Since the Summary variant is preserved on the enum (only the
EditApiKey EXIT transitions change), the render arm remains untouched.

```ast-grep
fn render_detail($$$ARGS) { $$$BODY }
```

## Pattern 7: footer_hint match arms (NOT changed)

`footer_hint` at `mod.rs:130-139` still returns a hint for
`DetailSub::Summary`. Since callers can no longer reach Summary
via EditApiKey exit, that arm becomes unused in the RPC-103
flow but remains valid for the legacy `visible_providers` fallback
inside `list::handle_list_key:101-104`. No change required by
RPC-162.

## Patterns NOT in scope

* `DetailSub::Summary` enum variant — retained for legacy callers.
* `DetailStatus` enum — retained for legacy callers.
* `status_text.rs::DetailStatus::to_span` — retained.
* `dispatch_rpc054.rs` action handlers (TestProviderConnection,
  RefreshProviderModels) — retained for follow-up cards.

## Test files that need updating

```
ast-grep --lang rust --pattern 'DetailSub::Summary { $$$_ }' \
  codelet/fspec-tui/tests/
```

Hits:
* `provider_settings_view_rpc054.rs:111` — scenario "Enter on an
  api_key row transitions to Detail::Summary" — DELETE (RPC-103
  superseded this with NavItem ApiKey → EditApiKey direct route)
* `provider_settings_view_rpc054.rs:164,188` — `t` and `r` keybind
  scenarios — retained (still test Summary's internal keybinds via
  the legacy entry path)
* `provider_settings_view_rpc054.rs:285` — "Enter on EditApiKey
  with non-empty draft" — UPDATE expected mode from
  `Detail::Summary { SavingCredentials }` to `List`
* `provider_settings_view_rpc054.rs:448` — "Esc in Detail::EditApiKey
  returns to Detail::Summary" — UPDATE scenario name + assertion
  to return to `List`

Feature file `spec/features/rpc054-provider-settings-view.feature`
needs matching scenario rewrites at lines 135-149 (Enter cases) and
207-212 (Esc case).

## Conclusion

The RPC-162 implementation is a surgical 6-line edit inside
`handle_edit_key` (Esc → List, Enter empty → List, Enter non-empty
→ List). All other identifiers, enums, dispatch handlers, and
render paths remain intact. The legacy `DetailSub::Summary` variant
is preserved to keep the source-shape test (`source_shape_rpc054.rs`)
green and to avoid cascading test rewrites; deletion of Summary is
deferred to a follow-up card after RPC-104/RPC-105/RPC-106 finish
porting the inline rendering pieces.
