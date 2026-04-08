# PROV-053 Slice 1: OAuth Device Flow & Token Storage

**Author:** Provider Integration Engineer (subordinate agent)
**Date:** 2026-04-07
**Scope:** OAuth device flow + persistent credential storage for GitHub Copilot.
**Explicitly out of scope:** SDK/fetch wrapper, model catalog, `chat.params`/`chat.headers` hooks, endpoint routing.

---

## 1. Reference Behavior (opencode)

All paths in this section are rooted at `/tmp/opencode/packages/opencode/src/`.

### 1.1 Constants and URL builder — `plugin/github-copilot/copilot.ts`

```ts
11: const CLIENT_ID = "Ov23li8tweQw6odWQebz"
14: const OAUTH_POLLING_SAFETY_MARGIN_MS = 3000   // +3 s on every poll sleep

19: function getUrls(domain: string) {
20:   return {
21:     DEVICE_CODE_URL:  `https://${domain}/login/device/code`,
22:     ACCESS_TOKEN_URL: `https://${domain}/login/oauth/access_token`,
23:   }
24: }
```

The `domain` is resolved to `"github.com"` for public, or the normalized enterprise domain (`copilot.ts:197-202`). Normalization strips `https?://` and trailing `/`.

### 1.2 User prompts (verbatim, `copilot.ts:158-193`)

```ts
158: prompts: [
159:   { type: "select", key: "deploymentType",
162:     message: "Select GitHub deployment type",
163:     options: [
165:       { label: "GitHub.com",        value: "github.com", hint: "Public" },
170:       { label: "GitHub Enterprise", value: "enterprise", hint: "Data residency or self-hosted" },
174:     ],
175:   },
176:   { type: "text", key: "enterpriseUrl",
179:     message:     "Enter your GitHub Enterprise URL or domain",
180:     placeholder: "company.ghe.com or https://company.ghe.com",
181:     when: { key: "deploymentType", op: "eq", value: "enterprise" },
182:     validate: (value) => {
184:       if (!value) return "URL or domain is required"
185:       try {
186:         const url = value.includes("://") ? new URL(value) : new URL(`https://${value}`)
187:         if (!url.hostname) return "Please enter a valid URL or domain"
188:         return undefined
189:       } catch {
190:         return "Please enter a valid URL (e.g., company.ghe.com or https://company.ghe.com)"
191:       }
192:     },
193:   },
194: ],
```

The `when` condition is evaluated in `cli/cmd/providers.ts:47-72`:

```ts
47: if (method.prompts) {
48:   for (const prompt of method.prompts) {
49:     if (prompt.when) {
50:       const value = inputs[prompt.when.key]
51:       if (value === undefined) continue
52:       const matches = prompt.when.op === "eq" ? value === prompt.when.value : value !== prompt.when.value
53:       if (!matches) continue
54:     }
55:     ...
56:     if (prompt.type === "select") { ... await prompts.select(...) ... }
57:     else                          { ... await prompts.text(...)   ... }
72:   }
73: }
```

**Semantics:** `inputs` accumulates answers in declaration order. A `when` that references a later-declared key always skips (`value === undefined` → `continue`). Operator is binary: `"eq"` (strict equality) or anything else (strict inequality). So the select MUST be declared before the conditional text prompt.

### 1.3 Device code POST (`copilot.ts:206-228`)

```ts
206: const deviceResponse = await fetch(urls.DEVICE_CODE_URL, {
207:   method: "POST",
208:   headers: {
209:     Accept: "application/json",
210:     "Content-Type": "application/json",
211:     "User-Agent": `opencode/${Installation.VERSION}`,
212:   },
213:   body: JSON.stringify({
214:     client_id: CLIENT_ID,
215:     scope: "read:user",
216:   }),
217: })
219: if (!deviceResponse.ok) { throw new Error("Failed to initiate device authorization") }
223: const deviceData = (await deviceResponse.json()) as {
224:   verification_uri: string
225:   user_code:        string
226:   device_code:      string
227:   interval:         number
228: }
```

Scope is **only** `read:user` — no `copilot` scope.

### 1.4 Authorize descriptor returned to CLI (`copilot.ts:230-234`)

```ts
230: return {
231:   url:          deviceData.verification_uri,
232:   instructions: `Enter code: ${deviceData.user_code}`,
233:   method:       "auto" as const,
234:   async callback() { /* polling loop, 235-305 */ },
235: }
```

`method: "auto"` tells the CLI to drive the polling loop itself.

### 1.5 Polling loop (`copilot.ts:235-305`)

```ts
235: while (true) {
236:   const response = await fetch(urls.ACCESS_TOKEN_URL, {
237:     method: "POST",
238:     headers: {
239:       Accept: "application/json",
240:       "Content-Type": "application/json",
241:       "User-Agent": `opencode/${Installation.VERSION}`,
242:     },
243:     body: JSON.stringify({
244:       client_id:   CLIENT_ID,
245:       device_code: deviceData.device_code,
246:       grant_type:  "urn:ietf:params:oauth:grant-type:device_code",
247:     }),
248:   })
250:   if (!response.ok) return { type: "failed" as const }
252:   const data = (await response.json()) as {
253:     access_token?: string
254:     error?:        string
255:     interval?:     number
256:   }
258:   if (data.access_token) {
259:     const result: { ... } = {
267:       type:    "success",
268:       refresh: data.access_token,
269:       access:  data.access_token,
270:       expires: 0,
271:     }
273:     if (deploymentType === "enterprise") { result.enterpriseUrl = domain }
277:     return result
278:   }
280:   if (data.error === "authorization_pending") {
281:     await sleep(deviceData.interval * 1000 + OAUTH_POLLING_SAFETY_MARGIN_MS)
282:     continue
283:   }
285:   if (data.error === "slow_down") {
286:     // RFC 8628 §3.5: +5 s
288:     let newInterval = (deviceData.interval + 5) * 1000
292:     const serverInterval = data.interval
293:     if (serverInterval && typeof serverInterval === "number" && serverInterval > 0) {
294:       newInterval = serverInterval * 1000
295:     }
297:     await sleep(newInterval + OAUTH_POLLING_SAFETY_MARGIN_MS)
298:     continue
299:   }
301:   if (data.error) return { type: "failed" as const }
303:   await sleep(deviceData.interval * 1000 + OAUTH_POLLING_SAFETY_MARGIN_MS)
305: }
```

**Observations:**

- `OAUTH_POLLING_SAFETY_MARGIN_MS = 3000` is added to **every** sleep (lines 281, 297, 303).
- `slow_down` handling follows RFC 8628 §3.5: default `interval + 5` seconds, but if the server supplies a fresh `interval` in the `slow_down` response, that value wins (lines 292-295).
- Any unrecognized error is terminal (`line 301`). Defensive fall-through at line 303 sleeps + retries (shouldn't happen against a compliant server).
- Network failures are terminal (`line 250`).

### 1.6 Why `refresh === access` and `expires === 0`

Verbatim (`copilot.ts:258-278`):

```ts
258: if (data.access_token) {
266:   const result = {
267:     type:    "success",
268:     refresh: data.access_token,   // SAME
269:     access:  data.access_token,   // SAME
270:     expires: 0,                   // sentinel
271:   }
272:   if (deploymentType === "enterprise") result.enterpriseUrl = domain
277:   return result
278: }
```

- GitHub OAuth Apps device flow returns a **single long-lived `gho_*` user-to-server token** with no refresh token. OAuth Apps tokens do not expire and there is no rotation endpoint (unlike GitHub Apps).
- The schema (`auth/index.ts:15-22`) mandates both `refresh: Schema.String` and `access: Schema.String`, so the same token fills both slots.
- `expires: 0` is a sentinel ("no expiry tracked"). The schema requires `expires: Schema.Number`, so `0` is the cheapest valid placeholder.
- **No code path ever reads `info.expires`.** The fetch wrapper unconditionally sends `Bearer ${info.refresh}` on every request (`copilot.ts:136`), and the models endpoint does the same (`copilot.ts:53`).
- **No refresh logic exists.** No `refresh_token` grant, no expiry comparison, no re-authorize from inside the fetch wrapper, no background timer. When a token becomes invalid, API calls 401 and the user must re-run `opencode auth login`.
- Naming oddity: the field is called `refresh` but it functions as the bearer credential.

### 1.7 Per-request credential re-read (`copilot.ts:63-153`)

```ts
63: auth: {
64:   provider: "github-copilot",
65:   async loader(getAuth) {
66:     const info = await getAuth()                    // (A) init-time read
67:     if (!info || info.type !== "oauth") return {}
69:     const baseURL = base(info.enterpriseUrl)
71:     return {
72:       baseURL,
73:       apiKey: "",
74:       async fetch(request, init) {
75:         const info = await getAuth()                // (B) per-request re-read
76:         if (info.type !== "oauth") return fetch(request, init)
...
136:         Authorization: `Bearer ${info.refresh}`,
...
147:         return fetch(request, { ...init, headers })
151:       },
152:     }
153:   },
```

**Two distinct reads:**
- **(A) Line 66** — once at loader-init, used to compute `baseURL` from the persisted `enterpriseUrl` and to bail out when no OAuth credential exists.
- **(B) Line 75** — re-read on **every** outbound fetch. The `const info` shadows the outer binding.

**Hot-swap implication:** If `auth.json` is updated externally — e.g. a concurrent `opencode auth login` in another terminal — the next API call picks up the new token automatically, because `Auth.get()` reads the file fresh each time (`auth/index.ts:65-67` has no cache). No daemon restart required. The enterprise `baseURL`, however, is frozen at loader init — switching `github.com` ↔ `enterprise` **does** require restart.

### 1.8 On-disk credential schema (`auth/index.ts`)

```ts
10: const file = path.join(Global.Path.data, "auth.json")

15: export class Oauth extends Schema.Class<Oauth>("OAuth")({
16:   type:          Schema.Literal("oauth"),
17:   refresh:       Schema.String,
18:   access:        Schema.String,
19:   expires:       Schema.Number,
20:   accountId:     Schema.optional(Schema.String),
21:   enterpriseUrl: Schema.optional(Schema.String),
22: }) {}

36: const _Info = Schema.Union([Oauth, Api, WellKnown]).annotate({ discriminator: "type", identifier: "Auth" })
```

- **File location:** `<Global.Path.data>/auth.json` (XDG-style; e.g. `~/.local/share/opencode/auth.json` on Linux). The CLI displays this path with HOME collapsed to `~` (`providers.ts:214-216`).
- **Keying:** by provider id string. For Copilot the key is the literal `"github-copilot"` (`copilot.ts:64` → `providers.ts:93-96`).
- **Key normalization:** trailing `/` stripped (`auth/index.ts:70`); this matters only for URL-shaped keys (WellKnown providers).
- **File permissions:** `0o600` enforced on every write (`auth/index.ts:75, 84`).

Operations:
- `all` (60-63): reads file, decodes each entry; dropped entries on decode failure.
- `get` (65-67): `all()[providerID]` — **re-reads file on every call, no cache**.
- `set` (69-77): normalizes key, deletes stale variants, writes at mode 0600.
- `remove` (79-85): deletes both raw and normalized keys.

### 1.9 CLI dispatch (`cli/cmd/providers.ts`)

```ts
254: export const ProvidersLoginCommand = cmd({
255:   command: "login [url]",
...
326: "github-copilot": 2,   // priority slot in the picker
...
390: const plugin = await Plugin.list().then((x) =>
391:   x.findLast((x) => x.auth?.provider === provider))
392: if (plugin && plugin.auth) {
393:   const handled = await handlePluginAuth({ auth: plugin.auth }, provider, args.method)
394:   if (handled) return
395: }
```

Inside `handlePluginAuth` (lines 19-170):
1. Method selection (19-43): Copilot has only one method → index 0.
2. Prompt loop (46-73): drives the `select` then the conditional `text`.
3. OAuth branch (75-148):
   - Calls `method.authorize(inputs)` → device code request.
   - Prints `Go to: <verification_uri>` and `Enter code: <user_code>`.
   - `prompts.spinner()` labeled `"Waiting for authorization..."`.
   - `await authorize.callback()` → runs the polling loop.
   - On success: `await Auth.set(saveProvider, { type: "oauth", refresh, access, expires, ...extra })` at line 96.

---

## 2. fspec Current State

### 2.1 Where provider/OAuth state lives — a three-layer split

fspec has **NO** `src/auth/` or `src/providers/` directory. OAuth and credentials are spread across three layers:

| Layer | Files | Responsibility |
|---|---|---|
| **TS TUI** | `src/tui/hooks/useProviderSettingsState.ts`, `src/tui/inputHandlers/oauthModeHandler.ts`, `src/tui/inputHandlers/listModeHandler.ts`, `src/tui/types/settingsMode.ts`, `src/tui/components/ProviderSettingsPanel.tsx` | UI mode state, input routing, nav items |
| **TS utils** | `src/utils/credentials.ts`, `src/utils/provider-config.ts`, `src/utils/config.ts` | Provider registry, on-disk **API-key** store, env var resolution |
| **Rust NAPI** | `codelet/providers/src/claude_oauth.rs`, `codelet/providers/src/codex/codex_oauth_server.rs`, `codelet/providers/src/codex/codex_device_auth.rs`, `codelet/providers/src/claude_auth.rs`, `codelet/providers/src/codex/codex_auth.rs` | Actual OAuth flows, token exchange, token persistence |

The TS layer deliberately has **no OAuth code of its own** — it dispatches to NAPI functions.

### 2.2 TS types (no Copilot-relevant shapes exist)

**`src/tui/types/provider.ts`** defines display types only:
- Line 8-12: imports `ProfileConfig` from `../../utils/provider-config` and re-exports `NapiModelInfo`.
- Lines 18-30: `ProviderSection` with `hasCredentials`, `profileName?`, `profileConfig?`, `isUnreachable?`.
- Lines 108-147: `ProfileFormField` and `PROFILE_FORM_FIELDS` — a form for local-LLM profiles (openai only).

No OAuth shapes live here.

**`src/tui/constants/providerSettings.ts`** (27 lines) is profile-UI constants only:
```ts
12: export const PROFILE_FORM_FIELDS: Array<keyof ProfileConfig> = ['baseUrl','apiKey','contextWindow','maxOutputTokens'];
22: export const DEFAULT_PROFILE_BASE_URL = 'http://localhost:8888';
27: export const SETTINGS_PANEL_CHROME_HEIGHT = 6;
```

**No per-provider registry here.** The actual registry is `PROVIDER_REGISTRY` in `src/utils/provider-config.ts:101-264` — 16 providers, each with `authType: 'api-key' | 'oauth'`. Only `anthropic` (line 119) and `codex` (line 260) have `authType: 'oauth'`.

**`src/tui/types/settingsMode.ts`** (41 lines) is the hook-side mode enum:
```ts
18: export type HookMode =
19:   | { type: 'list' }
20:   | { type: 'edit-api-key';          providerId: string }
21:   | { type: 'delete-api-key';        providerId: string }
22:   | { type: 'disconnect-oauth';      providerId: string }
23:   | { type: 'create-profile';        providerId: string }
24:   | { type: 'edit-profile';          providerId: string; profileName: string }
25:   | { type: 'delete-profile';        providerId: string; profileName: string }
26:   | { type: 'oauth-browser-waiting'; providerId: string }
27:   | { type: 'oauth-device-waiting';  providerId: string; userCode: string; verificationUrl: string }
28:   | { type: 'oauth-success';         providerId: string }
29:   | { type: 'oauth-error';           providerId: string; error: string }
30:   | { type: 'oauth-headless-code-entry';
31:       providerId: string; authorizeUrl: string; pkceVerifier: string; codeInput: string }
```

**Critical for Copilot:** `oauth-device-waiting` already carries `userCode` + `verificationUrl`, which is exactly what GitHub's device flow displays. This mode is currently used only by Codex.

### 2.3 Existing OAuth handler (`src/tui/inputHandlers/oauthModeHandler.ts`, 117 lines)

The existing handler is a pure **state-machine input router** — zero OAuth work happens here. Structure:

- Lines 4-10 (JSDoc): supported modes = `oauth-browser-waiting`, `oauth-device-waiting`, `oauth-headless-code-entry`, `oauth-success`, `oauth-error`.
- Lines 21-25: signature `handleOauthMode(input, key, providerSettings): boolean`.
- Lines 29-38: `oauth-browser-waiting` / `oauth-device-waiting` — only Esc → `cancelOauth()`; all other input absorbed.
- Lines 41-94: `oauth-headless-code-entry` — Esc, Enter→`submitHeadlessCode(codeInput, pkceVerifier)`, Backspace, `c`→copy URL, `o`→open browser (PROV-028), printable char→append to `codeInput`.
- Lines 97-103: `oauth-success` — Enter/Esc return to `{ type: 'list' }`.
- Lines 106-114: `oauth-error` — Enter→`retryOauth()`, Esc→`cancelOauth()`.

### 2.4 Existing OAuth orchestration (`src/tui/hooks/useProviderSettingsState.ts`)

Imports 10 NAPI OAuth functions (lines 25-38):
```ts
25: import {
...
29:   codexOauthGetTokens,
30:   codexOauthBrowserLogin,
31:   codexOauthDeviceLoginStart,
32:   codexOauthDeviceLoginPoll,
33:   codexOauthClearTokens,
34:   claudeOauthBrowserLogin,
35:   claudeOauthHeadlessStart,
36:   claudeOauthHeadlessComplete,
37:   claudeOauthGetTokens,
38:   claudeOauthClearTokens,
```

**`startBrowserLogin(providerId)` (lines 551-581):**
```ts
551: const startBrowserLogin = useCallback((providerId: string) => {
...
556:   setMode({ type: 'oauth-browser-waiting', providerId });
...
559:   try {
560:     if (providerId === 'anthropic') { await claudeOauthBrowserLogin(); }
562:     else                            { await codexOauthBrowserLogin();  }
...
568:     setMode({ type: 'oauth-success', providerId });
569:     await reload();
570:   } catch (err) {
...
576:     setMode({ type: 'oauth-error', providerId, error: errorMsg });
577:   }
```

**`startDeviceLogin(providerId)` (lines 586-652)** — this is where Codex device flow + Anthropic headless both live:

```ts
586: const startDeviceLogin = useCallback((providerId: string) => {
...
592:   if (providerId === 'anthropic') {
593:     // headless: two-phase PKCE, synchronous start
595:     try {
596:       const result = claudeOauthHeadlessStart();
...
600:       setMode({ type: 'oauth-headless-code-entry', providerId,
601:                 authorizeUrl: result.authorizeUrl,
602:                 pkceVerifier: result.pkceVerifier, codeInput: '' });
...
614:   } else {
615:     // Codex device flow
617:     void (async () => {
618:       try {
619:         const result = await codexOauthDeviceLoginStart();
...
623:         setMode({ type: 'oauth-device-waiting', providerId,
624:                   userCode:        result.userCode,
625:                   verificationUrl: result.verificationUrl });
630:         await codexOauthDeviceLoginPoll(result.deviceAuthId, result.interval);
...
638:         setMode({ type: 'oauth-success', providerId });
639:         await reload();
...
```

**The Copilot device flow will graft onto this exact shape** — two NAPI calls (`*Start` returning `{ userCode, verificationUrl, deviceAuthId, interval }`, then `*Poll`), with `setMode({ type: 'oauth-device-waiting', ... })` between them.

### 2.5 Nav item generation (`buildNavItems`, lines 128-185)

```ts
165: if (isOAuthProvider(provider.id)) {
166:   const isAnthropic = provider.id === 'anthropic';
167:   const browserLabel  = isAnthropic ? 'Login with Claude (browser)'  : 'Login with ChatGPT (browser)';
170:   const headlessLabel = isAnthropic ? 'Login with Claude (headless)' : 'Login with ChatGPT (headless)';
173:   items.push({ type: 'oauth-login', providerId: provider.id, method: 'browser',  label: browserLabel  });
179:   items.push({ type: 'oauth-login', providerId: provider.id, method: 'headless', label: headlessLabel });
185: }
```

**Hard-coded `isAnthropic` branch** — a new `github-copilot` provider would need a third label branch ("Login with GitHub" / "Login with GitHub (device)") OR a data-driven labels table. The method names (`browser` / `headless`) also don't map cleanly onto Copilot's device flow — Copilot's login is device-flow-only and browser-opening is just a convenience during waiting. **This is where the biggest divergence from the opencode reference will show up.**

### 2.6 List mode dispatch (`src/tui/inputHandlers/listModeHandler.ts:121-126`)

```ts
121: } else if (currentItem.type === 'oauth-login') {
122:   if (currentItem.method === 'browser') {
123:     providerSettings.startBrowserLogin(currentItem.providerId);
124:   } else if (currentItem.method === 'headless') {
125:     providerSettings.startDeviceLogin(currentItem.providerId);
126:   }
127: }
```

### 2.7 On-disk credential stores — three separate locations

fspec has **three coexisting credential stores**, none of them suitable as-is for Copilot:

**Store 1 — fspec API keys: `~/.fspec/credentials/credentials.json`**
Owner: `src/utils/credentials.ts`.
- Path: `getFspecUserDir() + '/credentials/credentials.json'` (line 87-89).
- Schema: `CredentialsFile { version: number; providers: Record<string, ProviderCredential> }` where `ProviderCredential = { apiKey: string; lastUpdated: string }` (lines 40-51).
- Permissions: **directory `0o700`** (line 137), **file `0o600`** (line 168). Explicit chmod.
- This is for **API keys only** — schema has no `refresh`/`access`/`expires`/`enterpriseUrl` fields.

**Store 2 — Codex OAuth tokens: `~/.codex/auth.json`**
Owner: `codelet/providers/src/codex/codex_auth.rs`.
- Path: `$CODEX_HOME/auth.json` or `$HOME/.codex/auth.json` (lines 49-61).
- Written via `fs::write` with `serde_json::to_string_pretty` (lines 125-137). **No explicit chmod** — relies on umask.
- macOS keychain support: `read_keychain_credentials` (lines 79-96) using `keyring::Entry` keyed by `cli|{sha256(canonical_path)[..16]}`.
- Inherited from the `codex` CLI — fspec reads the file written by the external `codex` tool and Codex re-login.

**Store 3 — Claude OAuth tokens: `~/.fspec/credentials/claude_auth.json`**
Owner: `codelet/providers/src/claude_auth.rs`.
- Path: `$FSPEC_HOME/claude_auth.json` or `$HOME/.fspec/credentials/claude_auth.json` (lines 29-43).
- Schema: `ClaudeAuthJson { access_token, refresh_token, expires: u64 (ms since epoch) }` (lines 21-27).
- Written via `tokio::fs::write` (lines 75-86). **No explicit chmod.**
- Comment at lines 6-7: _"Mirrors codex_auth.rs pattern but simpler — no keychain, no id_token, no account_id."_

**Observation for slice 1:** There is no single `auth.json` equivalent to opencode's one-file-for-all approach. Each OAuth provider owns its own file under its own directory. Copilot should follow this precedent — a dedicated `copilot_auth.json` is more idiomatic here than retrofitting the `credentials.json` store.

### 2.8 TUI prompts — select? conditional text? NO

**Search result:** there are **no** `select` components, no conditional text prompts, and no enterprise/deployment pickers anywhere in the TUI. Grep for `enterpriseUrl`, `enterprise_url`, `deployment` in `src/tui/` returns only unrelated matches (a `/loop` command test fixture). The only existing "form with fields" pattern is the openai-only profile form (`ProviderSettingsPanel.tsx:64-72`, `create-profile` / `edit-profile` HookMode variants).

Current OAuth providers (Anthropic, Codex) have **hard-coded endpoints in Rust** (`claude_oauth.rs:30-46`, `codex_oauth_server.rs:35-39`) and never prompt for custom URLs.

**Copilot will be the first provider needing a pre-OAuth configuration prompt.** Two viable patterns:

1. **New HookMode variant** `{ type: 'copilot-deployment-select'; providerId: 'github-copilot' }` + a new handler, followed by `{ type: 'copilot-enterprise-url-entry'; deploymentType: 'enterprise' }`. Mirrors the opencode two-prompt flow but adds two new modes.
2. **Reuse profile form shape** — create a "Copilot deployment" pseudo-profile with a custom field set. This would require removing the `providerId !== 'openai'` guard at `provider-config.ts:430-433`.

Option 1 is cleaner (no hidden profile-form semantics leak into OAuth). See §3.2.

### 2.9 Existing device-flow references

Grep for `device_code`, `device-code`, `urn:ietf:params:oauth:grant-type:device_code`, `deviceauth`:

- **Rust (Codex):** `codelet/providers/src/codex/codex_device_auth.rs` implements a **non-standard** device flow. The header comment (lines 1-12) shows:
  - `POST {ISSUER}/api/accounts/deviceauth/usercode` → `{ device_auth_id, user_code, interval }`
  - `POST {ISSUER}/api/accounts/deviceauth/token` with `device_auth_id` as form field
  - `SLOW_DOWN_INCREMENT_MS = 5_000` (line 79) per RFC 8628 §3.5
  - Type: `DeviceCodeResponse { device_auth_id, user_code, interval }` (lines 22-27)
- **Rust (Codex token-exchange):** `codex_auth.rs:190, 195` uses `urn:ietf:params:oauth:grant-type:token-exchange` and `urn:ietf:params:oauth:token-type:id_token` — **NOT** the device-code grant type Copilot needs.
- **TS:** zero references.

**Finding:** Copilot will be the **first RFC 8628-compliant** device flow in fspec. The Codex "device auth" is a custom OpenAI/ChatGPT flow (different endpoint names, different response shape, uses `device_auth_id` instead of `device_code`, polls with form body instead of JSON). **The Codex implementation is NOT a reusable skeleton** — it's a different protocol dialect.

---

## 3. Proposed fspec Design

### 3.1 File layout

**New files (Rust, owned by this slice):**

| Path | Purpose |
|---|---|
| `codelet/providers/src/copilot_auth.rs` | On-disk `CopilotAuthJson` schema + `get_copilot_auth_path()` + `read_copilot_auth()` / `write_copilot_auth()`. Mirrors `claude_auth.rs`. |
| `codelet/providers/src/copilot_oauth.rs` | `CopilotOauthConfig { deployment_type, enterprise_url }`, constants (`CLIENT_ID`, `OAUTH_POLLING_SAFETY_MARGIN_MS`, `getUrls`, `normalize_domain`, `base_api_url`), `request_device_code`, `poll_access_token`, and `device_oauth_login` orchestrator. |

**New NAPI bindings (Rust → TS bridge):**

| Path | New exports |
|---|---|
| `codelet/napi/src/copilot_oauth.rs` (new) | `copilotOauthDeviceLoginStart(config)`, `copilotOauthDeviceLoginPoll(deviceCode, interval, deploymentType, enterpriseUrl?)`, `copilotOauthGetTokens()`, `copilotOauthClearTokens()` |
| `codelet/napi/src/lib.rs` | Register the new module, re-export functions |

**TS files to extend:**

| Path | Change |
|---|---|
| `src/utils/provider-config.ts` | Add new `PROVIDER_REGISTRY` entry: `{ id: 'github-copilot', name: 'GitHub Copilot', baseUrl: 'https://api.githubcopilot.com', envVar: '', authMethod: 'bearer', authType: 'oauth', requiresApiKey: false, description: 'GitHub Copilot via OAuth device flow' }`. Extend `SUPPORTED_PROVIDERS` tuple. |
| `src/tui/types/settingsMode.ts` | Add two new `HookMode` variants: `{ type: 'copilot-deployment-select'; providerId: 'github-copilot' }` and `{ type: 'copilot-enterprise-url-entry'; providerId: 'github-copilot'; urlInput: string; error?: string }`. |
| `src/tui/inputHandlers/oauthModeHandler.ts` | Handle the two new modes (arrow-key select, text input, validators). |
| `src/tui/inputHandlers/listModeHandler.ts` | Expand the `oauth-login` dispatch: for `github-copilot`, `method==='browser'` opens the deployment select instead of calling `startBrowserLogin`. |
| `src/tui/hooks/useProviderSettingsState.ts` | Three new callbacks: `startCopilotLogin(providerId)`, `submitCopilotDeployment(choice)`, `submitCopilotEnterpriseUrl(url)`. Import new NAPI functions. Extend `buildNavItems` labels branch for `github-copilot`. |
| `src/tui/components/ProviderSettingsPanel.tsx` | Extend `PanelMode` union; add render branches for the two new modes. |

**Files explicitly NOT touched in this slice:**

- `src/utils/credentials.ts` — Copilot doesn't use the API-key store.
- Anything under `codelet/providers/src/codex/` or `claude_*` — Copilot gets its own module.
- `src/tui/constants/providerSettings.ts` — no new form fields.

### 3.2 TUI prompts — deployment select + conditional enterprise URL

Two new HookMode variants are simpler than shoehorning into the profile-form machinery. State transitions:

```
list
  └─[Enter on 'oauth-login' for github-copilot]→ copilot-deployment-select
         ├─[select 'github.com']→ oauth-device-waiting (immediate device flow start)
         └─[select 'enterprise']→ copilot-enterprise-url-entry
                └─[Enter with valid URL]→ oauth-device-waiting
         └─[Esc from either]→ list
oauth-device-waiting → oauth-success | oauth-error (existing)
```

**Mirrors opencode's `when` semantics without building a generic prompt engine.** The `when` condition is hard-coded as a state-transition branch. For fspec this is fine because Copilot is the only provider that needs it; when a second provider needs a similar flow, we can extract a generic `deployment-select-then-url` machine.

**Reuse the existing `oauth-device-waiting` mode unchanged** — its `{ userCode, verificationUrl }` shape is already exactly what GitHub returns (just with a different field name on the wire: `user_code`, `verification_uri`).

**Input handling:**
- `copilot-deployment-select`: Up/Down arrows, Enter, Esc. Keep current selection in mode state.
- `copilot-enterprise-url-entry`: printable chars append to `urlInput`, Backspace trims, Enter validates with `new URL(...)` (same logic as opencode lines 184-190), Esc cancels.

### 3.3 Token storage — new `copilot_auth.json`

**Path:** `$FSPEC_HOME/copilot_auth.json` or `$HOME/.fspec/credentials/copilot_auth.json`.

**Rationale:** symmetric with `claude_auth.json` (same directory, same Rust module shape). **Do NOT** retrofit `~/.fspec/credentials/credentials.json` — that store is API-key-only and doesn't have `refresh`/`access`/`expires`/`enterpriseUrl` fields.

**Schema (Rust):**

```rust
pub struct CopilotAuthJson {
    pub access_token: String,        // the gho_* bearer
    pub token_type: String,          // always "bearer" from GitHub
    pub scope: String,               // "read:user"
    pub enterprise_url: Option<String>,  // None for github.com
}
```

**Differences from opencode's schema:**
- Drop the `refresh === access` duplication. Copilot has no refresh token, so store just `access_token` under a single field.
- Drop `expires: 0` sentinel. It's never read.
- Rename field from `refresh` to `access_token` because fspec's precedent (`ClaudeAuthJson.access_token` at `claude_auth.rs:21-27`) uses that name.
- Keep `enterprise_url` only when the deployment is enterprise.

**Permissions:** `0o600` on the file. Unlike the existing Claude/Codex Rust modules, we should **explicitly chmod** here — missing chmod on those files is a latent hardening gap, and we shouldn't propagate it. Wrap the `tokio::fs::write` with a follow-up `set_permissions` call gated to Unix only.

**Directory creation:** reuse `get_fspec_home()` or add `get_fspec_home_copilot()`. Create with `0o700` for parity with `src/utils/credentials.ts:137`.

### 3.4 Device flow implementation — function signatures only

**`codelet/providers/src/copilot_oauth.rs`** (new):

```rust
pub const COPILOT_CLIENT_ID: &str = "Iv1.PLACEHOLDER_REGISTER_OWN_APP"; // see §4.1
pub const OAUTH_POLLING_SAFETY_MARGIN_MS: u64 = 3_000;
pub const COPILOT_OAUTH_SCOPE: &str = "read:user";

pub enum DeploymentType { GithubCom, Enterprise }

pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
    pub expires_in: u64,
}

pub struct CopilotOauthUrls { pub device_code_url: String, pub access_token_url: String }

pub fn normalize_domain(input: &str) -> String;
pub fn get_oauth_urls(deployment: &DeploymentType, enterprise_url: Option<&str>) -> CopilotOauthUrls;
pub fn base_api_url(deployment: &DeploymentType, enterprise_url: Option<&str>) -> String;

pub async fn request_device_code(urls: &CopilotOauthUrls) -> Result<DeviceCodeResponse>;
pub async fn poll_access_token(
    urls: &CopilotOauthUrls,
    device_code: &str,
    interval: u64,
    overall_timeout: Duration,
) -> Result<PollOutcome>;

pub enum PollOutcome {
    Success { access_token: String, token_type: String, scope: String },
    TerminalError { error: String },
}
```

**Orchestrator (two-phase to match fspec's existing TS pattern):**

```rust
// Phase 1: kicked from TS at startDeviceLogin
pub async fn device_login_start(
    deployment: DeploymentType,
    enterprise_url: Option<String>,
) -> Result<DeviceLoginStartResult>;

pub struct DeviceLoginStartResult {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub interval: u64,
}

// Phase 2: awaited by TS after UI shows the code
pub async fn device_login_poll(
    device_code: String,
    interval: u64,
    deployment: DeploymentType,
    enterprise_url: Option<String>,
) -> Result<()>; // on success, writes copilot_auth.json via write_copilot_auth()
```

**Why two phases:** mirrors `codexOauthDeviceLoginStart` / `codexOauthDeviceLoginPoll`. The TS hook needs the intermediate `{ userCode, verificationUrl }` before the poll begins, so the UI can render `oauth-device-waiting` before the polling future resolves.

**NAPI surface (`codelet/napi/src/copilot_oauth.rs`):**

```rust
#[napi(object)] pub struct NapiCopilotDeviceStartResult { /* camelCase fields */ }
#[napi] pub async fn copilot_oauth_device_login_start(
    deployment_type: String,            // "github.com" | "enterprise"
    enterprise_url: Option<String>,
) -> Result<NapiCopilotDeviceStartResult>;
#[napi] pub async fn copilot_oauth_device_login_poll(
    device_code: String,
    interval: u32,
    deployment_type: String,
    enterprise_url: Option<String>,
) -> Result<()>;
#[napi] pub fn copilot_oauth_get_tokens() -> Result<Option<NapiCopilotTokens>>;
#[napi] pub async fn copilot_oauth_clear_tokens() -> Result<()>;
```

Exposed to TS as camelCase: `copilotOauthDeviceLoginStart`, `copilotOauthDeviceLoginPoll`, `copilotOauthGetTokens`, `copilotOauthClearTokens`.

### 3.5 Wiring into existing TS OAuth handling

**`useProviderSettingsState.ts`:**

```ts
// New callbacks (sketch, not final):
const startCopilotLogin = useCallback((providerId: 'github-copilot') => {
  setMode({ type: 'copilot-deployment-select', providerId });
}, []);

const submitCopilotDeployment = useCallback((choice: 'github.com' | 'enterprise') => {
  if (choice === 'github.com') {
    void runCopilotDeviceFlow('github.com', undefined);
  } else {
    setMode({
      type: 'copilot-enterprise-url-entry',
      providerId: 'github-copilot',
      urlInput: '',
    });
  }
}, []);

const submitCopilotEnterpriseUrl = useCallback((url: string) => {
  // validate using same logic as opencode: new URL(url) or new URL('https://' + url)
  void runCopilotDeviceFlow('enterprise', normalized);
}, []);

// Shared body (mirrors startDeviceLogin's Codex branch):
async function runCopilotDeviceFlow(deploymentType, enterpriseUrl) {
  const started = await copilotOauthDeviceLoginStart(deploymentType, enterpriseUrl);
  setMode({ type: 'oauth-device-waiting', providerId: 'github-copilot',
            userCode: started.userCode, verificationUrl: started.verificationUrl });
  await copilotOauthDeviceLoginPoll(started.deviceCode, started.interval, deploymentType, enterpriseUrl);
  setMode({ type: 'oauth-success', providerId: 'github-copilot' });
  await reload();
}
```

**`listModeHandler.ts`** (lines 121-126) must learn that Copilot takes the deployment-select path instead of the direct `startBrowserLogin`/`startDeviceLogin` call:

```ts
} else if (currentItem.type === 'oauth-login') {
  if (currentItem.providerId === 'github-copilot') {
    providerSettings.startCopilotLogin('github-copilot');
  } else if (currentItem.method === 'browser') {
    providerSettings.startBrowserLogin(currentItem.providerId);
  } else if (currentItem.method === 'headless') {
    providerSettings.startDeviceLogin(currentItem.providerId);
  }
}
```

**`buildNavItems`** (lines 165-185): add a third branch. For Copilot, emit **one** login item (`label: 'Login with GitHub Copilot'`) rather than browser+headless pair — because device flow is the only method, there's no user-meaningful choice between two variants.

**`oauthModeHandler.ts`**: add two new cases (one for `copilot-deployment-select`, one for `copilot-enterprise-url-entry`) using the existing `return true` absorb-all pattern.

### 3.6 Hot-swap / re-read semantics

opencode re-reads `auth.json` on every API call (§1.7, line 75). fspec's equivalent would be `copilot_oauth_get_tokens()` called from the Rust fetch wrapper on every request (**slice 2 territory** — not this slice). For slice 1, it suffices that `write_copilot_auth()` writes atomically and that `read_copilot_auth()` has no in-memory cache. Use the same `tokio::fs::write` + `serde_json` pattern as `claude_auth.rs:75-86`.

---

## 4. Open Questions for the Product Owner

These are the red cards that should drive Example Mapping for the slice-1 work unit.

### 4.1 GitHub OAuth App identity

opencode hard-codes `CLIENT_ID = "Ov23li8tweQw6odWQebz"` (`copilot.ts:11`). **Do we:**
- **(a)** Register our own GitHub OAuth App under an `fspec` or `sengac` org and ship its ID?
- **(b)** Reuse opencode's public client ID? (legally questionable — couples our TOS to theirs, risks revocation)
- **(c)** Defer to a runtime config file so the user brings their own app ID?

Option (a) is the safe path but requires an out-of-band GitHub org action before this work unit can ship.

### 4.2 Enterprise URL normalization — accept domain or full URL?

opencode accepts both (`copilot.ts:184-190`). Do we match this permissively, or require a full `https://` URL? Permissive matching adds validator complexity and an extra branch in `base_api_url()` but is more user-friendly.

### 4.3 Token storage location — `~/.fspec/credentials/` or `~/.fspec/`?

Two precedents exist in the codebase:
- `~/.fspec/credentials/claude_auth.json` (`claude_auth.rs:29-43`) — for Claude OAuth.
- `~/.fspec/credentials/credentials.json` (`src/utils/credentials.ts:87-89`) — for API keys.

Should Copilot go under `~/.fspec/credentials/copilot_auth.json` (consistency with Claude, since both are NAPI-owned) or `~/.fspec/copilot_auth.json` (consistency with the fspec-prefixed TS store)? **Recommend the former** — keeps all OAuth-token files in one directory and matches the nearest-precedent Rust module (`claude_auth.rs`).

### 4.4 macOS keychain for Copilot tokens?

`codex_auth.rs:79-96` wires Codex tokens into the macOS keychain via the `keyring` crate; `claude_auth.rs` does not. The keychain is strictly better for at-rest security but adds a platform dependency and more code. **Should slice 1 include keychain support or defer to a follow-up hardening ticket?**

### 4.5 File permissions — tighten beyond existing precedent?

`claude_auth.rs:75-86` and `codex_auth.rs:125-137` **do not chmod** their output files — they rely on umask. `src/utils/credentials.ts:168` explicitly chmods to `0o600`. **Should Copilot's file-write path explicitly chmod 0600 (recommended) even though existing Rust stores don't?**

### 4.6 Overall timeout on device polling

opencode polls **forever** (infinite `while (true)`) — the only exit is Esc from the CLI or a terminal server error. The Codex device flow in fspec (`codex_device_auth.rs:45-49`) has an explicit `timeout_ms`. GitHub's `expires_in` (typically 900 s = 15 min) tells us when the device code dies. **Should we enforce a `Duration::from_secs(15 * 60)` client-side timeout, or poll until the server returns `expired_token`?**

### 4.7 Single login item or browser+headless pair in the nav UI?

Existing providers (Anthropic, Codex) each surface **two** login items in `buildNavItems` (`browser` + `headless`). Copilot only has one flow. Do we:
- **(a)** Show one item labeled `"Login with GitHub Copilot"` (clean, but inconsistent with other OAuth providers).
- **(b)** Show two items where both route through the same device flow (consistent, but misleading).

**Recommend (a).**

### 4.8 Re-login while tokens exist

When `hasOAuthTokens` is true, `buildNavItems` (lines 155-185) shows both a logout item AND the login items. Should Copilot follow this (allow re-login without logout) or require logout first?

### 4.9 Label text and deployment select UX

What are the final strings for:
- The login nav item label?
- The deployment-select prompt message? (opencode uses `"Select GitHub deployment type"`)
- The enterprise URL prompt message? (opencode uses `"Enter your GitHub Enterprise URL or domain"` with placeholder `"company.ghe.com or https://company.ghe.com"`)
- Error messages for URL validation?

### 4.10 Unified `credentials.json` envelope for future OAuth providers

This slice adds a third dedicated file (`copilot_auth.json`). Over time we'll accumulate files for every OAuth provider. Does the product owner want slice 1 to also kick off a refactor to unify OAuth credentials into a single opencode-style `auth.json` map? **Recommend deferring** — scope creep, and no second OAuth-with-enterprise provider is on the roadmap.

---

## 5. Acceptance Criteria Candidates

These bullets are draft-quality Gherkin candidates for the slice-1 feature file. They are intentionally scoped to OAuth flow + storage — no SDK, no model catalog, no chat.

1. **Given** a user has no existing Copilot credentials, **when** they select "Login with GitHub Copilot" from the provider settings panel, **then** they see a deployment-type selection prompt with options "GitHub.com" and "GitHub Enterprise".

2. **Given** the user selects "GitHub.com", **when** the selection is confirmed, **then** fspec immediately initiates the device-code request against `https://github.com/login/device/code` and displays the returned user code and verification URL in `oauth-device-waiting` mode.

3. **Given** the user selects "GitHub Enterprise", **when** the selection is confirmed, **then** an enterprise URL text entry prompt appears, which validates input as a parseable URL or domain before proceeding (matching the opencode validator at `copilot.ts:184-190`).

4. **Given** the device flow is polling and the server returns `authorization_pending`, **when** the advertised `interval` elapses plus the 3-second safety margin, **then** fspec re-polls without user intervention.

5. **Given** the device flow is polling and the server returns `slow_down`, **when** fspec next sleeps, **then** it uses the server-supplied `interval` if present, otherwise adds 5 seconds to the current interval (RFC 8628 §3.5), plus the 3-second safety margin.

6. **Given** the device flow succeeds for a `github.com` deployment, **when** the token is persisted, **then** `$FSPEC_HOME/copilot_auth.json` is written with mode `0o600` containing `{ access_token, token_type, scope }` and no `enterprise_url` field.

7. **Given** the device flow succeeds for an `enterprise` deployment, **when** the token is persisted, **then** `copilot_auth.json` includes `enterprise_url` set to the normalized domain (without `https://` prefix or trailing `/`).

8. **Given** persisted Copilot credentials exist, **when** `copilotOauthGetTokens()` is called, **then** it returns the current token and enterprise URL read fresh from disk (no in-memory cache).

---

## 6. Notes and Risks

- **The Codex device flow in fspec is NOT a usable skeleton** for GitHub Copilot. Codex uses OpenAI's `/api/accounts/deviceauth/usercode` custom endpoint with form-encoded polling and a `device_auth_id` field. GitHub uses the standard RFC 8628 `/login/device/code` with JSON polling and a `device_code` field. Implementing `copilot_oauth.rs` from scratch is cleaner than adapting `codex_device_auth.rs`.
- **Refresh: none.** Design in the assumption that the token is permanent-until-revoked. Any future "token refresh" ticket would need a complete rethink because GitHub OAuth Apps don't support refresh grants at all.
- **Hot-swap works for free** if `read_copilot_auth()` re-reads the file on every call (no cache). That's the minimum-work pattern and matches opencode's implicit behavior.
- **Slice boundary:** the fetch wrapper that reads `copilot_auth.json` on every API request is slice 2. Slice 1 only needs the write path and a NAPI `get` function the TS layer can call once to populate a UI status badge.
- **Testing strategy:** wiremock for the `/login/device/code` and `/login/oauth/access_token` endpoints (mirroring `codelet/providers/src/codex/codex_device_auth.rs` tests). Cover: happy path, `authorization_pending` → success, `slow_down`, terminal error, enterprise URL persistence, file permissions on the output file.
- **Cross-platform chmod:** gate `set_permissions` under `#[cfg(unix)]`. Windows gets default ACLs.

---

**End of slice 1 memo.**
