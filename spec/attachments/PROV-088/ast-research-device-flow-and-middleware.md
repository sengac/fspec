# AST research: device flow + refresh middleware

## Existing device-code flow building blocks

```
codelet/providers/src/oauth/device_flow.rs
  DeviceCodeFlow<P: DeviceCodeProvider>
  DeviceCodeProvider trait - request_device_code, poll, etc.
  PollResponse { Pending, Success, Denied, Expired, SlowDown }
```

PROV-088 reuses PollResponse as the canonical `status` value set but
does not couple ScriptedDeviceFlow to DeviceCodeFlow (no HTTP - we
delegate all HTTP to the Rhai script via `auth_poll`).

## Existing refresh middleware

```
codelet/providers/src/oauth/http_middleware.rs
  RefreshingHttpClient<S: TokenStrategy>
  TokenStrategy trait
codelet/providers/src/oauth/token_refresh.rs
  ensure_fresh_token free fn
```

These only handle Claude/Codex/Copilot-specific token types. PROV-088
introduces a parallel async helper for Rhai-scripted providers that
works on generic Rhai Maps stored by PROV-087.

## PROV-087 pieces we extend

```
codelet/providers/src/oauth/script_provider_aliases.rs
  auth_start_or_legacy / auth_exchange_or_legacy / auth_needs_refresh_or_legacy / auth_refresh_or_legacy
codelet/providers/src/oauth/custom_oauth.rs
  custom_oauth_store_path, read_stored_tokens, write_stored_tokens
```

PROV-088 adds:
1. `auth_poll_or_legacy` - wrapper that calls `auth_poll` with a
   fallback to PROV-060 `poll_for_token`.
2. `ScriptedDeviceFlow::start`/`poll` helpers that return normalised
   maps.
3. `scripted_refreshing_client::ensure_fresh_if_needed(provider_name, &ScriptedOAuthProvider)`
   that reads tokens, checks needs_refresh, optionally calls refresh
   and writes back.
