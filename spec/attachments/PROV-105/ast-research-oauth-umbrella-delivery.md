# PROV-105 — AST research: delivered OAuth login/disconnect surface (umbrella)

**Date:** 2026-06-23
**Purpose:** Records the actual AST-level OAuth surface delivered across the three
child slices (PROV-112 backend/disconnect, PROV-113 anthropic+codex login,
PROV-114 github-copilot device flow). This umbrella card carried the parity dossier
(`oauth-parity-spec.md`) and split into 112/113/114; all three are DONE. The surface
below is verified by grep/AST against the working tree.

## 1. Frontend transport boundary — `fspec-tui/src/transport/mod.rs`
`FspecBackend` trait OAuth methods (websocket inherits the unsupported/no-op
defaults; `EmbeddedFspecBackend` overrides each, forwarding to the RPC layer):
- `oauth_clear_tokens(provider_id)` — :609 (PROV-112 disconnect)
- `oauth_get_tokens(provider_id) -> bool` — :616
- `supports_browser_oauth() -> bool` — :625 (browser-row gating, default false)
- `oauth_browser_login(provider_id)` — :632 (PROV-113)
- `oauth_headless_start(provider_id) -> OAuthHeadlessStart` — :637 (PROV-113)
- `oauth_headless_complete(...)` — :642 (PROV-113)
- `oauth_device_start(provider_id) -> OAuthDeviceStart` — :652 (PROV-113 codex)
- `oauth_device_poll(...)` — :657 (PROV-113 shared poll)
- `oauth_copilot_device_start(enterprise_host: Option<String>)` — :670 (PROV-114)

## 2. Frontend modes — `fspec-tui/src/views/provider_settings/mode.rs`
`ProviderSettingsMode` OAuth variants:
- `OAuthBrowserWaiting` :44, `OAuthDeviceWaiting` :50, `OAuthHeadlessCodeEntry` :59,
  `OAuthSuccess` :67, `OAuthError` :73 (PROV-113 shared)
- `OAuthDeploymentTypeSelect` :82, `OAuthEnterpriseUrlEntry` :91 (PROV-114 copilot preamble)
- `OAuthNotice` :106 (legacy placeholder, now only for not-yet-wired rows)

## 3. Backend — codelet-providers-direct (NEVER codelet-napi; no_napi guard green)
- `rpc/src/oauth_login.rs` — claude_auth/claude_oauth/claude_oauth_server +
  codex_auth/codex_device_auth/codex_oauth/codex_oauth_server (PROV-113).
- `rpc/src/oauth_copilot.rs` — copilot::auth/oauth_device_code/oauth_polling/oauth_types
  + normalize_enterprise_domain (PROV-114).
- `rpc/src/oauth_disconnect.rs` — claude_auth (delete file), codex_auth (drop tokens,
  preserve OPENAI_API_KEY), copilot delete_copilot_auth (PROV-112). All idempotent,
  errors swallowed at the UI boundary.

## 4. Delivery status
| Slice | Scope | Status | Feature |
|---|---|---|---|
| PROV-112 | backend/RPC/transport OAuth surface + disconnect-oauth confirm | DONE | provider-settings-oauth-disconnect |
| PROV-113 | anthropic+codex login (browser/headless/device, success/error/retry/cancel, generation stale-cancel) | DONE | provider-settings-oauth-login |
| PROV-114 | github-copilot device flow (deployment-select, enterprise-url, device poll) | DONE | provider-settings-oauth-copilot-device |

All acceptance criteria of the umbrella are delivered by the three children. This
umbrella carries no feature file of its own; closure reflects child completion.

## 5. Decisions honored
§8.1 TS label parity; §8.2/§8.5 RESOLUTION codelet-providers-direct via embedded (not
napi, not SessionManagerHandle — avoids user WIP); §8.3 browser rows gated to embedded;
§8.4 built-in providers only (custom_oauth out of scope, follow-up).
