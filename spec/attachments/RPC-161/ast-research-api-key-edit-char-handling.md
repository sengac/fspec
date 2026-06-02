# AST research: KeyCode::Char handling in API-key edit form

Date: 2026-06-01
Work unit: RPC-161
Tool: AstGrep (rig::tool::Tool) over codelet/fspec-tui/src/views/provider_settings/detail.rs

## Question

> "Where in the Rust ProviderSettingsView::detail dispatcher does the EditApiKey form
> currently consume KeyCode::Char(c), and what is the surrounding contract that
> the RPC-161 ASCII 32-126 filter must wrap?"

## Findings

1. `fn handle_edit_key(view, key, provider_id, mut draft) -> ProviderSettingsEvent`
   lives at `codelet/fspec-tui/src/views/provider_settings/detail.rs:90`
   (verified via AST pattern `fn handle_edit_key($$$ARGS) -> $RET { $$$BODY }`).

2. The relevant `match key.code { ... }` arms (manual read since AstGrep matched
   the function but not the inner arms — Rust's `match` arms with leading
   discriminant-like patterns are not single AST nodes that we could query in
   isolation):

   - `KeyCode::Esc => ...`       (lines 97-104)   → switches mode back to Detail::Summary
   - `KeyCode::Enter => ...`     (lines 105-126)  → either inline validation OR Action::SaveProviderCredentials
   - `KeyCode::Backspace => ...` (lines 127-134)  → `draft.pop()` then re-enter EditApiKey
   - `KeyCode::Char(c) => ...`   (lines 135-146)  → `draft.push(c)` unconditionally, then clear "API key cannot be empty" status if set
   - `_ => ...`                  (lines 147-153)  → re-enter EditApiKey unchanged

3. Cross-reference TS contract:
   - `src/tui/utils/providerSettingsHelpers.ts:39-47`
     - `filterPrintableChars(input: string): string` filters by
       `code >= 32 && code <= 126`.
   - `src/tui/inputHandlers/apiKeyEditModeHandler.ts:51-54`
     - Calls `filterPrintableChars(input)` on every key event input string;
       only pushes the cleaned subset onto `editingApiKey`.
     - Backspace / Enter / Esc bypass the filter (handled earlier in the
       same function, lines 26-49).

4. AST search for any existing printable-ASCII helper in the repo (pattern
   `fn is_printable_ascii($$$ARGS) -> $RET { $$$BODY }`): no matches. The
   helper must be newly introduced by RPC-161.

## Surgical conclusion

RPC-161 modifies exactly one `match` arm — the `KeyCode::Char(c)` arm at
lines 135-146 — by wrapping the `draft.push(c)` + status-clearing logic in
an `if is_printable_ascii(c) { ... }` guard. The `_` fall-through arm
(lines 147-153) already handles "re-enter Detail::EditApiKey { draft }
unchanged" with no Action emitted, so the dropped-char path can either
reuse it (via `else` re-entry) or duplicate the same re-entry inline.

No other call sites or imports change. detail.rs at 233 lines has plenty
of room to grow ~10 lines for the helper + guard.
