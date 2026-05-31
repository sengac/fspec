# RPC-054 — AST research: provider-credentials RPC surface

## Goal
Plan how to widen the existing `SessionManagerHandle` + `FspecService` +
`FspecBackend` cross-transport surface with the 6 new credential-management
methods, and surface them through a new Rust ratatui `ProviderSettingsView`.

## Findings

### 1. Existing provider-credentials helpers (codelet-providers)
- `codelet_providers::credentials::ProviderCredentials::detect()` already
  scans env vars and auth files to determine whether each built-in
  provider has usable credentials. Returns booleans per provider plus a
  `custom_available: HashMap<String, bool>` for discovered custom
  providers.
- `codelet_providers::custom::list_providers_info()` returns
  `Vec<ProviderInfo>` for ALL providers (built-ins + discovered custom),
  with per-provider `available`, `is_custom`, `models: Vec<ProviderModelInfo>`.
- `codelet_providers::custom::test_provider_connection(name)` performs an
  HTTP `GET <base_url>/models` round-trip and returns
  `ProviderTestResult { reachable, status_code, matched_models }`.
- `codelet_providers::ProviderManager::with_provider(name)` is a
  lightweight credential-validation constructor (no network) — used by
  the existing CONFIG-004 NAPI `test_provider_connection` shim at
  `codelet/napi/src/session_bindings.rs:2848`.

### 2. Existing trait/service patterns (mature, ready to follow)
- `codelet_core::session_manager_handle::SessionManagerHandle` is a
  default-method-rich trait. Each per-session method takes
  `&SessionId` and returns owned wire-portable values. The
  `StubSessionManagerHandle` mirrors every method with deterministic
  state + atomic per-call counters (see RPC-049 `resume_session_calls`
  and RPC-050 `set_work_unit_context_calls` for the pattern).
- `codelet_rpc::FspecService` is a `#[tarpc::service]` trait. Each
  method takes a `tarpc::context::Context` (injected by the macro)
  plus the same arguments as the handle method. The corresponding
  `FspecServiceImpl` impl routes via `match self.inner.session_manager()
  { Some(handle) => handle.<method>(...), None => <sensible default> }`.
- `codelet_fspec_tui::transport::FspecBackend` exposes one async fn
  per RPC. Both `EmbeddedFspecBackend` and `WebSocketFspecBackend`
  forward as `self.client.<rpc>(context::current(), ...).await`.
  The WebSocket impl additionally consults `self.client.read().await`
  and returns `BackendError::Disconnected` if the client slot is empty.

### 3. New wire types to add (codelet/rpc-types/src/lib.rs)
- `ProviderCredentialInfo { provider_id, display_name, configured,
  credential_type, model_count }` — plain struct, napi(object) gated.
- `ProviderCredentialInput` — napi doesn't support tagged enums via
  `napi(object)`, so encode as a struct with `kind: String` discriminant
  + optional fields per variant (`api_key`, `oauth_token`,
  `oauth_refresh_token`, `custom_endpoint`, `custom_api_key`). Keep the
  Rust API ergonomic via convenience constructors.
- `TestConnectionResult { success, error: Option<String>, latency_ms }`
  — plain struct, napi(object) gated.

### 4. Slash-command + view wiring (codelet/fspec-tui)
- `SlashCommandAction::Provider` currently falls into the "notice"
  default arm in `app/dispatch_rpc020.rs::handle_slash_command`. Add a
  new branch that dispatches `Action::OpenProviderSettingsView`.
- Navigator (`views/navigator.rs`) currently has `ViewMode::{Board,
  Agent}`. Add `ViewMode::ProviderSettings`. `apply_action` flips on
  `Action::OpenProviderSettingsView` / `Action::CloseProviderSettingsView`.
- New view module `views/provider_settings/mod.rs` housing a
  `ProviderSettingsView` struct with internal state for list mode and
  edit-api-key mode, plus rendering + key handling.
- New file `app/dispatch_rpc054.rs` housing the App helpers:
  `handle_open_provider_settings_view`, `handle_close_provider_settings_view`,
  `handle_provider_credentials_loaded`, `handle_provider_test_complete`,
  `handle_provider_models_refreshed`, and the save/delete spawns.

## Risks
- Adding 6 RPCs touches 4 crates (rpc-types, core, rpc, fspec-tui).
  Sequence: rpc-types → core → rpc → fspec-tui to keep each step
  compilable. The WebSocket backend will get one-line forwarders so
  tarpc's generated client regenerates with the new methods.
- The napi feature gate on `ProviderCredentialInput` must avoid tagged
  enums (napi_derive limitation). Using a struct-with-discriminant is
  the established workaround (see how `StreamChunk` variants are
  handled).
- OAuth and custom-provider creation are EXPLICITLY out of scope; we
  surface an inline read-only notice for OAuth rows and skip the
  custom-create UI entirely (deferred to a follow-up card).
