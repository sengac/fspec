# PROV-105 — OAuth Login & Disconnect: Deep TypeScript-Parity Specification

> **Purpose.** This document is the authoritative, exhaustive parity reference for wiring
> the **Rust fspec-tui** Provider Settings OAuth **login** and **disconnect/logout** flows
> to match the original **TypeScript TUI**. Every behavioural claim is traceable to a
> `file:line` in the TS source or the current Rust source. It is intended to contain ALL
> information required to reproduce deep parity without re-reading the TS implementation.
>
> Companion scope note: `oauth-flows-scope.md` (problem statement & gate constraints).

---

## 0. Architecture at a glance

The TS flow is split across layers; the Rust port must mirror the same separation:

| Concern | TypeScript source | Rust target |
| --- | --- | --- |
| Nav-row labels (which login/logout rows exist) | `src/tui/utils/oauthLoginLabels.ts`, `oauthProviderLabels.ts` | `provider_settings/projection.rs`, `nav_item.rs` (already parity-ready) |
| List-mode Enter/`d` dispatch | `src/tui/inputHandlers/listModeHandler.ts:119-175` | `provider_settings/list_actions.rs` (currently STUBBED) |
| State machine + napi calls (anthropic/codex) | `src/tui/hooks/useProviderSettingsState.ts` | new dispatch handlers + `App` actions |
| State machine + napi calls (copilot) | `src/tui/utils/copilotLoginFlow.ts` | new dispatch handlers |
| Keyboard during OAuth modes (anthropic/codex) | `src/tui/inputHandlers/oauthModeHandler.ts` | new `ProviderSettingsMode` sub-handlers |
| Keyboard during copilot deployment/url modes | `src/tui/inputHandlers/copilotOauthModeHandler.ts` | new `ProviderSettingsMode` sub-handlers |
| Mode types | `src/tui/types/settingsMode.ts:18-64` | new `ProviderSettingsMode` variants |
| Mode mapping (the only translation point) | `src/tui/utils/providerSettingsModeMapper.ts` | view-level mode → render selection |
| Rendering / user-visible strings | `ProviderSettingsPanel.tsx`, `CopilotOauthRender.tsx`, `oauthProviderLabels.ts` | `provider_settings` renderers |
| OAuth backend (token exchange/persist) | `@sengac/codelet-napi` (already in Rust!) | `codelet/napi/src/{claude,codex,copilot,custom}_oauth.rs` (DONE) |

**Key insight:** All OAuth token exchange/persistence already exists in `codelet/napi`. PROV-105
is a **wiring** card: expose those napi flows across the RPC/transport boundary and drive them
from the fspec-tui frontend, replacing the dead-end `DetailSub::OAuthNotice` placeholder.

---

## 1. OAuth providers, nav rows, labels (display surface — already parity-ready)

### 1.1 OAuth providers
`authType: 'oauth'` providers (TS `provider-registry.ts:61,202,212`): **`anthropic`, `codex`, `github-copilot`**.
Rust equivalent `projection.rs:24-26` `is_oauth_provider`: `credential_type=="oauth"` OR id ∈ {anthropic, codex, github-copilot}. ✅ matches.

### 1.2 Login rows per provider — `OAUTH_LOGIN_REGISTRY` (`oauthLoginLabels.ts:39-51`)
Exact rows, exact ordering, exact label strings:

| providerId | order | method | label |
| --- | --- | --- | --- |
| `anthropic` | 1 | `browser` | `Login with Claude (browser)` |
| `anthropic` | 2 | `headless` | `Login with Claude (headless)` |
| `codex` | 1 | `browser` | `Login with ChatGPT (browser)` |
| `codex` | 2 | `headless` | `Login with ChatGPT (headless)` |
| `github-copilot` | 1 | `headless` | `Login with GitHub Copilot (device flow)` |

- `buildOauthLoginNavItems(providerId)` (`oauthLoginLabels.ts:61-72`) → `{ type:'oauth-login', providerId, method, label }`. **Fails closed** (`[]`) for unknown providers.
- `OauthLoginMethod = 'browser' | 'headless'` (`oauthLoginLabels.ts:22`). GitHub Copilot is device-flow only (single headless row).
- **Rust note:** current Rust label strings in `projection.rs:43-66` differ ("Sign in with browser" / "Sign in with code" / "Sign in with device code"). **PARITY DECISION REQUIRED** — see §8 Open Decisions. The TS strings above are the parity target; the Rust strings are the current state.

### 1.3 Logout/status row (`useProviderSettingsState.ts:159-165`)
When `isOAuthProvider(id) && hasOAuthTokens`, a row is added **before** the login rows:
```
{ type:'oauth-status', providerId, label: `Logout from OAuth [${provider.status?.source || provider.name}]` }
```
Source tag comes from `provider.status.source` (set during reload, §5.4). e.g. `Logout from OAuth [Claude]`, `Logout from OAuth [ChatGPT]`, `Logout from OAuth [GitHub Copilot]`.

### 1.4 Nav ordering within an expanded OAuth provider (`buildNavItems`, `useProviderSettingsState.ts:132-206`)
Exact order:
1. `provider` row.
2. `oauth-status` row — **only if** `hasOAuthTokens`.
3. `oauth-login` rows — **always** (initial login AND re-login), one per registry entry.
4. `api-key` row — only if `id !== 'openai'` and registry `requiresApiKey || envVar`.
5. profiles — only for `openai`.

Rust `nav_item.rs:119-140 build_nav_items` already emits OAuthStatus (when `is_oauth && has_oauth_tokens`) then OAuthLogin rows. ✅ ordering matches.

### 1.5 Provider → label/source map (`oauthProviderLabels.ts:38-60`)

| providerId | source | browserWaitingTitle | deviceWaitingTitle | disconnectLabel | successLabel |
| --- | --- | --- | --- | --- | --- |
| `anthropic` | `Claude` | `Claude OAuth Login` | `Claude Device Login` | `Disconnect Claude OAuth?` | `✓ Connected to Claude` |
| `codex` | `ChatGPT` | `Codex OAuth Login` | `Codex Device Login` | `Disconnect ChatGPT OAuth?` | `✓ Connected to ChatGPT` |
| `github-copilot` | `GitHub Copilot` | `GitHub Copilot OAuth Login` | `GitHub Copilot Device Login` | `Disconnect GitHub Copilot OAuth?` | `✓ Connected to GitHub Copilot` |

**FALLBACK** (`oauthProviderLabels.ts:68-74`, used for unknown id — never throws):
`source:'OAuth'`, `browserWaitingTitle:'OAuth Login'`, `deviceWaitingTitle:'Device Login'`, `disconnectLabel:'Disconnect OAuth?'`, `successLabel:'✓ Connected'`.

`getOauthProviderLabels(id)` = `REGISTRY[id] ?? FALLBACK` (`oauthProviderLabels.ts:81-84`).

---

## 2. List-mode dispatch: what Enter and `d` do (`listModeHandler.ts:119-175`)

### 2.1 Enter on an `oauth-login` row (`listModeHandler.ts:122-132`)
```
if (currentItem.type === 'oauth-login') {
  if (currentItem.providerId === 'github-copilot') startCopilotLogin(ps, providerId);   // checked FIRST
  else if (currentItem.method === 'browser')        ps.startBrowserLogin(providerId);
  else if (currentItem.method === 'headless')       ps.startDeviceLogin(providerId);
}
```

| Provider | method | Function | Resulting mode | Keyboard handler |
| --- | --- | --- | --- | --- |
| anthropic | browser | `startBrowserLogin('anthropic')` | `oauth-browser-waiting` | `oauthModeHandler` |
| anthropic | headless | `startDeviceLogin('anthropic')` | `oauth-headless-code-entry` | `oauthModeHandler` |
| codex | browser | `startBrowserLogin('codex')` | `oauth-browser-waiting` | `oauthModeHandler` |
| codex | headless | `startDeviceLogin('codex')` | `oauth-device-waiting` | `oauthModeHandler` |
| github-copilot | headless row | `startCopilotLogin('github-copilot')` | `oauth-deployment-type-select` | `copilotOauthModeHandler` |

> Copilot's row is `method:'headless'` but the `providerId === 'github-copilot'` check is FIRST, so it never reaches `startDeviceLogin` (`listModeHandler.ts:126-127`).

### 2.2 Enter on an `oauth-status` row (`listModeHandler.ts:148-152`)
→ `setMode({ type:'disconnect-oauth', providerId })`. **No napi call yet** — opens confirm only.

### 2.3 `d`/`D` on rows (`listModeHandler.ts:158-175`)
- `oauth-status` (`:164-168`) → `setMode({ type:'disconnect-oauth', providerId })` — **identical to Enter**.
- `api-key` → `delete-api-key`; `profile` → `delete-profile`.
- (`d` on an `oauth-login` row → no-op in TS.)

> **Rust current state (`list_actions.rs:124-126,134`):** `d` on OAuthStatus collapses into the generic `open_delete_confirm` (delete-credentials), and `d` on OAuthLogin is an explicit no-op. PROV-105 must change `d`/Enter on OAuthStatus to open a dedicated **disconnect-oauth** confirm (not the api-key delete confirm).

---

## 3. BROWSER login flow (anthropic & codex)

### 3.1 TS control flow — `startBrowserLogin(providerId)` (`useProviderSettingsState.ts:555-585`)
1. `const thisGeneration = ++oauthGeneration.current;` — generation counter to invalidate stale promises after cancel (`:557`).
2. Set retry refs: `oauthLastMethodRef='browser'`, `oauthProviderIdRef=providerId` (`:558-559`).
3. `setMode({ type:'oauth-browser-waiting', providerId })` (`:560`) — UI shows waiting screen immediately.
4. Fire-and-forget async IIFE (`:562-582`):
   - anthropic → `await claudeOauthBrowserLogin()` (`:565`); codex → `await codexOauthBrowserLogin()` (`:567`).
   - **Stale check**: if `oauthGeneration.current !== thisGeneration` return silently (user cancelled) (`:569-571`).
   - Success → `setMode({type:'oauth-success', providerId})` then `await reload()` (`:572-573`).
   - Error → stale check again; `errorMsg = err instanceof Error ? err.message : 'OAuth login failed'`; `setMode({type:'oauth-error', providerId, error})` (`:578-580`).

### 3.2 What the napi call does internally
- **`claudeOauthBrowserLogin()`** — binds a local HTTP server on an **ephemeral port**, opens browser to authorize URL, shows a form to paste `code#state`, validates state, exchanges code → tokens, **persists to `claude_auth.json`**. Returns `NapiClaudeTokens`.
- **`codexOauthBrowserLogin()`** — binds local HTTP server on **port 1455**, opens browser, awaits callback with a **5-minute timeout**, exchanges code → tokens, **persists to `auth.json`**. Returns `NapiCodexTokens`.

The entire authorize-URL build / browser open / callback capture / exchange / persist is INSIDE napi. The TUI shows only a spinner.

### 3.3 UI — `oauth-browser-waiting` (`ProviderSettingsPanel.tsx:379-402`)
- Title (bold yellow): `getOauthProviderLabels(providerId).browserWaitingTitle` → "Claude OAuth Login" / "Codex OAuth Login".
- Spinner line (cyan `⠋ ` + white): **"Waiting for authorization..."**
- Dim hint: **"Press Esc to cancel"**

### 3.4 Keyboard — browser-waiting (`oauthModeHandler.ts:29-38`)
- Esc → `cancelOauth()`. **All other input absorbed** (returns `true`).

### 3.5 Success & error UI + keyboard (shared by all flows)
- **Success** (`ProviderSettingsPanel.tsx:441-460`): bold green `successLabel` ("✓ Connected to Claude/ChatGPT/GitHub Copilot"); dim hint **"Press Enter or Esc to continue"**. Keyboard (`oauthModeHandler.ts:97-103`): Enter OR Esc → `setMode({type:'list'})`; all absorbed.
- **Error** (`ProviderSettingsPanel.tsx:517-538`): bold red **"OAuth Login error"**; red `{mode.error}`; dim hint **"Press Enter to retry | Esc to go back"**. Keyboard (`oauthModeHandler.ts:106-114`): Enter → `retryOauth()`; Esc → `cancelOauth()`; all absorbed.
- `retryOauth()` (`useProviderSettingsState.ts:670-682`): reads `oauthProviderIdRef`/`oauthLastMethodRef`; `'browser'` → `startBrowserLogin(pid)`; `'device'` → `startDeviceLogin(pid)`.
- `cancelOauth()` (`:661-665`): `++oauthGeneration` (invalidates running promise), reset refs, `setMode({type:'list'})`.

---

## 4. HEADLESS / DEVICE login flows

There are THREE distinct headless mechanisms behind the "headless" method, dispatched by
`startDeviceLogin` (`useProviderSettingsState.ts:590-656`) and `startCopilotLogin`:
- **anthropic** → paste `code#state` (`oauth-headless-code-entry`) — TWO-PHASE.
- **codex** → device-code poll (`oauth-device-waiting`) — single-phase async.
- **github-copilot** → device flow with deployment-type/enterprise-URL preamble (§4.3).

### 4.1 Anthropic headless ("Sign in with code") — TWO-PHASE

**Phase 1 — start** (`startDeviceLogin`, `:596-618`):
1. `++oauthGeneration.current`; refs `oauthLastMethodRef='device'`, `oauthProviderIdRef=providerId`.
2. `const result = claudeOauthHeadlessStart();` — **SYNCHRONOUS** napi call (`:600`).
3. Stale check (`:601-603`).
4. `setMode({ type:'oauth-headless-code-entry', providerId, authorizeUrl: result.authorizeUrl, pkceVerifier: result.pkceVerifier, codeInput: '' })` (`:604-610`).
5. Error → `errorMsg = err.message || 'Headless login failed'`; `setMode({type:'oauth-error',...})`.

`claudeOauthHeadlessStart()` → `{ authorizeUrl, pkceVerifier }`. Generates PKCE + builds authorize URL synchronously so it can be displayed immediately. URL contains `claude.ai/oauth/authorize`, `code_challenge=`, `code_challenge_method=S256`, `response_type=code`, `redirect_uri`, `state`; verifier ≥ 43 chars; unique per call.

**UI — `oauth-headless-code-entry`** (`ProviderSettingsPanel.tsx:463-495`):
- Bold yellow: **"Claude Headless Login"**
- **"Visit: "** + blue `{authorizeUrl}`
- **"Authorize on claude.ai, then paste code#state below:"**
- Input box (`width=max(20, width-12)`, `wrap="truncate"`): cyan **"Code: "** + `{codeInput}` + inverse cursor block.
- Dim hint: **"Enter to submit | c: copy URL | o: open URL | Esc to cancel"**

**Keyboard — headless-code-entry** (`oauthModeHandler.ts:41-94`):
- Esc → `cancelOauth()`.
- Enter → if `codeInput.length>0` → `submitHeadlessCode(codeInput, pkceVerifier)`; else no-op.
- Backspace/Delete → if non-empty, drop last char.
- `'c'` (ONLY when `codeInput.length===0`, not ctrl/meta) → `copyToClipboard(authorizeUrl)` (errors swallowed) — PROV-028.
- `'o'` (ONLY when `codeInput.length===0`, not ctrl/meta) → `openInBrowser({url:authorizeUrl, wait:false})` (errors swallowed) — PROV-028.
- Any other printable (not ctrl/meta) → append to `codeInput`.
- All input absorbed. NOTE: once `codeInput` non-empty, `c`/`o` append as normal chars (`'abc'`+`'c'`→`'abcc'`).

**Phase 2 — complete** (`submitHeadlessCode`, `:714-738`):
1. `++oauthGeneration.current`; `providerId='anthropic'` (hard-coded).
2. `await claudeOauthHeadlessComplete(codeWithState, pkceVerifier)` (`:721`).
3. Stale check; success → `setMode({type:'oauth-success', providerId:'anthropic'})` + `await reload()`.
4. Error → `errorMsg = err.message || 'Headless login failed'`; `setMode({type:'oauth-error',...})`.

`claudeOauthHeadlessComplete(codeWithState, pkceVerifier)` → `NapiClaudeTokens`. Validates state (CSRF), exchanges code, **persists to `claude_auth.json`**.

### 4.2 Codex headless (device-code poll) — single-phase async (`:619-653`)
1. `const result = await codexOauthDeviceLoginStart();` (`:623`).
2. Stale check.
3. `setMode({ type:'oauth-device-waiting', providerId, userCode: result.userCode, verificationUrl: result.verificationUrl })` (`:627-632`).
4. `await codexOauthDeviceLoginPoll(result.deviceAuthId, result.interval)` (`:635-638`) — blocks until authorize/expire/error.
5. Stale check; success → `oauth-success` + `await reload()`.
6. Error → `errorMsg = err.message || 'Device auth failed'`; `oauth-error`.

`codexOauthDeviceLoginStart()` → `{ userCode, verificationUrl, deviceAuthId, interval }`.
`codexOauthDeviceLoginPoll(deviceAuthId, interval)` → `NapiCodexTokens`; polls at `interval`, extracts account_id from JWT, **persists to `auth.json`**.

**UI — `oauth-device-waiting`** (`ProviderSettingsPanel.tsx:405-438`):
- Bold yellow: `getOauthProviderLabels(providerId).deviceWaitingTitle` ("Codex Device Login" / "Claude Device Login" / "GitHub Copilot Device Login").
- **"Your code: "** + bold cyan `{userCode}`
- **"Visit: "** + blue `{verificationUrl}`
- Spinner (cyan `⠋ `) + **"Enter the code on another device"**
- Dim hint: **"Press Esc to cancel"**

**Keyboard — device-waiting** (`oauthModeHandler.ts:29-38`): same as browser-waiting (Esc → cancel, all else absorbed).

### 4.3 GitHub Copilot device flow (multi-step) — `copilotLoginFlow.ts`

**Step 1 — `startCopilotLogin(ps, providerId)`** (`copilotLoginFlow.ts:46-55`):
- `setMode({ type:'oauth-deployment-type-select', providerId, selectedIndex: 0 })`.

**UI — deployment-type-select** (`CopilotOauthRender.tsx:43-76`):
- Bold yellow: **"GitHub Copilot Login — Select deployment type"**
- Two radio options (`DEPLOYMENT_OPTIONS`): **"GitHub.com" — "Public"** (idx 0); **"GitHub Enterprise" — "Self-hosted / data residency"** (idx 1). Active row `▶ ` black-on-cyan; inactive `  ` dim.
- Dim hint: **"↑/↓: switch · Enter: select · Esc: cancel"**

**Keyboard — deployment-type-select** (`copilotOauthModeHandler.ts:37-65`):
- Esc → `cancelOauth()`. Up → `selectedIndex:0`; Down → `selectedIndex:1`.
- Enter → `choice = selectedIndex===0 ? 'github.com' : 'enterprise'`; `submitCopilotDeploymentType(ps, choice)`. All else absorbed.

**Step 2 — `submitCopilotDeploymentType(ps, deployment)`** (`copilotLoginFlow.ts:65-78`):
- `'enterprise'` → `setMode({ type:'oauth-enterprise-url-entry', providerId:'github-copilot', urlInput:'' })`.
- `'github.com'` → `await beginCopilotDevicePolling(ps, null)`.

**UI — enterprise-url-entry** (`CopilotOauthRender.tsx:81-113`):
- Bold yellow: **"GitHub Copilot Login — Enter Enterprise URL"**
- **"Type your enterprise host (e.g. company.ghe.com):"**
- Input box (truncate): cyan **"URL: "** + `{urlInput}` or dim placeholder **"company.ghe.com"** + inverse cursor.
- If `validationError`: red `{validationError}` line.
- Dim hint: **"Enter: submit · Esc: cancel"**

**Keyboard — enterprise-url-entry** (`copilotOauthModeHandler.ts:68-116`):
- Esc → `cancelOauth()`.
- Enter → if `urlInput.length===0` set `validationError: 'URL or domain is required'`; else `submitCopilotEnterpriseUrl(ps, urlInput)`.
- Backspace/Delete → pop last char, clear `validationError`.
- Printable ASCII 32–126 only → append, clear `validationError`. All input absorbed.

**Step 3 (enterprise) — `submitCopilotEnterpriseUrl(ps, rawUrl)`** (`copilotLoginFlow.ts:85-91`):
- `const host = copilotNormalizeEnterpriseDomain(rawUrl)` (sync; strips scheme & trailing `/`).
- `await beginCopilotDevicePolling(ps, host)`.

**Shared — `beginCopilotDevicePolling(ps, enterpriseHost)`** (`copilotLoginFlow.ts:98-141`):
1. `const start = await copilotOauthDeviceLoginStart(enterpriseHost)` (`:103`).
2. `setMode({ type:'oauth-device-waiting', providerId:'github-copilot', userCode: start.userCode, verificationUrl: start.verificationUrl })`.
3. Fire-and-forget poll IIFE: `await copilotOauthDeviceLoginPoll(start.deviceCode, start.interval, start.hostUrl, start.enterpriseHost ?? null)`.
   - Success → `oauth-success` + `await ps.reload()`.
   - Error → `message = err.message || 'Copilot device login failed'`; `oauth-error`.
4. Outer catch (start failed) → `message = err.message || 'Failed to start Copilot device login'`; `oauth-error`.

napi: `copilotOauthDeviceLoginStart(enterpriseUrl?)` → `{ userCode, verificationUrl, deviceCode, interval, hostUrl, deploymentType, enterpriseHost? }` (null → `https://github.com/login/device/code`). `copilotOauthDeviceLoginPoll(deviceCode, interval, hostUrl, enterpriseHost?)` → `NapiCopilotCredential`; **persists to `~/.fspec/credentials/copilot_auth.json` (mode 0600)**. `copilotNormalizeEnterpriseDomain(input)` → `string` (sync, pure).

---

## 5. DISCONNECT / LOGOUT flow

### 5.1 Trigger (covered §2.2/§2.3)
Enter OR `d`/`D` on an `oauth-status` row → `setMode({ type:'disconnect-oauth', providerId })`. No token mutation yet.

### 5.2 Confirm dialog — `deleteConfirmModeHandler.ts:36-73`
`disconnect-oauth` passes through `mapToEffectivePanelMode` unchanged (`providerSettingsModeMapper.ts:47-49`). Footer hint for oauth-status: `"Enter: logout · / filter · Tab: Switch to models · Esc: close"`.
```
if (mode.type === 'disconnect-oauth') {
  return handleConfirmation(input, key,
    () => providerSettings.disconnectOauth(mode.providerId),   // onConfirm
    () => providerSettings.setMode({ type: 'list' }));         // cancel
}
```
`handleConfirmation` (`:14-29`):
- `'y'|'Y'` → `void onConfirm().then(onCancel)` → disconnect THEN return to list. Returns `true`.
- `Esc | 'n'|'N'` → `onCancel()` only (back to list, tokens preserved). Returns `true`.
- **Any other input → returns `true` (consumes all input in confirm mode).**

### 5.3 `disconnectOauth(providerId)` — napi calls & files (`useProviderSettingsState.ts:690-709`)
```
if (isOAuthProvider(providerId)) {
  if (providerId === 'anthropic')            await claudeOauthClearTokens();
  else if (providerId === 'github-copilot')  await copilotOauthClearCredential();
  else                                       codexOauthClearTokens();   // codex/fallback — SYNC
}
navigateToProviderRef.current = providerId;
await reload();
// catch → logger.error(...) swallowed, never rethrown
```

| providerId | napi fn | sync/async | File mutated | Removed / preserved |
| --- | --- | --- | --- | --- |
| `anthropic` | `claudeOauthClearTokens()` | async | `claude_auth.json` | whole file deleted; idempotent |
| `github-copilot` | `copilotOauthClearCredential()` | async | `~/.fspec/credentials/copilot_auth.json` | credential deleted; idempotent |
| `codex` (+fallback) | `codexOauthClearTokens()` | **sync** | `auth.json` | removes **only** `tokens` field; **PRESERVES cached `OPENAI_API_KEY`** |

- Guarded by `isOAuthProvider(providerId)` (`authType==='oauth'`).
- Errors caught + logged, never rethrown; UI silently returns to list.

### 5.4 Nav-tree refresh after disconnect (and after every successful login)
`navigateToProviderRef.current = providerId;` then `await reload()` (`:702-703`).
`reload` (`:251-351`) reprobes EVERY provider's tokens:
- anthropic → `await claudeOauthGetTokens()` (`:284`); sets `status.source = 'Claude'`, `maskedKey:'OAuth'` when present.
- github-copilot → `await copilotOauthGetCredential()` (`:295`); source `"GitHub Copilot (<enterpriseUrl>)"` if enterprise, else base.
- codex → `codexOauthGetTokens()` (sync, `:310`); source `'ChatGPT'`.
- After clear, these return null → `hasOAuthTokens=false` → no `oauth-status` row built.
- Expansion state preserved via `expandedProviderIds` ref.
Post-reload cursor (`:367-379`): `idx = navItems.findIndex(provider row == target)`; `setSelectedIndex(idx)`; `setScrollOffset(max(0, idx-2))` → **cursor returns to the parent provider row** (PROV-036). Same as `removeApiKey`/`removeProfile`.

---

## 6. Settings mode types (TS `settingsMode.ts:18-64`) → Rust mode variants

TS `HookMode` OAuth-related variants the Rust `ProviderSettingsMode` must gain (TS currently
has dedicated modes; Rust only has `List`, `Detail{sub}`, `CreateProfile`, `EditProfile`):

| TS HookMode | Fields | Rust target variant (proposed) |
| --- | --- | --- |
| `disconnect-oauth` (`:22`) | providerId | `DisconnectOAuth { provider_id }` (confirm dialog) |
| `oauth-browser-waiting` (`:26`) | providerId | `OAuthBrowserWaiting { provider_id }` |
| `oauth-device-waiting` (`:27-32`) | providerId, userCode, verificationUrl | `OAuthDeviceWaiting { provider_id, user_code, verification_url }` |
| `oauth-success` (`:33`) | providerId | `OAuthSuccess { provider_id }` |
| `oauth-error` (`:34`) | providerId, error | `OAuthError { provider_id, error }` |
| `oauth-headless-code-entry` (`:35-41`) | providerId, authorizeUrl, pkceVerifier, codeInput | `OAuthHeadlessCodeEntry { provider_id, authorize_url, pkce_verifier, code_input }` |
| `oauth-deployment-type-select` (`:48-52`) | providerId, selectedIndex:0\|1 | `OAuthDeploymentTypeSelect { provider_id, selected_index }` |
| `oauth-enterprise-url-entry` (`:59-63`) | providerId, urlInput, validationError? | `OAuthEnterpriseUrlEntry { provider_id, url_input, validation_error }` |

> These **replace** the dead-end `DetailSub::OAuthNotice` (`mod.rs:69`, `detail.rs:242-251`, `footer_hints.rs:39`, `list.rs:107`). `mapToEffectivePanelMode` is the single translation point — Rust mirror is the view-level mode→renderer selection.

---

## 7. Rust gap analysis — exists vs must-build (bottom-up)

### ✅ Already exists (parity-ready)
- **napi**: ALL OAuth flows for claude/codex/copilot/custom (browser, headless start/complete, device start/poll, clear, get, refresh, normalize-enterprise). See §9 for full signatures.
- **fspec-tui projection/nav/render**: OAuth row detection, labels, methods, status synthesis, `RowKind::OauthLogin`/`OauthStatus` rendering (`projection.rs`, `nav_item.rs build_nav_items`, `list_nav_render.rs`, `footer_hints.rs`).
- **wiring template**: RPC-054 / PROV-109 spawn→backend→`ProviderCredentialsLoaded` refresh loop (`dispatch_provider_settings_profiles.rs:26-110`).
- **rpc-types**: `ProviderCredentialInput::oauth(token, refresh)` shape (`rpc-types/src/lib.rs:521`).

### ❌ Must be added (the wiring gap)
1. **napi→core bridge / OAuth persistence.** napi OAuth fns bypass core. Either call napi directly from a new embedded path, or replicate flows on `SessionManagerHandle`. The deferred OAuth persistence at `handle_impl.rs:1267-1277` ("oauth persistence is a follow-up; input accepted") and `session_manager_handle.rs:2045-2047` must be implemented if going through the handle.
2. **rpc-types**: new flow types — `OAuthDeviceStart { user_code, verification_url, device_code/device_auth_id, interval, host_url?, enterprise_host? }`, `OAuthHeadlessStart { authorize_url, pkce_verifier }`, plus token/credential result types or reuse of existing `ProviderCredentialInfo`.
3. **rpc `FspecService`** (`rpc/src/lib.rs` — currently ZERO oauth methods): new trait methods + impls dispatched by `provider_id`: `oauth_browser_login`, `oauth_headless_start`, `oauth_headless_complete`, `oauth_device_start`, `oauth_device_poll`, `oauth_clear_tokens`, `oauth_get_tokens` (+ copilot enterprise variants & `normalize_enterprise_domain`).
4. **`FspecBackend` trait + both backends** (`transport/mod.rs` defaults; `embedded.rs:607-658`; `websocket.rs:949-1004`): matching async methods with no-op defaults + one-line forwarders.
5. **Action enum** (`components/mod.rs:597-730`): new variants — `OAuthBrowserLogin{provider_id}`, `OAuthHeadlessStart{provider_id}`, `OAuthHeadlessComplete{provider_id, code}`, `OAuthDeviceStart{provider_id, enterprise_url?}`, `OAuthDevicePoll{...}`, `OAuthDisconnect{provider_id}`, plus fold variants `OAuthDeviceStarted{...}`, `OAuthLoginComplete{provider_id}`, `OAuthLoginFailed{provider_id, error}`.
6. **Dispatch handlers**: new `handle_oauth_*` helpers + router arms in `try_dispatch_provider_settings` (or a new `dispatch_provider_settings_oauth.rs` sibling mirroring the profiles split). Each spawns the backend call and ends with `list_provider_credentials` → `ProviderCredentialsLoaded` refresh.
7. **`ProviderSettingsMode` variants + sub-handlers + renderers** (§6) replacing `DetailSub::OAuthNotice`.
8. **`list_actions.rs` routing**: Enter on OAuthLogin → browser/headless/copilot per §2.1; Enter/`d` on OAuthStatus → `DisconnectOAuth` confirm per §2.2/§2.3 (NOT the api-key delete confirm).

---

## 8. Open parity decisions (resolve during Example Mapping)

1. **Login row label strings.** TS uses `Login with Claude (browser)` / `(headless)` etc. (§1.2); current Rust `projection.rs:43-66` uses `Sign in with browser` / `Sign in with code` / `Sign in with device code`. Decide: adopt TS strings exactly (recommended for parity) OR keep Rust strings and document divergence. Affects nav-row tests.
2. **Backend path: through-core vs napi-direct.** Either implement OAuth persistence on `SessionManagerHandle`/RPC (heavier, satisfies the `handle_impl.rs:1267-1277` follow-up) or call napi OAuth fns directly from the embedded transport. Decide before estimating — this is the dominant cost driver and affects whether websocket transport can do OAuth at all.
3. **Browser/headless availability per transport.** Browser login binds a local HTTP server inside napi; over the websocket transport the server runs on the backend host, not the user's machine. Decide whether browser login is embedded-only and headless/device is the websocket path (likely), and gate the UI accordingly.
4. **`custom_oauth` providers.** The TS TUI hosts the loopback/browser for scripted providers (`custom_oauth.rs:40-42` note). Decide whether PROV-105 covers custom/Rhai OAuth providers or only the three built-ins (anthropic/codex/github-copilot). Recommend: built-ins only; custom is a follow-up.
5. **Splitting the card.** Browser + headless + device + disconnect across 3 providers + 4 backend layers is almost certainly > 13 points. Likely split: (a) backend/RPC/transport OAuth surface + disconnect; (b) anthropic+codex login wiring (browser+headless); (c) github-copilot device flow (deployment/enterprise preamble). Re-estimate after Example Mapping.

---

## 9. napi OAuth backend reference (already implemented — call targets)

### `claude_oauth.rs` (Anthropic)
| Fn | Signature | Returns |
| --- | --- | --- |
| `claude_oauth_browser_login` | `async fn()` (:88) | `Result<NapiClaudeTokens>` |
| `claude_oauth_headless_start` | `fn()` sync (:149) | `NapiClaudeHeadlessStartResult` |
| `claude_oauth_headless_complete` | `async fn(code_with_state: String, pkce_verifier: String)` (:169) | `Result<NapiClaudeTokens>` |
| `claude_oauth_refresh_token` | `async fn(refresh_token: String)` (:213) | `Result<NapiClaudeTokens>` |
| `claude_oauth_get_tokens` | `async fn()` (:234) | `Result<Option<NapiClaudeTokens>>` |
| `claude_oauth_clear_tokens` | `async fn()` (:256) | `Result<()>` (deletes `claude_auth.json`, idempotent) |

- `NapiClaudeTokens` (:41-47): `access_token, refresh_token: String; expires: f64`.
- `NapiClaudeHeadlessStartResult` (:64-68): `authorize_url, pkce_verifier: String`.

### `codex_oauth.rs` (ChatGPT/Codex)
| Fn | Signature | Returns |
| --- | --- | --- |
| `codex_oauth_browser_login` | `async fn()` (:121) | `Result<NapiCodexTokens>` |
| `codex_oauth_device_login_start` | `async fn()` (:142) | `Result<NapiDeviceAuthStartResult>` |
| `codex_oauth_device_login_poll` | `async fn(device_auth_id: String, interval: f64)` (:167) | `Result<NapiCodexTokens>` |
| `codex_oauth_refresh_token` | `async fn(refresh_token: String)` (:225) | `Result<NapiCodexTokens>` |
| `codex_oauth_get_tokens` | `fn()` sync (:245) | `Result<Option<NapiCodexTokens>>` |
| `codex_oauth_clear_tokens` | `fn()` sync (:264) | `Result<()>` (preserves cached `OPENAI_API_KEY`) |

- `NapiCodexTokens` (:34-40): `id_token, access_token, refresh_token, account_id: String`.
- `NapiDeviceAuthStartResult` (:58-64): `user_code, verification_url, device_auth_id: String; interval: f64`.

### `copilot_oauth.rs` (GitHub Copilot — device-only, enterprise-aware)
| Fn | Signature | Returns |
| --- | --- | --- |
| `copilot_oauth_device_login_start` | `async fn(enterprise_url: Option<String>)` (:111) | `Result<NapiCopilotDeviceStartResult>` |
| `copilot_oauth_device_login_poll` | `async fn(device_code: String, interval: f64, host_url: String, enterprise_host: Option<String>)` (:152) | `Result<NapiCopilotCredential>` |
| `copilot_oauth_get_credential` | `async fn()` (:206) | `Result<Option<NapiCopilotCredential>>` |
| `copilot_oauth_clear_credential` | `async fn()` (:217) | `Result<()>` (idempotent) |
| `copilot_normalize_enterprise_domain` | `fn(input: String)` sync pure (:233) | `String` |

- `NapiCopilotCredential` (:53-62): `access_token, refresh_token: String; expires: f64 (0=never); enterprise_url: Option<String>`.
- `NapiCopilotDeviceStartResult` (:81-95): `user_code, verification_url, device_code: String; interval: f64; host_url, deployment_type: String; enterprise_host: Option<String>`.

### `custom_oauth.rs` (scripted/Rhai — PROV-088; likely out of scope, see §8.4)
`custom_oauth_authorize(provider_name)` → `NapiCustomAuthorizeResult{payload_json}`; `custom_oauth_exchange(provider_name, code, verifier)` → `NapiCustomTokens{tokens_json}`; `custom_oauth_needs_refresh`, `custom_oauth_refresh`, `custom_oauth_clear`, `custom_oauth_get_tokens`, `custom_oauth_device_start`, `custom_oauth_device_poll(provider_name, device_data_json)`. Custom flows expect the host (TUI) to run the loopback callback / open browser.

---

## 10. ACDD test strategy (offline — gate constraint)

- **No real OAuth network.** Inject the token-exchange boundary via a path-injectable / faked transport (a `MockBackend` implementing the new `FspecBackend` OAuth methods with call counters + scripted Ok/Err), mirroring the PROV-109 `provider_settings_profile_dispatch` test (`provider_settings_profile_dispatch_prov109.rs`).
- **View/key tests**: construct `KeyEvent`s directly, drive `ProviderSettingsView::handle_key` through every mode in §6, assert resulting mode transitions and emitted `Action`s. No env mutation, no real `~/.fspec`.
- **Dispatch tests**: drive `App::dispatch` with the new OAuth actions against the MockBackend; assert it spawns the backend call, then on Ok emits `ProviderSettingsStatus` + re-fetches `list_provider_credentials` → `ProviderCredentialsLoaded`; on Err emits an error status WITHOUT leaking the RPC name (parity with PROV-109 error-path assertion).
- **Disconnect tests**: assert `y` → backend `oauth_clear`(provider) called exactly once + refresh; `n`/`Esc` → no backend call, return to list; any other key consumed.
- **Generation/stale-cancel**: model the TS `oauthGeneration` invalidation — cancelling (Esc) during waiting must drop a late-arriving Ok/Err without changing mode.
- **Parity assertions**: exact label/title/hint strings from §1.5, §3.3, §4.1-4.3, §5; exact nav ordering from §1.4.

## 11. Constraints / gates (from scope note)
- Strict 100% ACDD: feature file → failing tests (`@step` mapped) → impl.
- Files < 300 LoC; `cargo clippy -D warnings` clean; `cargo fmt` clean; build incl. downstream core+napi.
- **NO git** (user directive) — work directly in the working tree; never touch user WIP (`main.rs`, `session_manager.rs`).
- Parity verified against every `file:line` cited above.

---

## 12. Source reference index

**TypeScript:** `src/tui/inputHandlers/listModeHandler.ts` (Enter/`d` dispatch); `oauthModeHandler.ts`, `copilotOauthModeHandler.ts` (key handling); `src/tui/hooks/useProviderSettingsState.ts` (state machine, reload, disconnect, retry/cancel); `src/tui/utils/{copilotLoginFlow,copilotLoginDispatch,oauthLoginLabels,oauthProviderLabels,providerSettingsModeMapper}.ts`; `src/tui/types/settingsMode.ts`; `src/tui/components/{ProviderSettingsPanel,CopilotOauthRender}.tsx`; `src/tui/inputHandlers/deleteConfirmModeHandler.ts`; `src/tui/services/cloudSectionBuilder.ts`. Tests: `src/tui/__tests__/{provider-settings-oauth-logout,provider-settings-oauth-guards,oauth-tui-broken-flows,anthropic-oauth-tui,claude-oauth-napi-e2e}.test.ts`, `src/tui/inputHandlers/__tests__/listModeHandler-codex-oauth.test.ts`.

**Rust (current):** `codelet/fspec-tui/src/views/provider_settings/{list_actions,mod,detail,projection,nav_item,list,list_nav_render,footer_hints,row_render}.rs`; `codelet/fspec-tui/src/app/{dispatch_provider_settings,dispatch_provider_settings_profiles}.rs`; `codelet/fspec-tui/src/components/mod.rs` (Action enum); `codelet/fspec-tui/src/transport/{mod,embedded,websocket}.rs`. **Rust (napi backend):** `codelet/napi/src/{claude,codex,copilot,custom}_oauth.rs`. **Boundary gaps:** `codelet/rpc/src/lib.rs` (FspecService — no OAuth), `codelet/rpc-types/src/lib.rs` (ProviderCredentialInput::oauth only), `codelet/core/src/session_manager_handle.rs:2045-2047`, `codelet/sessions/src/handle_impl.rs:1267-1277` (deferred OAuth persistence).
