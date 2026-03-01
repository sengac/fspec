# PROV-017: TUI OAuth Login Flow for Provider Settings

## Overview

Add Codex OAuth login options to the ProviderSettingsScreen TUI component. When a user selects the "codex" provider and no OAuth tokens exist, they should see options to log in via browser OAuth or device auth (headless). On success, the provider list refreshes and codex becomes usable.

## Dependency Chain

```
PROV-011 (codex_oauth.rs - DONE, in specifying but code is written)
  └─ PROV-013 (Browser OAuth HTTP Server - NOT started, backlog)
  └─ PROV-014 (Device Auth Flow - NOT started, backlog)
     └─ PROV-015 (NAPI Bindings - NOT started, backlog)
        └─ PROV-016 (Custom Fetch - Token Refresh + API Rewriting - NOT started, backlog)
        └─ PROV-017 (THIS - TUI OAuth Login Flow - NOT started, backlog)
```

**CRITICAL**: PROV-013, PROV-014, PROV-015 must be completed before PROV-017. PROV-017 depends on NAPI bindings that don't exist yet.

## What's Been Implemented (PROV-011)

### codelet/providers/src/codex/codex_oauth.rs (NEW - uncommitted)
Pure library functions — no runtime/server logic:
- `generate_pkce()` → PkceCodes { verifier, challenge, challenge_method }
- `generate_state()` → String (CSRF protection)
- `parse_jwt_claims(token)` → serde_json::Value
- `extract_account_id_from_claims(claims)` → Option<String> (3 fallback paths)
- `extract_account_id(id_token, access_token)` → Option<String>
- `build_authorize_url(redirect_uri, pkce, state)` → String
- `validate_oauth_callback(callback_state, expected_state)` → Result<()>
- `rewrite_codex_url(url)` → String
- `build_codex_headers(access_token, account_id)` → HashMap
- `refresh_access_token(refresh_token)` → Result<TokenRefreshResponse> (async, uses reqwest)
- Constants: CODEX_CLIENT_ID, CODEX_ISSUER, CODEX_API_ENDPOINT, OAUTH_PORT=1455, OAUTH_TIMEOUT_MS=300000
- HTML templates: HTML_SUCCESS, html_error()
- OAuthTimeout struct with is_expired_after_ms()

### codelet/providers/src/codex/mod.rs (MODIFIED - uncommitted)
- Added `CodexAuthMode` enum: `ApiKey` | `OAuthDirect { account_id }`
- Added `from_oauth_tokens(access_token, account_id, model)` constructor
- `new()` now checks for OAuth tokens first (Mode 1: Direct Codex API), falls back to legacy API key (Mode 2)

### codelet/providers/tests/codex_oauth_test.rs (NEW - uncommitted)
Full test coverage for all 11 scenarios in codex-oauth-login.feature with @step comments.

### codelet/napi/index.d.ts (MODIFIED - uncommitted)
No OAuth NAPI bindings yet — the .node binary was rebuilt but doesn't expose OAuth functions.

## What's NOT Yet Implemented

### PROV-013: Browser OAuth HTTP Server
- Need a hyper-based HTTP server on port 1455
- Routes: `/auth/callback` (code+state), `/cancel`, 404 default
- Uses existing: generate_pkce, generate_state, build_authorize_url, validate_oauth_callback
- Exchanges code for tokens, persists via write_codex_auth
- Opens browser via `open` crate
- 5-minute timeout via OAuthTimeout

### PROV-014: Device Auth Flow
- POST to `{ISSUER}/api/accounts/deviceauth/usercode` → get device_auth_id + user_code + interval
- Display user_code and URL to user
- Poll `{ISSUER}/api/accounts/deviceauth/token` at interval
- On success: exchange authorization_code for tokens, persist

### PROV-015: NAPI Bindings (CRITICAL for this story)
Expected NAPI functions (not yet defined):
```typescript
// Browser OAuth - starts server, opens browser, resolves when callback completes
export declare function codexOauthBrowserLogin(): Promise<CodexOauthTokens>

// Device Auth - returns user_code/URL immediately, polls in background
export declare function codexOauthDeviceLogin(): Promise<CodexDeviceAuthResult>
// where CodexDeviceAuthResult = { userCode: string, verificationUrl: string, promise: Promise<CodexOauthTokens> }

// Token refresh
export declare function codexOauthRefreshToken(refreshToken: string): Promise<CodexOauthTokens>

// Read existing tokens
export declare function codexOauthGetTokens(): CodexOauthTokens | null

// Token result type
interface CodexOauthTokens {
  idToken: string
  accessToken: string
  refreshToken: string
  accountId: string
  expiresIn?: number
}
```

### PROV-016: Custom Fetch (Token Refresh + API Rewriting)
Middleware layer that intercepts API calls to:
- Auto-refresh expired tokens before requests
- Rewrite URLs from /v1/responses to chatgpt.com/backend-api/codex/responses  
- Add Bearer + ChatGPT-Account-Id headers

## Reference: OpenCode's TUI Pattern (dialog-provider.tsx)

### Architecture Overview
OpenCode uses a **plugin system** where `codex.ts` registers auth methods:
```typescript
methods: [
  { label: "ChatGPT Pro/Plus (browser)", type: "oauth", authorize: async () => {...} },
  { label: "ChatGPT Pro/Plus (headless)", type: "oauth", authorize: async () => {...} },
  { label: "Manually enter API Key", type: "api" }
]
```

### TUI Flow (dialog-provider.tsx)
1. User selects provider from list
2. If provider has >1 auth method → show method selection dialog
3. If method.type === "oauth":
   - Call `sdk.client.provider.oauth.authorize({providerID, method: index})`
   - Server returns `{ url, method: "auto"|"code", instructions }`
   - If `method === "auto"` (browser): show spinner + "Waiting for authorization..." + code
   - If `method === "code"` (headless): show text input for authorization code
4. If method.type === "api": show API key text input
5. On success: dispose instance, bootstrap sync, show model selection

### Key UI States
- **MethodSelection**: List of auth methods to choose from
- **OAuthAutoView**: URL + user_code display + spinner + "Waiting for authorization..."
- **OAuthCodeView**: URL link + text input for manual code entry
- **ApiAuthView**: Text input for API key

## Codelet's Current ProviderSettingsScreen Architecture

### Component Tree
```
ProviderSettingsScreen.tsx (orchestrator)
  ├── useProviderSettingsState.ts (hook - state management)
  ├── useProviderSettingsInput.ts (hook - keyboard handling)
  ├── providerSettingsModeMapper.ts (maps hook mode → panel mode)
  └── ProviderSettingsPanel.tsx (presentation)
```

### Current PanelMode Types
```typescript
type PanelMode =
  | { type: 'list' }                           // Provider list
  | { type: 'edit-api-key'; ... }              // API key editing
  | { type: 'profile-form'; ... }              // Profile creation/editing
  | { type: 'delete-confirm'; ... }            // Delete confirmation
```

### What Needs to Be Added (New Modes)

```typescript
// New PanelMode variants needed:
| { type: 'oauth-method-select'; providerId: string; methods: OAuthMethod[] }
| { type: 'oauth-browser-waiting'; providerId: string; url: string }
| { type: 'oauth-device-waiting'; providerId: string; userCode: string; verificationUrl: string }
| { type: 'oauth-success'; providerId: string }
| { type: 'oauth-error'; providerId: string; error: string }
```

### New Hook State Needed in useProviderSettingsState
```typescript
// OAuth flow state
oauthStatus: 'idle' | 'waiting' | 'success' | 'error';
oauthError: string | null;
oauthUrl: string | null;
oauthUserCode: string | null;
oauthVerificationUrl: string | null;
```

### New Actions in useProviderSettingsInput
- On provider with codex OAuth → show method selection (browser/device/API key)
- On "Login with ChatGPT (browser)" → call NAPI codexOauthBrowserLogin(), show waiting state
- On "Login with ChatGPT (headless)" → call NAPI codexOauthDeviceLogin(), show code+URL, wait
- On success → reload providers, set codex as active
- On failure → show error, allow retry
- Esc during OAuth → cancel/cleanup

### New Panel Rendering in ProviderSettingsPanel
- OAuth method selection list (similar to OpenCode's MethodSelection)
- Browser waiting view: URL + spinner + "Waiting for authorization..."
- Device waiting view: user_code display + URL + spinner
- Success view: "✓ Connected" message
- Error view: error message + "Press Enter to retry"

## Current State of PROV-011

PROV-011 is in "specifying" status BUT has completed implementation:
- codex_oauth.rs: Full library with all functions ✅
- codex_oauth_test.rs: Full test coverage (11 scenarios) with @step comments ✅
- codex-oauth-login.feature: Full feature file with 11 scenarios ✅
- codex-oauth-login.feature.coverage: 100% coverage mapped ✅
- CodexProvider mod.rs: Updated with OAuthDirect auth mode ✅

**PROV-011 needs to be moved through testing → implementing → validating → done** to unblock downstream work. The code exists but the ACDD lifecycle isn't completed.

## Gap Summary

| Component | Status | Blocker |
|-----------|--------|---------|
| codex_oauth.rs (library) | ✅ Done | — |
| codex_oauth_test.rs | ✅ Done | — |
| CodexProvider OAuth mode | ✅ Done | — |
| Feature file (PROV-011) | ✅ Done | — |
| Coverage mapping (PROV-011) | ✅ Done | — |
| **PROV-011 ACDD lifecycle** | ⚠️ In specifying | Needs to walk through testing→done |
| Browser HTTP server (PROV-013) | ❌ Not started | PROV-011 done |
| Device auth flow (PROV-014) | ❌ Not started | PROV-011 done |
| NAPI bindings (PROV-015) | ❌ Not started | PROV-013, PROV-014 |
| Custom fetch middleware (PROV-016) | ❌ Not started | PROV-015 |
| PanelMode OAuth variants | ❌ Not started | PROV-015 |
| useProviderSettingsState OAuth state | ❌ Not started | PROV-015 |
| useProviderSettingsInput OAuth handlers | ❌ Not started | PROV-015 |
| ProviderSettingsPanel OAuth views | ❌ Not started | PROV-015 |
| providerSettingsModeMapper OAuth mapping | ❌ Not started | PROV-015 |
| Feature file for PROV-017 | ❌ Not started | — |

## Implementation Order (When Dependencies Are Ready)

1. **Feature file**: Write `spec/features/tui-oauth-login-flow.feature` with scenarios
2. **Types**: Add OAuth PanelMode variants to ProviderSettingsPanel.tsx
3. **State**: Add OAuth state fields to useProviderSettingsState.ts
4. **Mode mapper**: Add OAuth mode mappings to providerSettingsModeMapper.ts
5. **Input**: Add OAuth keyboard handlers to useProviderSettingsInput.ts
6. **Panel**: Add OAuth view rendering to ProviderSettingsPanel.tsx
7. **Integration**: Wire up NAPI calls (codexOauthBrowserLogin, codexOauthDeviceLogin)
8. **Tests**: Write ProviderSettingsScreen integration tests for OAuth flows

## File Locations (Codelet)

- `src/tui/components/ProviderSettingsScreen.tsx` - Orchestrator
- `src/tui/components/ProviderSettingsPanel.tsx` - Presentation (types + rendering)
- `src/tui/hooks/useProviderSettingsState.ts` - State hook
- `src/tui/hooks/useProviderSettingsInput.ts` - Input hook
- `src/tui/utils/providerSettingsModeMapper.ts` - Mode mapping
- `codelet/providers/src/codex/codex_oauth.rs` - Rust OAuth library
- `codelet/napi/src/session_manager.rs` - NAPI bindings (where new functions go)
- `codelet/napi/index.d.ts` - TypeScript type declarations for NAPI

## File Locations (OpenCode Reference)

- `/tmp/opencode/packages/opencode/src/plugin/codex.ts` - Full OAuth flow + plugin
- `/tmp/opencode/packages/opencode/src/cli/cmd/tui/component/dialog-provider.tsx` - TUI OAuth dialog
- `/tmp/opencode/packages/app/src/components/dialog-connect-provider.tsx` - Web OAuth dialog
- `/tmp/opencode/packages/opencode/src/provider/auth.ts` - Provider auth orchestration
