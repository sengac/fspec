# RPC-163 AST Research: Delete-key binding parity in API-key edit

**Goal:** Confirm the exact code location of the current `KeyCode::Backspace` arm and verify that no Rust code currently binds `KeyCode::Delete` for the EditApiKey draft buffer. Compare with the TS canonical implementation that binds `key.backspace || key.delete`.

## TypeScript canonical source

`grep "key.backspace || key.delete" src/tui/` — the TS frontend uses an `||`
pattern across every text-input handler. Representative hits (all consistent):

| File | Line | Code |
|------|------|------|
| `src/tui/inputHandlers/apiKeyEditModeHandler.ts` | 46 | `if (key.backspace || key.delete) { ... }` |
| `src/tui/inputHandlers/profileFormModeHandler.ts` | 59 | `if (key.backspace || key.delete) { ... }` |
| `src/tui/inputHandlers/oauthModeHandler.ts` | 52 | `if (key.backspace || key.delete) { ... }` |
| `src/tui/inputHandlers/copilotOauthModeHandler.ts` | 88 | `if (key.backspace || key.delete) { ... }` |
| `src/tui/inputHandlers/customModelFormHandler.ts` | 128 | `if (key.backspace || key.delete) { ... }` |
| `src/tui/inputHandlers/filterModeHandler.ts` | 35 | `if (key.backspace || key.delete) { ... }` |
| `src/tui/components/ModelSelectorScreen.tsx` | 137 | `if (key.backspace || key.delete) { setFilter(filter.slice(0, -1)); return; }` |
| `src/tui/components/MultiLineInput.tsx` | 150 | `if (key.backspace || key.delete) { ... }` |

**Conclusion:** The TS codebase consistently treats `backspace` and `delete` as
sibling triggers for the same pop-last-char behaviour. The Rust port must
match.

## Rust impl — current state

`grep "KeyCode::(Backspace|Delete)" codelet/fspec-tui/src` — five hits:

| File | Line | Code | Relevance |
|------|------|------|-----------|
| `codelet/fspec-tui/src/views/provider_settings/detail.rs` | 139 | `KeyCode::Backspace => {` | **🎯 RPC-163 target** — pops the EditApiKey draft. NO Delete arm. |
| `codelet/fspec-tui/src/views/provider_settings/list.rs` | 116 | `KeyCode::Backspace => {` | List-mode collapse — out of scope. |
| `codelet/fspec-tui/src/components/hitl_dialog.rs` | 289 | `KeyCode::Backspace if self.is_free_text_selected() =>` | Dialog input — out of scope. |
| `codelet/fspec-tui/src/views/agent/multiline_input.rs` | 288 | `KeyCode::Char(_) \| KeyCode::Backspace \| KeyCode::Delete \| KeyCode::Tab` | Already binds both — precedent for the `|` pattern. |
| `codelet/fspec-tui/src/views/agent/search_history_view.rs` | 205 | `KeyCode::Backspace => {` | Search filter — out of scope. |

**Conclusions:**

1. `KeyCode::Delete` is NOT bound anywhere in `views/provider_settings/`.
2. `views/agent/multiline_input.rs:288` proves the merge-arm pattern
   `KeyCode::Backspace | KeyCode::Delete` is already in use elsewhere in the
   codebase, so it is the idiomatic local style.
3. The change is confined to **one file** (`detail.rs`) and **one match arm**
   (currently lines 139-146).

## Surrounding context — detail.rs:139-146

```rust
KeyCode::Backspace => {
    draft.pop();
    view.mode = ProviderSettingsMode::Detail {
        provider_id,
        sub: DetailSub::EditApiKey { draft },
    };
    ProviderSettingsEvent::Consumed
}
```

The arm has **no side-effects** beyond `draft.pop()` (which is a no-op on
empty), the mode re-entry, and the `Consumed` event. Crucially it does NOT
clear `view.status`, which is what allows Rule 5 (status "API key cannot be
empty" must survive a Delete) to hold for free.

## Sub-mode routing — Summary / OAuthNotice

`detail.rs:37-44` dispatches by `DetailSub` variant. The Summary handler
(`handle_summary_key`) and OAuthNotice handler (`handle_oauth_notice_key`)
both already have catch-all `_ =>` / non-Esc fallbacks that preserve state.
Delete will land there as an unrelated key with zero state mutation — Rule 4
is satisfied by the existing code, but explicit regression scenarios are
needed.

## Recommended implementation

Merge into a single arm:

```rust
KeyCode::Backspace | KeyCode::Delete => {
    draft.pop();
    view.mode = ProviderSettingsMode::Detail {
        provider_id,
        sub: DetailSub::EditApiKey { draft },
    };
    ProviderSettingsEvent::Consumed
}
```

Single shared body guarantees Rule 3 (no divergence) at the type-system
level — there is no possible code path where Backspace and Delete behave
differently.

## Estimated impact

- 1 line changed (`KeyCode::Backspace =>` → `KeyCode::Backspace | KeyCode::Delete =>`)
- 0 new functions
- detail.rs grows from 288 → 288 LoC (no net change)
- Risk: minimal; mirrors precedent at multiline_input.rs:288.
