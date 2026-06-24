# PROV-109 AST Research — Transport + App Dispatch Wiring for Profile Save/Delete

Goal: mirror the established RPC-054 (provider-credentials dispatch) and RPC-347
(custom-model transport) patterns to wire `save_profile` / `delete_profile`
from the TUI down to the PROV-108 backend (already exposed on `FspecService`).

## Transport trait + impls (mirror custom-model write surface)

AstGrep pattern: `async fn delete_custom_model(&self, $$$ARGS) -> Result<()> { $$$BODY }`

- codelet/fspec-tui/src/transport/mod.rs:218 — trait default no-op body (`let _ = (...); Ok(())`)
- codelet/fspec-tui/src/transport/embedded.rs:243 — delegates to `self.client.delete_custom_model(context::current(), ...)`
- codelet/fspec-tui/src/transport/websocket.rs:447 — websocket override

=> Add `save_profile(provider_id, profile_name, ProfileDefinition)` and
`delete_profile(provider_id, profile_name)` in the SAME three places, delegating
to `FspecService::save_profile` / `delete_profile` (rpc/src/lib.rs:218,227).

## Dispatch handlers (mirror RPC-054 provider-settings handlers)

AstGrep pattern: `pub(crate) fn $NAME(&mut self, $$$ARGS) { $$$BODY }` in
codelet/fspec-tui/src/app/dispatch_provider_settings.rs:

- handle_save_provider_credentials:85 — spawn write → on Ok status + list refresh
- handle_delete_provider_credentials:207 — spawn delete → on Ok status + list refresh
- handle_provider_credentials_loaded:69 — folds list AND reloads openai profile slice
  via `profiles_config::load_openai_profiles()` (so a list refresh reloads profiles).
- try_dispatch_provider_settings:242 — routing match arm (catch-all).

=> Add `handle_save_profile`, `handle_delete_profile` + route 3 new Action arms.

## Dispatch routing entry point

AstGrep pattern: `self.try_dispatch_provider_settings($$$ARGS)`
- codelet/fspec-tui/src/app/dispatch.rs:277 — called in the catch-all `_ =>` chain.
  No change needed; new Action arms are handled inside try_dispatch_provider_settings.

## Action enum

codelet/fspec-tui/src/components/mod.rs:106 `enum Action`; existing peers
`SaveProviderCredentials` (665), `DeleteProviderCredentials` (694),
`ConfirmDeleteProviderCredentials` (701), and the RPC-347 custom-model variants
`AddCustomModel`/`EditCustomModel`/`DeleteCustomModel` (636-657).

=> Add `SaveProfile { provider_id, profile_name, definition }`,
`DeleteProfile { provider_id, profile_name }`,
`ConfirmDeleteProfile { provider_id, profile_name }`.

## Test harness

codelet/fspec-tui/tests/provider_settings_dispatch_rpc054.rs is the pattern:
fresh App over `Arc<MockBackend>`, `drain_pending`, `wait_until`, per-call
counters. MockBackend (tests/common/mod.rs) needs save_profile/delete_profile
counters + last-capture + error scripting + trait overrides (mirror
set/delete_provider_credentials at 2425/2454).

## Backend already wired (PROV-108)

- rpc/src/lib.rs:218 `save_profile`, :227 `delete_profile` on FspecService.
- codelet_rpc_types::ProfileDefinition (rpc-types/src/lib.rs:395): base_url, api_key,
  optional context_window/max_output_tokens, flat compaction_threshold_type/value.
