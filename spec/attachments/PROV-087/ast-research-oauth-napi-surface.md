# AST research: existing OAuth NAPI surface

## Existing built-in OAuth NAPI functions

```
codelet/napi/src/claude_oauth.rs:88  claude_oauth_browser_login() -> Result<NapiClaudeTokens>
codelet/napi/src/claude_oauth.rs:149 claude_oauth_headless_start() -> NapiClaudeHeadlessStartResult
codelet/napi/src/claude_oauth.rs:169 claude_oauth_headless_complete(...)
codelet/napi/src/claude_oauth.rs:217 claude_oauth_refresh_token(refresh_token: String) -> Result<NapiClaudeTokens>
codelet/napi/src/claude_oauth.rs:238 claude_oauth_get_tokens() -> Result<Option<NapiClaudeTokens>>
codelet/napi/src/claude_oauth.rs:260 claude_oauth_clear_tokens() -> Result<()>
```

Observation: built-in providers split the browser-loopback flow into a single
`_browser_login` call (which opens the browser, spawns the loopback server, and
exchanges the code in one shot). For scripted providers we must expose the
exchange step separately because the script chooses when to exchange.

## Existing Scripted OAuth provider (PROV-060)

```
codelet/providers/src/oauth/script_provider.rs
  ScriptedOAuthProvider::load(...)
  ScriptedOAuthProvider::from_script(...)
  build_authorization_request / exchange_code / refresh_token / poll_for_token / needs_refresh
```

Observation: Script function names already use the legacy scheme. We rename
the preferred names to `auth_start` / `auth_exchange` / `auth_needs_refresh` /
`auth_refresh` and keep the legacy names as fallback so PROV-060 scripts
continue to work.

## Existing callback server

```
codelet/providers/src/oauth/callback_server.rs
```

The callback server listens on a random loopback port, opens the OS browser,
waits for the `/callback?code=...&state=...` redirect and returns the captured
values. We reuse it from the scripted provider rather than reimplementing it.

## Existing CredentialStore

```
codelet/providers/src/oauth/credential_store.rs
```

Offers get/put/delete keyed by provider name. This becomes the persistence
layer for scripted tokens, identical to built-in providers.

## Plan

1. Add `ScriptedOAuthProvider::browser_loopback_authorize()` (or a free
   function in `oauth` mod) that:
   - calls `auth_start` (falling back to `build_authorization_request`)
   - spawns the `callback_server`
   - opens the browser with the returned URL
   - returns `{ code, state, pkce_verifier }` to the caller.

2. Add `codelet/napi/src/custom_oauth.rs` exposing five `#[napi] pub async fn`:
   - `custom_oauth_authorize(provider_name)`
   - `custom_oauth_exchange(provider_name, code, verifier)`
   - `custom_oauth_needs_refresh(provider_name)`
   - `custom_oauth_refresh(provider_name)`
   - `custom_oauth_clear(provider_name)`

3. Extend `ScriptedOAuthProvider` to try the new `auth_*` names first, then
   the legacy PROV-060 names.

4. Tokens returned by `auth_exchange` / `auth_refresh` are stored via
   `CredentialStore::put(provider_name, tokens)`; `custom_oauth_clear` calls
   `CredentialStore::delete(provider_name)`.
