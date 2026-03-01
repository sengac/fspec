# PROV-019: Codex OAuth Integration — Bug Analysis & OpenCode Reference

## Screenshots

- `codex-1.png` — Provider Settings: Codex (ChatGPT) expanded but no OAuth login items visible, no status indicator
- `codex-2.png` — 401 `token_expired` errors on every API call (3 retries, all fail)
- `codex-3.png` — "Edit API Key: Codex (ChatGPT)" form shown when pressing 'e' on Codex provider

---

## Bug 1: 401 token_expired — RefreshingCodexClient thinks stale tokens are fresh

### Root Cause

In `codelet/providers/src/codex/mod.rs` line 98-105, `CodexProvider::new()` loads tokens from `~/.codex/auth.json` and calls:

```rust
Self::from_oauth_tokens(
    &tokens.access_token,
    &tokens.refresh_token,
    &tokens.account_id,
    None, // expires_in not stored in auth.json — defaults to 3600s
    codex_oauth::CODEX_ISSUER,
    &model_name,
);
```

In `refreshing_client.rs` line 82-83, `new_oauth()` sets:

```rust
let expiry_secs = expires_in_secs.unwrap_or(DEFAULT_EXPIRY_SECS); // 3600
let expires_at = Instant::now() + Duration::from_secs(expiry_secs);
```

This means tokens loaded from disk (which could be **weeks old**) are treated as fresh for 1 hour from the moment the provider is constructed. The `ensure_fresh_token()` check passes, the stale access_token is sent, and the API returns 401.

### Fix

When loading tokens from disk (as opposed to receiving fresh tokens from an OAuth flow), **assume the access_token is already expired**. Set `expires_at` to `Instant::now()` (or earlier) so the very first request triggers an immediate refresh via the refresh_token.

Concrete change in `codex/mod.rs`:

```rust
// Pass Some(0) to force immediate refresh on first request
Self::from_oauth_tokens(
    &tokens.access_token,
    &tokens.refresh_token,
    &tokens.account_id,
    Some(0), // Force immediate refresh — tokens from disk are of unknown age
    codex_oauth::CODEX_ISSUER,
    &model_name,
);
```

### How OpenCode handles this

OpenCode checks token expiry on **every request** using a wall-clock timestamp (`Date.now()`), not a relative `Instant`:

```typescript
// packages/opencode/src/plugin/codex.ts lines 438-454
if (!currentAuth.access || currentAuth.expires < Date.now()) {
  log.info("refreshing codex access token")
  const tokens = await refreshAccessToken(currentAuth.refresh)
  // ... update stored auth with new tokens and new expiry
  await input.client.auth.set({
    body: {
      type: "oauth",
      refresh: tokens.refresh_token,
      access: tokens.access_token,
      expires: Date.now() + (tokens.expires_in ?? 3600) * 1000,
    },
  })
}
```

Key differences:
- OpenCode stores `expires` as an **absolute wall-clock timestamp** (milliseconds since epoch), not a relative duration
- On every fetch, it checks `currentAuth.expires < Date.now()` — if the stored expiry is in the past, it refreshes
- Our code uses `tokio::time::Instant` which resets on each process start, so there's no way to persist or restore "when was this token actually issued"

---

## Bug 2: "Edit API Key" form shown for Codex provider

### Root Cause

In `src/tui/inputHandlers/listModeHandler.ts` lines 141-146, the `'e'` key handler unconditionally opens the API key editor for any provider:

```typescript
if (input === 'e' || input === 'E') {
    if (currentItem.type === 'provider') {
      providerSettings.setEditingApiKey('');
      providerSettings.setMode({
        type: 'edit-api-key',
        providerId: currentItem.providerId,
      });
    }
```

There is no check for whether the provider uses OAuth instead of API keys.

### Fix

For the `codex` provider, pressing `'e'` should either:
- Start the browser OAuth login flow (same as clicking the OAuth login item), or
- Show a message explaining that Codex uses OAuth authentication

```typescript
if (input === 'e' || input === 'E') {
    if (currentItem.type === 'provider') {
      if (currentItem.providerId === 'codex') {
        // Codex uses OAuth, not API keys — start browser login
        providerSettings.startBrowserLogin(currentItem.providerId);
      } else {
        providerSettings.setEditingApiKey('');
        providerSettings.setMode({
          type: 'edit-api-key',
          providerId: currentItem.providerId,
        });
      }
    }
```

Same fix needed in `src/tui/components/ProviderSettingsView.tsx` lines 490-494 (the legacy handler).

### How OpenCode handles this

OpenCode's provider system doesn't have a generic "edit API key" action. Instead, each provider's auth is defined by a plugin with explicit `methods` array:

```typescript
// packages/opencode/src/plugin/codex.ts lines 497-615
methods: [
  {
    label: "ChatGPT Pro/Plus (browser)",
    type: "oauth",
    authorize: async () => { /* browser OAuth flow */ },
  },
  {
    label: "ChatGPT Pro/Plus (headless)",
    type: "oauth",
    authorize: async () => { /* device auth flow */ },
  },
  {
    label: "Manually enter API Key",
    type: "api",
  },
],
```

The UI renders these methods as choices. There is no generic "edit key" action that would accidentally appear for OAuth-based providers.

---

## Bug 3: Provider Settings display issues for Codex

### Root Cause

The `useProviderSettingsState.ts` hook checks for OAuth tokens and sets `hasOAuthTokens`, and the nav items builder conditionally shows OAuth login options. However:

1. When `hasOAuthTokens === true`, the OAuth login options are hidden (correct), but there's no positive indicator like "✓ OAuth connected" or "Logout" option
2. When `hasOAuthTokens === false`, the OAuth login items are shown, but only when the provider is expanded — the collapsed line just shows "Codex (ChatGPT)" with no status

### Fix

- When Codex has OAuth tokens: show "✓ OAuth" as the status source (already partially done — `source: 'OAuth'` is set but may not render correctly)
- Add a "Logout" or "Disconnect" option when OAuth tokens exist
- Consider showing the OAuth email or account info as the masked key display

### How OpenCode handles this

OpenCode stores auth state per-provider and the UI renders the current auth state (connected/disconnected) with the auth method label. The codex plugin is registered under the `openai` provider ID with `auth.provider: "openai"`, so the openai provider row shows the OAuth connection status.

---

## OpenCode Architecture Comparison

### Key Architectural Difference

OpenCode treats Codex as a **plugin that hooks into the OpenAI provider**, not as a separate provider:

```typescript
// packages/opencode/src/plugin/codex.ts line 353-355
auth: {
  provider: "openai",  // ← hooks into openai, not a separate provider
  async loader(getAuth, provider) {
    const auth = await getAuth()
    if (auth.type !== "oauth") return {}
    // Filter models to only codex models
    // Zero out costs
    // Return custom fetch that rewrites URLs
  }
}
```

When OAuth is active for the `openai` provider:
1. The `loader` filters the openai model list to only codex-compatible models
2. It returns a custom `fetch` function that:
   - Strips the dummy API key authorization header
   - Checks token expiry and refreshes if needed
   - Rewrites the URL from `api.openai.com/v1/responses` to `chatgpt.com/backend-api/codex/responses`
   - Sets `Bearer {access_token}` and `ChatGPT-Account-Id` headers
3. It injects a `gpt-5.3-codex` model if missing from models.dev

### Our Architecture

We have a separate Rust `CodexProvider` with its own `RefreshingCodexClient` HTTP middleware. The TypeScript TUI creates a synthetic "codex" section by extracting codex models from the OpenAI models.dev list. Session creation routes `codex/model-id` to the Rust CodexProvider.

This is fundamentally different but workable — the main gap is the token expiry handling described in Bug 1.

---

## Files to Modify

### Rust
- `codelet/providers/src/codex/mod.rs` — Pass `Some(0)` instead of `None` for `expires_in_secs` when loading from auth.json
- `codelet/providers/src/codex/refreshing_client.rs` — (Optional) Add logging when token refresh is triggered

### TypeScript
- `src/tui/inputHandlers/listModeHandler.ts` — Skip API key edit for codex, launch OAuth instead
- `src/tui/components/ProviderSettingsView.tsx` — Same fix for the legacy 'e' handler
