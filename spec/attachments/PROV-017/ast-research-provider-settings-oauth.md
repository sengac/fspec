# AST Research: Provider Settings OAuth Integration

## Research Target
Existing ProviderSettingsScreen component architecture and integration points for OAuth login flow.

## Key Source Files Analyzed

### 1. ProviderSettingsScreen.tsx (Orchestrator)
- Composes `useProviderSettingsState` (state) + `useProviderSettingsInput` (keyboard) + `ProviderSettingsPanel` (UI)
- Maps hook mode to panel mode via `mapToEffectivePanelMode()`
- Calculates `visibleHeight` from terminal height minus chrome

### 2. ProviderSettingsPanel.tsx (Presentation)
- Current `PanelMode` types: `list`, `edit-api-key`, `profile-form`, `delete-confirm`
- `SettingsNavItem` types: `provider`, `profile`, `add-profile`
- Need new modes: `oauth-method-select`, `oauth-browser-waiting`, `oauth-device-waiting`, `oauth-success`, `oauth-error`
- Need new nav item: `oauth-login` (for "Login with ChatGPT (browser)" and "Login with ChatGPT (headless)")

### 3. useProviderSettingsState.ts (State Hook)
- Manages providers, nav items, mode, filter, form state, API key state
- `reload()` loads from provider registry + credentials
- Need new OAuth state: `oauthStatus`, `oauthError`, `oauthUserCode`, `oauthVerificationUrl`
- Need new actions: `startBrowserLogin()`, `startDeviceLogin()`, `cancelOauth()`

### 4. useProviderSettingsInput.ts (Input Hook)
- Dispatches to mode-specific handlers: deleteConfirm, apiKeyEdit, profileForm, filter, list
- Need new handler: `oauthModeHandler` for OAuth-specific keyboard input
- OAuth modes handle: Escape (cancel), Enter (retry from error), arrow keys (method select)

### 5. providerSettingsModeMapper.ts (Mode Mapper)
- Maps hook mode → panel mode for rendering
- Need new mappings for all 5 OAuth mode variants

### 6. NAPI Bindings (codelet/napi/src/codex_oauth.rs)
- `codex_oauth_browser_login()` → async, returns NapiCodexTokens
- `codex_oauth_device_login_start()` → async, returns NapiDeviceAuthStartResult
- `codex_oauth_device_login_poll(device_auth_id, interval)` → async, returns NapiCodexTokens
- `codex_oauth_get_tokens()` → sync, returns Option<NapiCodexTokens>
- `codex_oauth_refresh_token(refresh_token)` → async, returns NapiCodexTokens

### 7. Input Handler Pattern (src/tui/inputHandlers/)
- Each handler returns `boolean` (true = handled, false = pass through)
- Handlers take: mode, input, key, providerSettings
- Pattern: check mode type → handle keys → return true
- New file needed: `oauthModeHandler.ts`

### 8. Integration Test Pattern (ProviderSettingsScreen.integration.test.tsx)
- Uses `createProviderSettingsScreenFixture()` for test setup
- Mocks NAPI at module level with `vi.mock('@sengac/codelet-napi')`
- Uses `pressKey()` and `waitFor()` helpers from keyboardHelpers
- Tests real component rendering via ink-testing-library
- Asserts on `lastFrame()` output text content

## Provider Registry
- "codex" is NOT in the current SUPPORTED_PROVIDERS list
- Will need to be added or handled as a special provider
- OAuth tokens provide authentication (alternative to API key)
- `codex_oauth_get_tokens()` must be checked during provider config loading

## Test Strategy
- Mock all NAPI OAuth functions alongside existing mocks
- Test OAuth flow state transitions through keyboard interactions
- Verify rendering output at each state (waiting, success, error)
- Test Escape cancellation from waiting states
- Test retry flow from error state
