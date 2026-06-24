# PROV-114 — AST research: GitHub Copilot OAuth device-flow surface

Work unit: PROV-114 "GitHub Copilot OAuth device flow (deployment/enterprise preamble)"
Scope: codelet-fspec-tui (views/provider_settings, app dispatch, transport) + codelet-providers copilot OAuth.
Method: AstGrep + Grep over the Rust workspace.

## 1. Existing mode enum (extend, do not replace)

`src/views/provider_settings/mode.rs:15` — `pub enum ProviderSettingsMode`
PROV-113 already added the shared login modes that PROV-114 REUSES:
- `OAuthDeviceWaiting { provider_id, user_code, verification_url }`
- `OAuthSuccess { provider_id }`
- `OAuthError { provider_id, error }`

PROV-114 ADDS two copilot-only preamble variants:
- `OAuthDeploymentTypeSelect { provider_id, selected_index }`
- `OAuthEnterpriseUrlEntry { provider_id, url_input, validation_error: Option<String> }`

Exhaustive `match`es on `ProviderSettingsMode` that MUST gain arms for the
two new variants (found via Grep):
- `src/views/provider_settings/mod.rs:195` `handle_key`
- `src/views/provider_settings/body_render.rs:23` `render_mode_body`
- `src/views/provider_settings/footer_hints.rs:31` `compute_footer_hint`

## 2. Enter routing for the login row

`src/views/provider_settings/list_actions.rs:103` —
`NavItemKind::OAuthLogin { method, .. } => oauth_login::start_oauth_login(view, provider_id, method)`

`src/views/provider_settings/oauth_login.rs:27` `start_oauth_login` —
currently keys on `method` only (Browser→waiting; Headless→emit start). PROV-114
must route `provider_id == "github-copilot"` FIRST → `OAuthDeploymentTypeSelect`
(dossier §2.1, checked before method).

`projection.rs:56` — github-copilot login methods = single
`(OAuthMethod::Headless, "Sign in with device code")` row. Label will adopt the
feature's "Login with GitHub Copilot (device flow)" wording.

## 3. Backend surface (codelet-providers-direct, NEVER codelet-napi)

Existing PROV-113 device methods (reused for poll/success/error):
- trait default `src/transport/mod.rs:652` `oauth_device_start(provider_id)`
- trait default `src/transport/mod.rs:657` `oauth_device_poll(provider_id, device_auth_id, interval)`
- embedded override `src/transport/embedded.rs:713` / `:720`

PROV-114 ADDS a copilot-specific start carrying the optional enterprise host:
- new trait method `oauth_copilot_device_start(enterprise_host: Option<String>) -> Result<OAuthDeviceStart>`
  (default Err stub in transport/mod.rs; embedded forwards to RPC client → codelet-providers).
- success path reuses `Action::OAuthDeviceReady` → `oauth_device_poll`.

codelet-providers authoritative functions (NO napi):
- `providers/src/copilot/oauth_device_code.rs:24` `normalize_enterprise_domain(&str) -> String`
  (strip `https://`/`http://` prefix, trim trailing `/`) — matches Rule 3 host normalization.
- `providers/src/copilot/oauth_device_code.rs:36` `request_device_code(host_url) -> CopilotDeviceCodeResponse`
- `providers/src/copilot/oauth_polling.rs:38` `poll_device_token(...)`
- `providers/src/copilot/mod.rs:88` re-exports `normalize_enterprise_domain`, `request_device_code`.

## 4. Action enum

`src/components/mod.rs:754-800` — PROV-113 OAuth actions. PROV-114 ADDS
`Action::OAuthCopilotDeviceStart { enterprise_host: Option<String>, generation: u64 }`,
emitted by deployment-select Enter (index 0 → None) and enterprise-url Enter
(non-empty → Some(normalized host)). Dispatch routes it in
`dispatch_provider_settings_oauth.rs` (already at 299 LoC → copilot dispatch goes
in a NEW sibling module, not appended).

## 5. Test doubles

`tests/common/mod.rs` MockBackend already has `oauth_device_start`/`oauth_device_poll`
counters + scripted results. PROV-114 adds `oauth_copilot_device_start` counter +
`Vec<Option<String>>` host capture + scripted Ok/Err so tests assert the enterprise
host is passed through. Offline only (no real network / ~/.fspec).
