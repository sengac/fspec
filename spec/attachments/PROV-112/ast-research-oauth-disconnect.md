# PROV-112 AST Research — OAuth disconnect wiring boundary

AST + grep survey of the call targets and integration seams for the
disconnect-oauth foundation slice.

## napi clear targets (parity reference — NOT called directly; fspec-tui forbids napi)
- `napi/src/claude_oauth.rs:256` `pub async fn claude_oauth_clear_tokens() -> Result<()>`
  → internally `tokio::fs::remove_file(claude_auth::get_claude_auth_path())`, NotFound→Ok (idempotent).
- `napi/src/codex_oauth.rs:264` `pub fn codex_oauth_clear_tokens() -> Result<()>`
  → `read_codex_auth()`; `auth.tokens = None`; `write_codex_auth(&auth)` (preserves `OPENAI_API_KEY`).
- `napi/src/copilot_oauth.rs:217` `pub async fn copilot_oauth_clear_credential() -> Result<()>`
  → `copilot::auth::delete_copilot_auth().await` (idempotent).

## providers primitives reachable from codelet-rpc (rpc→providers permitted; rpc→napi forbidden)
- `codelet_providers::claude_auth::get_claude_auth_path() -> PathBuf` (FSPEC_HOME/$HOME/.fspec/credentials/claude_auth.json).
- `codelet_providers::codex::codex_auth::{read_codex_auth, write_codex_auth, CodexAuthJson, CodexTokens}`.
  `CodexAuthJson { OPENAI_API_KEY: Option<String>, tokens: Option<CodexTokens>, last_refresh: Option<String> }`
  → stripping `tokens` preserves the separate `openai_api_key` field.
- `codelet_providers::copilot::auth::{delete_copilot_auth (async), read_copilot_auth (async), get_copilot_auth_path}`.

## fspec-tui integration seams
- `provider_settings/mod.rs:45` `pub enum ProviderSettingsMode { List, Detail{provider_id,sub}, CreateProfile{..}, EditProfile{..} }`
  → add `DisconnectOAuth { provider_id }`. Every `match self.mode` site must gain an arm:
  mod.rs handle_key + render, footer_hints.rs compute_footer_hint, nav_tree_ops.rs delete_target_provider_id.
- `provider_settings/nav_item.rs:35` `NavItemKind::OAuthStatus { label }` (vs `OAuthLogin { method }`).
- `list_actions.rs:89` Enter currently routes BOTH OAuthLogin|OAuthStatus → DetailSub::OAuthNotice; split so
  OAuthStatus → DisconnectOAuth mode, OAuthLogin keeps OAuthNotice (login = PROV-113/114).
- `list_actions.rs:124` `d` currently routes OAuthStatus → open_delete_confirm; reroute OAuthStatus → DisconnectOAuth.
- `components/mod.rs` Action enum (provider region ~597-730): add `OAuthDisconnect { provider_id }`.
- `app/dispatch_provider_settings.rs:221` `try_dispatch_provider_settings` router; catch-all currently → `try_dispatch_profile_write`.
  Add OAuthDisconnect arm → new `dispatch_provider_settings_oauth.rs::handle_oauth_disconnect`.

## backend trait surface
- `transport/mod.rs` `trait FspecBackend` — RPC-054 provider methods have no-op defaults
  (`delete_provider_credentials` etc). Add `oauth_clear_tokens`/`oauth_get_tokens` with no-op/false defaults.
- `transport/embedded.rs:639` `delete_provider_credentials` forwards `self.client.<rpc>(context::current(),..)` — mirror for oauth.
- `transport/websocket.rs:984` overrides exist; for oauth, DO NOT override → inherits no-op default (scenario 9 websocket half).

## RPC service surface
- `rpc/src/lib.rs:57` `#[tarpc::service] trait FspecService` — add `oauth_clear_tokens(provider_id)->Result<(),String>`
  + `oauth_get_tokens(provider_id)->Result<bool,String>`.
- `rpc/src/lib.rs:1565` `delete_provider_credentials` impl pattern (match session_manager). OAuth impl does NOT use
  session_manager — delegates to new `rpc/src/oauth_disconnect.rs` dispatch (providers-direct).

## Reference wiring template
- `app/dispatch_provider_settings_profiles.rs` PROV-109 spawn→backend→ProviderCredentialsLoaded refresh loop.
- `tests/provider_settings_profile_dispatch_prov109.rs` + `tests/common/mod.rs` MockBackend (call counters + scripted errors).
