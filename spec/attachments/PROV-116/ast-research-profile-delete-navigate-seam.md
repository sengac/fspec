# PROV-116 — AST research: profile-delete navigate seam

AST queries (AstGrep, language=rust) confirming the exact dispatch seam to wire
and the existing navigate mechanism to reuse. Complements the verified parity
doc `profile-delete-navigate-parity.md`.

## Query 1 — the delete dispatch handler (the gap)

Pattern: `pub(crate) fn handle_delete_profile(&mut self, $$$ARGS) { $$$BODY }`

Hit:
- `codelet/fspec-tui/src/app/dispatch_provider_settings_profiles.rs:119`
  `pub(crate) fn handle_delete_profile(&mut self, provider_id: String, profile_name: String)`

This spawns the backend `delete_profile` task; on `Ok` it sends a status +
`ProviderCredentialsLoaded` reload, on `Err` it only sends a status. It NEVER
calls `set_navigate_target`, so after a successful delete the reload fold's
`apply_pending_navigate()` is a no-op and the cursor is stranded. This is the
seam to wire (set the target on the `Ok` path only, before the reload Action is
emitted — research doc §3 option 1).

## Query 2 — the navigate mechanism to reuse (already exists)

Pattern: `pub fn set_navigate_target(&mut self, $$$ARGS) { $$$BODY }`

Hit:
- `codelet/fspec-tui/src/views/provider_settings/nav_tree_ops.rs:152`
  `pub fn set_navigate_target(&mut self, provider_id: impl Into<String>)`

Sets `pending_navigate_provider`, consumed by `apply_pending_navigate()` (line
161) on the next nav rebuild — already invoked by
`handle_provider_credentials_loaded` (dispatch_provider_settings_profiles.rs:43)
and already used by the OAuth-disconnect dispatch path
(`dispatch_provider_settings_oauth.rs:97,252`). The delete path must call the
SAME method; no parallel mechanism.

## Conclusion (matches the parity doc)

- Wire `self.navigator.provider_settings.set_navigate_target(provider_id)` on the
  delete **success** path only (both `DeleteProfile` / `ConfirmDeleteProfile`
  route through `handle_delete_profile`).
- Err path must NOT set the target (no cursor jump on failure).
- Save path (`handle_save_profile`) stays unchanged — TS only navigates on delete.
