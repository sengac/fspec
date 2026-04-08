# Wiring Gap Analysis: GitHub Copilot Provider

**Investigation date:** 2026-04-07
**Scope:** Confirm that GitHub Copilot is NOT visible in the providers screen of the TUI because the Rust `codelet::providers::copilot` module is not wired through the NAPI bridge to the TypeScript layer.

**Result:** CONFIRMED. The Rust implementation (`/Users/rquast/projects/fspec/codelet/providers/src/copilot/`) is complete (auth, oauth device flow, provider, header facade, endpoint routing, models). The TypeScript/TUI layer has ZERO references to it. There is a complete wiring break at the NAPI boundary.

---

## Confirmed Gaps (🔴 BLOCKING)

### A. Provider Registry — TypeScript layer is completely unaware of Copilot

1. **`/Users/rquast/projects/fspec/src/utils/provider-config.ts:77-94`** — `SUPPORTED_PROVIDERS` array does NOT contain `'github-copilot'`. Confirmed: 16 entries (`openai, anthropic, cohere, gemini, mistral, xai, together, huggingface, openrouter, groq, deepseek, moonshot, galadriel, azure, zai, codex`). No copilot.
2. **`/Users/rquast/projects/fspec/src/utils/provider-config.ts:101-264`** — `PROVIDER_REGISTRY` has NO entry for `github-copilot`. Last entry is `codex` ending at line 263.
3. **`/Users/rquast/projects/fspec/src/utils/provider-config.ts:96`** — `ProviderId` type is derived from `SUPPORTED_PROVIDERS`, so `github-copilot` is not even an assignable string literal. Any TUI code referring to `github-copilot` will get a TS error.

### B. NAPI Bridge — no Rust→TS bridge file exists for Copilot

4. **`/Users/rquast/projects/fspec/codelet/napi/src/`** — No `copilot_oauth.rs` file. Directory listing confirms only `claude_oauth.rs` and `codex_oauth.rs`.
5. **`/Users/rquast/projects/fspec/codelet/napi/src/lib.rs:18-19`** — Only `claude_oauth` and `codex_oauth` are declared as modules. No `mod copilot_oauth;`.
6. **`/Users/rquast/projects/fspec/codelet/napi/src/lib.rs:91-93`** — Only `pub use claude_oauth::*` and `pub use codex_oauth::*`. No copilot re-export.
7. **`/Users/rquast/projects/fspec/codelet/napi/index.d.ts`** — `grep -ic copilot` returns `0`. There are NO copilot exports in the auto-generated TypeScript declaration. (For comparison, `claudeOauth` appears 6 times.) The TS layer literally has no symbol to import.
8. **`/Users/rquast/projects/fspec/codelet/napi/src/credentials/`** — `grep -rn copilot` returns nothing in `mod.rs`, `napi_bindings.rs`, `resolver.rs`, `store.rs`, `types.rs`. The unified credential store does NOT surface Copilot OAuth state to the TS layer. (Note: `codelet_providers::credentials::has_github_copilot_auth()` exists in Rust at `credentials.rs:136`, but is not bridged through NAPI.)

### C. TUI Hook State — `useProviderSettingsState` has no Copilot branches

9. **`/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts:28-38`** — Imports from `@sengac/codelet-napi` only include `codexOauth*` and `claudeOauth*` symbols. No `copilotOauth*` imports (and none exist to import — see gap #7).
10. **`/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts:289-319`** — `reload()` only handles `providerId === 'anthropic'` (calls `claudeOauthGetTokens()`) and `else` (assumes Codex, calls `codexOauthGetTokens()`). Copilot providers would be misclassified as Codex and call the wrong API. ❗
11. **`/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts:551-581`** — `startBrowserLogin()` only branches on `'anthropic'` vs `else (codex)`. No Copilot branch. (Also, Copilot has no browser login — it is device-only — so this menu item should not even appear for Copilot.)
12. **`/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts:586-652`** — `startDeviceLogin()` only branches on `'anthropic'` (headless code entry) vs `else (codex device)`. Copilot needs its own branch that:
    - Prompts for `deploymentType` (github.com vs enterprise)
    - If enterprise, prompts for `enterpriseUrl`
    - Then calls a copilot-specific NAPI device-flow start + poll
13. **`/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts:684-701`** — `disconnectOauth()` only handles `'anthropic'` (calls `claudeOauthClearTokens`) and `else` (calls `codexOauthClearTokens`). Copilot tokens stored in `~/.fspec/credentials/copilot_auth.json` are never deleted; the file would silently leak.
14. **`/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts:165-185`** — `buildNavItems()` hard-codes only two label variants:
    ```ts
    const isAnthropic = provider.id === 'anthropic';
    const browserLabel = isAnthropic
      ? 'Login with Claude (browser)'
      : 'Login with ChatGPT (browser)';
    ```
    Even if `github-copilot` were registered, it would be labelled "Login with ChatGPT (browser)". And the unwanted `browser` row would appear for a provider that has no browser flow.

### D. HookMode Types — no Copilot-specific modes

15. **`/Users/rquast/projects/fspec/src/tui/types/settingsMode.ts:18-41`** — `HookMode` union has 12 variants, NONE of which model:
    - Selecting deployment type (github.com vs enterprise)
    - Entering enterprise URL

    `oauth-device-waiting` could be reused for the polling phase, but it currently has only `userCode` + `verificationUrl` fields. To label the Copilot flow distinctly we would need to either extend it or add a new variant.

### E. Input Handlers — no handlers for the new modes

16. **`/Users/rquast/projects/fspec/src/tui/inputHandlers/oauthModeHandler.ts:21-117`** — Handles only the existing 5 OAuth modes (`oauth-browser-waiting`, `oauth-device-waiting`, `oauth-headless-code-entry`, `oauth-success`, `oauth-error`). NO handler for `oauth-deployment-type-select` or `oauth-enterprise-url-entry`. Without handlers, key presses in those modes would fall through to list navigation.

### F. Panel Rendering — no render branches for Copilot

17. **`/Users/rquast/projects/fspec/src/tui/components/ProviderSettingsPanel.tsx:359-385`** — `oauth-browser-waiting` render uses `mode.providerId === 'anthropic' ? 'Claude OAuth Login' : 'Codex OAuth Login'`. No Copilot title.
18. **`/Users/rquast/projects/fspec/src/tui/components/ProviderSettingsPanel.tsx:388-420`** — `oauth-device-waiting` render hard-codes `<Text>Codex Device Login</Text>` (line 398). No Copilot title.
19. **`/Users/rquast/projects/fspec/src/tui/components/ProviderSettingsPanel.tsx:423-445`** — `oauth-success` hard-codes `mode.providerId === 'anthropic' ? '✓ Connected to Claude' : '✓ Connected to ChatGPT'`. No Copilot label.
20. **`/Users/rquast/projects/fspec/src/tui/components/ProviderSettingsPanel.tsx:220-245`** — `disconnect-oauth` hard-codes `mode.providerId === 'anthropic' ? 'Claude' : 'ChatGPT'`. No Copilot.
21. **`/Users/rquast/projects/fspec/src/tui/components/ProviderSettingsPanel.tsx`** — No render branches for `oauth-deployment-type-select` or `oauth-enterprise-url-entry`.
22. **🔴 FILE SIZE VIOLATION:** `/Users/rquast/projects/fspec/src/tui/components/ProviderSettingsPanel.tsx` is **772 lines** (project rule: keep files under 300 lines). Adding more render branches without refactoring violates the standard.
23. **🔴 FILE SIZE VIOLATION:** `/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts` is **776 lines** — already over budget.
24. **🔴 FILE SIZE VIOLATION:** `/Users/rquast/projects/fspec/src/utils/provider-config.ts` is **533 lines** — already over budget.

### G. Mode Mapper — no Copilot pass-through

25. **`/Users/rquast/projects/fspec/src/tui/utils/providerSettingsModeMapper.ts:60-68`** — `mapToEffectivePanelMode()` passes through only the existing 5 oauth modes. New `oauth-deployment-type-select` and `oauth-enterprise-url-entry` modes would not be mapped to the panel, defaulting to `{ type: 'list' }`.

### H. Credentials Utility — no Copilot env var, no Copilot-aware status

26. **`/Users/rquast/projects/fspec/src/utils/credentials.ts:65-82`** — `PROVIDER_ENV_VARS` does NOT include `github-copilot`. Even if a user set `GITHUB_TOKEN` or `COPILOT_TOKEN`, `getProviderConfig('github-copilot')` would return `{}`.
27. **`/Users/rquast/projects/fspec/src/utils/credentials.ts:275-289`** — `maskApiKey()` regex prefixes don't include `ghu_*`. Copilot tokens would either default to `apiKey.slice(0,6)` (acceptable, not blocking).


---

## Implementation Plan

For each gap, what file needs to change and what is the change.

### Rust / NAPI layer

- **`/Users/rquast/projects/fspec/codelet/napi/src/copilot_oauth.rs`** (NEW FILE) — Wrap `codelet_providers::copilot::oauth::*` and `codelet_providers::copilot::auth::*` in NAPI bindings. Mirror the pattern in `claude_oauth.rs`. Functions: see *NAPI Bridge Missing Functions* below.
- **`/Users/rquast/projects/fspec/codelet/napi/src/lib.rs`** — Add `mod copilot_oauth;` (next to `mod claude_oauth;` at line 18) and `pub use copilot_oauth::*;` (next to line 91).
- **`/Users/rquast/projects/fspec/codelet/napi/index.d.ts`** — Will be auto-regenerated by `napi build`. After regeneration, verify it contains `copilotOauth*` exports.
- **`/Users/rquast/projects/fspec/codelet/providers/src/copilot/oauth.rs`** — Likely already complete; expose a two-phase split (start vs poll) as separate `pub async fn` calls if not already shaped for NAPI consumption. The existing `copilot_device_auth_login` is a single-call orchestrator that takes a callback — NAPI cannot easily ferry a callback across the boundary, so we need either the existing `request_device_code` + `poll_device_token` building blocks (already `pub`) or two new `start_login` / `poll_login` wrappers.

### Provider registry / credentials

- **`/Users/rquast/projects/fspec/src/utils/provider-config.ts`**:
  - Append `'github-copilot'` to `SUPPORTED_PROVIDERS` (line 77-94).
  - Add a 17th entry to `PROVIDER_REGISTRY` (after `codex`, line 263) — see *Provider Registry Entry Needed* below.
  - Note: Adding to `SUPPORTED_PROVIDERS` widens `ProviderId`, which is fine.
- **`/Users/rquast/projects/fspec/src/utils/credentials.ts`**:
  - Line 65-82: Add `'github-copilot': []` to `PROVIDER_ENV_VARS` (Copilot uses OAuth, no API key env var, but the entry is still needed so `getProviderConfig` doesn't throw on lookup).
  - Optionally: extend `maskApiKey` regex on line 281 to recognise the `ghu_*` prefix.

### TUI hook (useProviderSettingsState)

- **`/Users/rquast/projects/fspec/src/tui/hooks/useProviderSettingsState.ts`**:
  - Lines 28-38: Add imports `copilotOauthGetCredential`, `copilotOauthClearCredential`, `copilotOauthDeviceLoginStart`, `copilotOauthDeviceLoginPoll`, `copilotNormalizeEnterpriseDomain` from `@sengac/codelet-napi`.
  - Lines 289-319 (`reload`): Add a third branch `else if (providerId === 'github-copilot') { const cred = await copilotOauthGetCredential(); if (cred) { hasOAuthTokens = true; status = { hasKey: true, maskedKey: 'OAuth', source: cred.enterpriseUrl ? 'GitHub Enterprise' : 'GitHub Copilot' }; } }`. Note: the existing branching is `if anthropic / else (assume codex)` — this MUST be restructured to `if anthropic / else if codex / else if github-copilot / else (no oauth)` to avoid future provider misclassification.
  - Lines 165-185 (`buildNavItems`): Replace the binary `isAnthropic` ternary with a switch on `provider.id`. For `github-copilot`, only emit a single device-flow row (`Login with GitHub Copilot`), NOT a browser row.
  - Lines 551-581 (`startBrowserLogin`): Add explicit guard — Copilot does not support browser login. If invoked for `'github-copilot'` it should be a no-op or set an error mode.
  - Lines 586-652 (`startDeviceLogin`): Add a third branch for `'github-copilot'` that transitions to `oauth-deployment-type-select` mode (see new `HookMode` variants below). The existing async device-flow polling should be reused once deployment+url are gathered.
  - Lines 684-701 (`disconnectOauth`): Add a third branch for `'github-copilot'` that calls `await copilotOauthClearCredential()`.
  - Add a new exported callback `submitCopilotDeployment(deploymentType: 'github.com' | 'enterprise')` to advance from `oauth-deployment-type-select` → either polling (github.com) or `oauth-enterprise-url-entry`.
  - Add a new exported callback `submitCopilotEnterpriseUrl(url: string)` to normalise the URL via `copilotNormalizeEnterpriseDomain` and start polling.

### HookMode types

- **`/Users/rquast/projects/fspec/src/tui/types/settingsMode.ts`** — Add two new variants to `HookMode` (see *TUI HookMode Variants Needed* below).

### Input handlers

- **`/Users/rquast/projects/fspec/src/tui/inputHandlers/oauthModeHandler.ts`**:
  - Add a handler block for `oauth-deployment-type-select` — arrow keys to move selection between `github.com` / `enterprise`, Enter to submit, Esc to cancel.
  - Add a handler block for `oauth-enterprise-url-entry` — character input into `urlInput`, backspace, Enter to submit (calls `providerSettings.submitCopilotEnterpriseUrl(mode.urlInput)`), Esc to cancel.
  - Update `oauth-device-waiting` rendering (already covered by F#18).

### Mode mapper

- **`/Users/rquast/projects/fspec/src/tui/utils/providerSettingsModeMapper.ts`**:
  - Lines 60-68: Add `oauth-deployment-type-select` and `oauth-enterprise-url-entry` to the pass-through list.

### Panel rendering

- **`/Users/rquast/projects/fspec/src/tui/components/ProviderSettingsPanel.tsx`** (file is 772 lines — refactor REQUIRED before adding):
  - Line 222 (`disconnect-oauth`): Replace binary ternary with provider→label map that includes `github-copilot → 'GitHub Copilot'`.
  - Lines 360-363 (`oauth-browser-waiting`): Same — but Copilot will never reach this path, so the title fallback is a defensive change.
  - Lines 397-399 (`oauth-device-waiting`): Replace `Codex Device Login` literal with provider-aware title (`'GitHub Copilot Device Login'` vs `'Codex Device Login'`). Optionally add an "Enterprise: <host>" subline when Copilot+Enterprise.
  - Lines 424-427 (`oauth-success`): Add `'github-copilot' → '✓ Connected to GitHub Copilot'`.
  - Add new render branches for `oauth-deployment-type-select` (radio-style list of github.com / enterprise) and `oauth-enterprise-url-entry` (text input box mirroring the headless-code-entry layout at lines 448-480).
  - **Refactor recommendation:** before adding more branches, extract the OAuth render branches into a sibling file `ProviderSettingsOauthPanel.tsx`. This is now urgent because the file is already at 772 lines.

### Tests

- **`/Users/rquast/projects/fspec/src/tui/__tests__/provider-settings-mode-types.test.ts`** — Add cases for the two new HookMode variants.
- **`/Users/rquast/projects/fspec/src/tui/__tests__/provider-settings-oauth-guards.test.ts`** — Add a test that confirms Copilot's `startBrowserLogin` is rejected and that `startDeviceLogin` enters `oauth-deployment-type-select`.
- New test file: `src/tui/inputHandlers/__tests__/copilotOauthModeHandler.test.ts` — exercises the deployment-type and enterprise-url handlers.


---

## Reference: opencode prompt flow

Extracted from `/tmp/opencode/packages/opencode/src/plugin/github-copilot/copilot.ts:154-208`:

```ts
methods: [
  {
    type: "oauth",
    label: "Login with GitHub Copilot",
    prompts: [
      {
        type: "select",
        key: "deploymentType",
        message: "Select GitHub deployment type",
        options: [
          { label: "GitHub.com",         value: "github.com",  hint: "Public" },
          { label: "GitHub Enterprise",  value: "enterprise",  hint: "Data residency or self-hosted" },
        ],
      },
      {
        type: "text",
        key: "enterpriseUrl",
        message: "Enter your GitHub Enterprise URL or domain",
        placeholder: "company.ghe.com or https://company.ghe.com",
        when: { key: "deploymentType", op: "eq", value: "enterprise" },
        validate: (value) => {
          if (!value) return "URL or domain is required";
          try {
            const url = value.includes("://") ? new URL(value) : new URL(`https://${value}`);
            if (!url.hostname) return "Please enter a valid URL or domain";
            return undefined;
          } catch {
            return "Please enter a valid URL (e.g., company.ghe.com or https://company.ghe.com)";
          }
        },
      },
    ],
    async authorize(inputs = {}) {
      const deploymentType = inputs.deploymentType || "github.com";
      let domain = "github.com";
      if (deploymentType === "enterprise") {
        domain = normalizeDomain(inputs.enterpriseUrl!);
      }
      // ...request device code at https://${domain}/login/device/code
      // ...poll https://${domain}/login/oauth/access_token
    }
  }
]
```

The key UX shape fspec must replicate:

1. **Always** show a deployment-type selector first.
2. **Conditionally** show the enterprise URL prompt only when `deploymentType === 'enterprise'` (the `when` clause).
3. Validate that the URL is a parseable hostname BEFORE issuing the device-code request.
4. Normalise the entered URL via `normalizeDomain` (strip scheme, strip trailing slash) — this is exactly what `copilot::oauth::normalize_enterprise_domain` already does in Rust at `oauth.rs:146-152`.
5. Use the resulting `host` to build the device-code URL (`https://{host}/login/device/code`) and poll URL (`https://{host}/login/oauth/access_token`).

Opencode's polling loop (lines 234-306) maps onto the existing Rust `poll_device_token` in `copilot/oauth.rs:197-306`, which already handles `authorization_pending`, `slow_down` (RFC 8628 §3.5), `expired_token`, and `access_denied`. No changes needed there.


---

## NAPI Bridge Missing Functions

To be added in `/Users/rquast/projects/fspec/codelet/napi/src/copilot_oauth.rs`. Mirrors the pattern established by `claude_oauth.rs` and `codex_oauth.rs`.

### Rust NAPI signatures

```rust
use codelet_providers::copilot::auth::{
    delete_copilot_auth, read_copilot_auth, write_copilot_auth,
    CopilotAuthJson, COPILOT_TOKEN_NEVER_EXPIRES,
};
use codelet_providers::copilot::oauth::{
    normalize_enterprise_domain, poll_device_token, request_device_code,
    CopilotDeploymentType, CopilotDeviceCodeResponse, CopilotPollConfig, CopilotPollResult,
    COPILOT_DEFAULT_HOST,
};
use napi::bindgen_prelude::*;

/// Copilot OAuth credential exposed to TypeScript. Mirrors CopilotAuthJson.
/// expires == 0.0 encodes "never expires" (ghu_* tokens never auto-expire).
#[napi(object)]
pub struct NapiCopilotCredential {
    pub access_token: String,
    pub refresh_token: String,
    /// Expiry timestamp in ms since Unix epoch. 0 = never expires.
    pub expires: f64,
    /// Some(normalized host) for GHE, None for github.com.
    pub enterprise_url: Option<String>,
}

/// Phase-1 result: device code + everything needed to drive polling.
#[napi(object)]
pub struct NapiCopilotDeviceStartResult {
    pub user_code: String,
    pub verification_url: String,
    pub device_code: String,
    /// Server-provided polling interval, in seconds.
    pub interval: f64,
    /// Resolved host URL (`https://github.com` or `https://<ghe-host>`).
    pub host_url: String,
    /// "github.com" | "enterprise" — echoed back so polling can persist it.
    pub deployment_type: String,
    /// Normalized enterprise host (Some when deployment_type == "enterprise").
    pub enterprise_host: Option<String>,
}

/// Phase 1: Start Copilot device auth flow.
///
/// If `enterprise_url` is None → uses `https://github.com`.
/// If `enterprise_url` is Some(raw) → normalizes via `normalize_enterprise_domain`
/// and builds `https://<host>`.
///
/// Returns user_code + verification_url + device_code so the TUI can
/// display them to the user and begin polling.
#[napi]
pub async fn copilot_oauth_device_login_start(
    enterprise_url: Option<String>,
) -> Result<NapiCopilotDeviceStartResult>;

/// Phase 2: Poll the device-token endpoint until the user authorizes
/// or a terminal error occurs. On success, persists the credential to
/// `~/.fspec/credentials/copilot_auth.json` (mode 0600) and returns it.
///
/// `host_url` and `enterprise_host` come from the start result.
#[napi]
pub async fn copilot_oauth_device_login_poll(
    device_code: String,
    interval: f64,
    host_url: String,
    enterprise_host: Option<String>,
) -> Result<NapiCopilotCredential>;

/// Read stored Copilot credential. Returns null if file is missing.
#[napi]
pub async fn copilot_oauth_get_credential() -> Result<Option<NapiCopilotCredential>>;

/// Delete `~/.fspec/credentials/copilot_auth.json`. Idempotent.
#[napi]
pub async fn copilot_oauth_clear_credential() -> Result<()>;

/// Pure helper exposed so the TS layer can preview the normalized host
/// before submitting. Could also live in TS (trivial regex), but mirroring
/// the Rust implementation prevents drift.
#[napi]
pub fn copilot_normalize_enterprise_domain(input: String) -> String;
```

### Auto-generated TypeScript signatures

After `napi build`, `codelet/napi/index.d.ts` should gain:

```ts
export interface NapiCopilotCredential {
  accessToken: string
  refreshToken: string
  /** Expiry timestamp in ms since Unix epoch. 0 = never expires. */
  expires: number
  /** Some(normalized host) for GHE, undefined for github.com. */
  enterpriseUrl?: string
}

export interface NapiCopilotDeviceStartResult {
  userCode: string
  verificationUrl: string
  deviceCode: string
  /** Server-provided polling interval, in seconds. */
  interval: number
  /** Resolved host URL. */
  hostUrl: string
  /** "github.com" | "enterprise". */
  deploymentType: string
  /** Normalized enterprise host (present when deploymentType === "enterprise"). */
  enterpriseHost?: string
}

export declare function copilotOauthDeviceLoginStart(
  enterpriseUrl?: string | undefined | null
): Promise<NapiCopilotDeviceStartResult>

export declare function copilotOauthDeviceLoginPoll(
  deviceCode: string,
  interval: number,
  hostUrl: string,
  enterpriseHost?: string | undefined | null
): Promise<NapiCopilotCredential>

export declare function copilotOauthGetCredential(): Promise<NapiCopilotCredential | null>

export declare function copilotOauthClearCredential(): Promise<void>

export declare function copilotNormalizeEnterpriseDomain(input: string): string
```


---

## TUI HookMode Variants Needed

Add to the `HookMode` union in `/Users/rquast/projects/fspec/src/tui/types/settingsMode.ts`:

```ts
export type HookMode =
  // ... existing 12 variants ...
  | {
      type: 'oauth-deployment-type-select';
      providerId: string;
      // Index into ['github.com', 'enterprise']
      selectedIndex: 0 | 1;
    }
  | {
      type: 'oauth-enterprise-url-entry';
      providerId: string;
      urlInput: string;
      // Optional validation error message to display below the input.
      validationError?: string;
    };
```

### State transitions

```
list
  └── user activates 'Login with GitHub Copilot'
       └── oauth-deployment-type-select { selectedIndex: 0 }
            ├── user picks github.com + Enter
            │    └── oauth-device-waiting (Copilot variant — see below)
            │         └── (poll) ─┬─ oauth-success
            │                     └─ oauth-error
            └── user picks enterprise + Enter
                 └── oauth-enterprise-url-entry { urlInput: '' }
                      ├── Enter with valid URL
                      │    └── oauth-device-waiting
                      │         └── (poll) ─┬─ oauth-success
                      │                     └─ oauth-error
                      └── Enter with invalid URL
                           └── oauth-enterprise-url-entry { validationError: '...' }
```

### Existing `oauth-device-waiting` extension (optional)

The current shape is:

```ts
| {
    type: 'oauth-device-waiting';
    providerId: string;
    userCode: string;
    verificationUrl: string;
  }
```

For Copilot we want to show an additional "Enterprise: <host>" line in the panel. Two options:

1. **Extend** `oauth-device-waiting` with an optional `enterpriseHost?: string` field. Low risk; all existing call sites still compile because the field is optional.
2. **Add a new variant** `oauth-copilot-device-waiting`. Purer separation but requires a new render branch and a new input handler.

Recommendation: **Option 1** (extend with optional field) to keep the scope minimal and avoid duplicating the entire render branch.


---

## Provider Registry Entry Needed

Add to `PROVIDER_REGISTRY` in `/Users/rquast/projects/fspec/src/utils/provider-config.ts` as the 17th entry, appended after the `codex` entry (line 263):

```ts
{
  id: 'github-copilot',
  name: 'GitHub Copilot',
  baseUrl: 'https://api.githubcopilot.com',
  envVar: '',                 // No env var — OAuth device flow only
  authMethod: 'bearer',       // Bearer ghu_* on every request
  authType: 'oauth',          // OAuth device flow (RFC 8628)
  requiresApiKey: false,      // Credentials come from copilot_auth.json, not the key store
  description:
    'GitHub Copilot via OAuth device flow. Supports github.com and GitHub Enterprise deployments. Tokens are stored in ~/.fspec/credentials/copilot_auth.json and never expire.',
},
```

And append `'github-copilot'` to `SUPPORTED_PROVIDERS` (line 77-94):

```ts
export const SUPPORTED_PROVIDERS = [
  'openai',
  'anthropic',
  'cohere',
  'gemini',
  'mistral',
  'xai',
  'together',
  'huggingface',
  'openrouter',
  'groq',
  'deepseek',
  'moonshot',
  'galadriel',
  'azure',
  'zai',
  'codex',
  'github-copilot',  // NEW
] as const;
```

Also add to `PROVIDER_ENV_VARS` in `/Users/rquast/projects/fspec/src/utils/credentials.ts` (line 65-82):

```ts
const PROVIDER_ENV_VARS: Record<string, string[]> = {
  // ... existing entries ...
  'github-copilot': [],  // OAuth only, no env var
};
```


---

## Complete File Change Checklist

Every file that must change to make GitHub Copilot visible in the TUI providers screen AND fully functional end-to-end:

### Rust / NAPI

| File | Action | Purpose |
|------|--------|---------|
| `codelet/napi/src/copilot_oauth.rs` | **CREATE** | NAPI wrappers around `codelet_providers::copilot::oauth` + `auth` |
| `codelet/napi/src/lib.rs` | EDIT | `mod copilot_oauth;` + `pub use copilot_oauth::*;` |
| `codelet/napi/index.d.ts` | AUTO-REGENERATE | Will gain `copilotOauth*` exports after `napi build` |

### TypeScript utility layer

| File | Action | Purpose |
|------|--------|---------|
| `src/utils/provider-config.ts` | EDIT | Add `'github-copilot'` to `SUPPORTED_PROVIDERS` + `PROVIDER_REGISTRY` |
| `src/utils/credentials.ts` | EDIT | Add `'github-copilot': []` to `PROVIDER_ENV_VARS`; optional `ghu_*` mask |

### TUI hook / mode layer

| File | Action | Purpose |
|------|--------|---------|
| `src/tui/types/settingsMode.ts` | EDIT | Add `oauth-deployment-type-select` + `oauth-enterprise-url-entry` variants; optionally extend `oauth-device-waiting` with `enterpriseHost?` |
| `src/tui/hooks/useProviderSettingsState.ts` | EDIT | Import copilot NAPI symbols; add copilot branches in `reload`, `buildNavItems`, `startDeviceLogin`, `disconnectOauth`; add `submitCopilotDeployment` + `submitCopilotEnterpriseUrl` callbacks |
| `src/tui/utils/providerSettingsModeMapper.ts` | EDIT | Pass-through new HookMode variants to PanelMode |
| `src/tui/inputHandlers/oauthModeHandler.ts` | EDIT | Handle deployment-type-select + enterprise-url-entry input |

### TUI rendering

| File | Action | Purpose |
|------|--------|---------|
| `src/tui/components/ProviderSettingsPanel.tsx` | EDIT (**REFACTOR FIRST**) | Provider-aware labels for disconnect/browser-waiting/device-waiting/success; new render branches for the two new modes. File is 772 lines — must extract OAuth branches to a new file first. |
| `src/tui/components/ProviderSettingsOauthPanel.tsx` | **CREATE (recommended)** | Extract the 5 existing OAuth render branches + the 2 new ones into a dedicated sub-component to keep the parent under 300 lines. |

### Tests

| File | Action | Purpose |
|------|--------|---------|
| `src/tui/__tests__/provider-settings-mode-types.test.ts` | EDIT | Add cases for new HookMode variants |
| `src/tui/__tests__/provider-settings-oauth-guards.test.ts` | EDIT | Guard test: Copilot rejects browser login; Copilot device login enters deployment-type-select |
| `src/tui/inputHandlers/__tests__/copilotOauthModeHandler.test.ts` | **CREATE** | Input handler tests for deployment-type-select + enterprise-url-entry |
| `src/utils/__tests__/provider-config.test.ts` | EDIT (if exists) | Assert `'github-copilot'` is registered with correct `authType: 'oauth'` + `requiresApiKey: false` |
| `codelet/napi/tests/copilot_oauth.test.ts` | **CREATE** | End-to-end NAPI test for start/poll/get/clear using a wiremock server |

---

## Summary

The Rust `codelet::providers::copilot` implementation is complete and well-tested (auth.rs, oauth.rs, provider.rs, behavior_facade.rs, endpoint.rs, header_facade.rs, refreshing_client.rs, classifier.rs, constants.rs, models/). It builds. Its `has_github_copilot_auth()` function is wired into `ProviderManager::detect_active` at `manager.rs:374` so the Rust side can already route inference requests to Copilot if a credential exists.

**But no credential can ever exist via the TUI** because:

1. There is no NAPI bridge file (`copilot_oauth.rs`).
2. There are no TS-side exports (`copilotOauth*` symbols do not exist in `@sengac/codelet-napi`).
3. `provider-config.ts` does not register `github-copilot`, so it never appears in the `getProviderRegistry()` list that `useProviderSettingsState.reload()` iterates over.
4. Even if it appeared, `reload()`, `buildNavItems()`, `startDeviceLogin()`, and `disconnectOauth()` all have hard-coded binary branches that would misclassify Copilot as Codex.
5. There are no TUI modes, handlers, or render branches for the deployment-type-select and enterprise-url-entry steps that the Copilot device flow requires.

**Minimum number of files to change: 11** (3 Rust, 4 TS utilities/hooks, 4 TUI components/handlers).
**Recommended additional files: 3** (new `ProviderSettingsOauthPanel.tsx` to address the 300-line rule, plus 2 new test files).
**Blocking technical debt discovered:** three files already exceed the 300-line limit (`ProviderSettingsPanel.tsx` at 772, `useProviderSettingsState.ts` at 776, `provider-config.ts` at 533) and MUST be refactored before adding more Copilot-specific code to them.
