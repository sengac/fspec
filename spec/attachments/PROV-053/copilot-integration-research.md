# GitHub Copilot Integration Research

**Source project:** [anomalyco/opencode](https://github.com/anomalyco/opencode) (cloned into `/tmp/opencode` on 2026-04-07)

**Purpose:** Technical reference for implementing a GitHub Copilot provider in fspec. This document captures the complete end-to-end flow used by opencode — OAuth device flow, token storage, provider registration, custom `fetch` wrapper, model discovery, endpoint selection, and option mapping — with file/line citations so every design decision is traceable back to working reference code.

---

## 1. High-Level Architecture

Copilot is implemented in opencode as an **internal plugin** exposing an `AuthHook` and a `ProviderHook`. It is wired alongside other auth plugins (Codex, GitLab, Poe, Cloudflare) via `INTERNAL_PLUGINS` and plugs into the generic provider resolution machinery through:

- A `BUNDLED_PROVIDERS` entry keyed by the **fake** npm package name `@ai-sdk/github-copilot` (the SDK is vendored, not published).
- A `custom()` registry entry under the provider id `github-copilot` that overrides model resolution.
- A vendored, Copilot-specific fork of the OpenAI-compatible AI SDK that knows about both `/chat/completions` and `/responses` endpoints.

### Key files

| Path | Purpose |
|---|---|
| `packages/opencode/src/plugin/github-copilot/copilot.ts` | OAuth device flow, provider `fetch` override, `chat.params` / `chat.headers` hooks |
| `packages/opencode/src/plugin/github-copilot/models.ts` | `/models` API client + Zod schema + merge logic |
| `packages/opencode/src/plugin/index.ts` | Registers `CopilotAuthPlugin` in `INTERNAL_PLUGINS` |
| `packages/opencode/src/auth/index.ts` | On-disk auth store (`auth.json`) and the `Oauth` / `Api` / `WellKnown` schemas |
| `packages/opencode/src/provider/provider.ts` | `BUNDLED_PROVIDERS["@ai-sdk/github-copilot"]`, `custom()["github-copilot"]`, `shouldUseCopilotResponsesApi()` |
| `packages/opencode/src/provider/sdk/copilot/copilot-provider.ts` | `createOpenaiCompatible` factory returning `chat(id)` / `responses(id)` / `languageModel(id)` |
| `packages/opencode/src/provider/sdk/copilot/chat/openai-compatible-chat-language-model.ts` | `POST {baseURL}/chat/completions` wrapper |
| `packages/opencode/src/provider/sdk/copilot/responses/openai-responses-language-model.ts` | `POST {baseURL}/responses` wrapper |
| `packages/opencode/src/provider/transform.ts` | `@ai-sdk/github-copilot` → `copilot` provider-options key mapping, reasoning/cache variants |
| `packages/opencode/src/provider/schema.ts` | `ProviderID.githubCopilot = "github-copilot"` |
| `packages/opencode/src/cli/cmd/providers.ts` | `opencode auth login` command that drives `AuthHook.methods[*].authorize()` |

### Tests worth mirroring

- `packages/opencode/test/plugin/github-copilot-models.test.ts` — model merging behavior
- `packages/opencode/test/provider/copilot/convert-to-copilot-messages.test.ts` — message conversion
- `packages/opencode/test/provider/copilot/copilot-chat-model.test.ts` — streaming chat model
- `packages/opencode/test/provider/transform.test.ts` — provider options mapping

---

## 2. OAuth Device Flow

File: `packages/opencode/src/plugin/github-copilot/copilot.ts`

### 2.1 Constants and URL construction (lines 11–28)

```ts
const CLIENT_ID = "Ov23li8tweQw6odWQebz"                           // line 11
const OAUTH_POLLING_SAFETY_MARGIN_MS = 3000                        // line 14

function normalizeDomain(url: string) {                            // line 15
  return url.replace(/^https?:\/\//, "").replace(/\/$/, "")
}

function getUrls(domain: string) {                                 // line 19
  return {
    DEVICE_CODE_URL:   `https://${domain}/login/device/code`,
    ACCESS_TOKEN_URL:  `https://${domain}/login/oauth/access_token`,
  }
}

function base(enterpriseUrl?: string) {                            // line 26
  return enterpriseUrl
    ? `https://copilot-api.${normalizeDomain(enterpriseUrl)}`
    : "https://api.githubcopilot.com"
}
```

**Observations:**

- Hard-coded GitHub OAuth App client ID: `Ov23li8tweQw6odWQebz`. No client secret (device flow doesn't need one).
- Public GitHub: OAuth endpoints are under `https://github.com/...`, Copilot API under `https://api.githubcopilot.com`.
- GitHub Enterprise (self-hosted or data-resident): OAuth endpoints under the enterprise domain, Copilot API under `https://copilot-api.<domain>`.

### 2.2 Auth method definition (lines 154–309)

The plugin exposes **one** auth method of `type: "oauth"` labeled `"Login with GitHub Copilot"`. It prompts the user first:

1. `select` prompt with key `deploymentType`: `"github.com"` vs `"enterprise"` (lines 159–175).
2. Conditional `text` prompt with key `enterpriseUrl` that only shows `when: { key: "deploymentType", op: "eq", value: "enterprise" }`, validating it parses as a URL (lines 176–192).

The CLI code that walks these prompts lives in `packages/opencode/src/cli/cmd/providers.ts:19-170` (function `handlePluginAuth`). For each prompt it uses `@clack/prompts` (`select`, `text`, `password`). The `when` rule is evaluated at lines 49–53 of that file.

### 2.3 Device code request (lines 206–228)

```ts
const deviceResponse = await fetch(urls.DEVICE_CODE_URL, {
  method: "POST",
  headers: {
    Accept: "application/json",
    "Content-Type": "application/json",
    "User-Agent": `opencode/${Installation.VERSION}`,
  },
  body: JSON.stringify({
    client_id: CLIENT_ID,
    scope: "read:user",
  }),
})
```

- POST `https://github.com/login/device/code` (or enterprise equivalent).
- Scope is only `read:user`. **No `copilot` scope is requested** — the Copilot API accepts the user token directly without a dedicated scope.
- Response parsed as `{ verification_uri, user_code, device_code, interval }`.

### 2.4 Authorize callback returned to CLI (lines 230–307)

```ts
return {
  url: deviceData.verification_uri,
  instructions: `Enter code: ${deviceData.user_code}`,
  method: "auto" as const,
  async callback() { /* polling loop */ },
}
```

The CLI, in `packages/opencode/src/cli/cmd/providers.ts:75-112`, detects `method === "auto"`:

- Prints `Go to: <verification_uri>`
- Prints `Enter code: <user_code>`
- Spins a `prompts.spinner()` labeled `"Waiting for authorization..."`
- Awaits `authorize.callback()`

### 2.5 Polling loop (lines 235–305)

```ts
while (true) {
  const response = await fetch(urls.ACCESS_TOKEN_URL, {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "User-Agent": `opencode/${Installation.VERSION}`,
    },
    body: JSON.stringify({
      client_id: CLIENT_ID,
      device_code: deviceData.device_code,
      grant_type: "urn:ietf:params:oauth:grant-type:device_code",
    }),
  })

  if (!response.ok) return { type: "failed" as const }

  const data = await response.json() as {
    access_token?: string
    error?: string
    interval?: number
  }

  if (data.access_token) {
    const result = {
      type: "success" as const,
      refresh: data.access_token,          // NOTE: refresh == access
      access:  data.access_token,          // both set to same GitHub PAT
      expires: 0,                          // never expires (treated as opaque)
    }
    if (deploymentType === "enterprise") result.enterpriseUrl = domain
    return result
  }

  if (data.error === "authorization_pending") {
    await sleep(deviceData.interval * 1000 + OAUTH_POLLING_SAFETY_MARGIN_MS)
    continue
  }

  if (data.error === "slow_down") {
    // RFC 8628 §3.5 requires adding 5s to polling interval on slow_down
    let newInterval = (deviceData.interval + 5) * 1000
    const serverInterval = data.interval
    if (serverInterval && typeof serverInterval === "number" && serverInterval > 0) {
      newInterval = serverInterval * 1000
    }
    await sleep(newInterval + OAUTH_POLLING_SAFETY_MARGIN_MS)
    continue
  }

  if (data.error) return { type: "failed" as const }
  await sleep(deviceData.interval * 1000 + OAUTH_POLLING_SAFETY_MARGIN_MS)
}
```

**Important observations for a clean re-implementation:**

1. **Only `access_token` is used.** GitHub's device flow for OAuth Apps returns just `access_token` (a `gho_...` or similar long-lived GitHub token) — not a refresh token. opencode copies the same value into both `access` and `refresh` fields of its storage schema. **`expires` is set to `0`** meaning opencode never checks expiry and never rotates the token. A logout re-runs the whole flow.
2. `slow_down` handling conforms to RFC 8628 §3.5 (increase interval by 5s, or honor server-provided interval).
3. A 3-second safety margin (`OAUTH_POLLING_SAFETY_MARGIN_MS`) is added to every sleep to avoid hitting the server slightly too early due to clock/timer drift.
4. `enterpriseUrl` is persisted only for the enterprise branch so subsequent API calls know to use the `copilot-api.<domain>` base URL.

### 2.6 Persisting the credential

The CLI handler in `packages/opencode/src/cli/cmd/providers.ts:92-103` takes the success result and calls:

```ts
await Auth.set(saveProvider /* "github-copilot" */, {
  type: "oauth",
  refresh,
  access,
  expires,
  ...extraFields,   // includes enterpriseUrl for enterprise
})
```

---

## 3. Access Token Storage, Retrieval, Refresh

### 3.1 On-disk schema

File: `packages/opencode/src/auth/index.ts`

```ts
const file = path.join(Global.Path.data, "auth.json")               // line 10

export class Oauth extends Schema.Class<Oauth>("OAuth")({           // line 15
  type: Schema.Literal("oauth"),
  refresh: Schema.String,
  access: Schema.String,
  expires: Schema.Number,
  accountId: Schema.optional(Schema.String),
  enterpriseUrl: Schema.optional(Schema.String),
}) {}
```

- A single JSON file at `$OPENCODE_DATA/auth.json`, written with mode `0o600` (line 75).
- Keyed by provider id — for Copilot, the key is the literal string `"github-copilot"`.
- `all` / `get` / `set` / `remove` are implemented (lines 60–85) using `AppFileSystem`. Trailing `/` normalization is applied (line 70) because other providers use URL-shaped keys.

The list command at `packages/opencode/src/cli/cmd/providers.ts:208-252` reads from the same `auth.json` and shows the file path in the intro message. Logout removes the provider entry (lines 455–478).

### 3.2 There is NO token refresh

The Copilot plugin treats the GitHub OAuth App access token as **opaque and permanent**:

- `expires: 0` at issuance (`copilot.ts:270`).
- No code path calls `Auth.set` with an updated token after initial login.
- The token used on every API call (`info.refresh`, see §4) is whatever was written at login time.
- If the token is ever revoked/expired on GitHub's side, the next API call will simply fail and the user must run `opencode auth login` again.

**Quirk:** the plugin reads `info.refresh`, not `info.access`, even though both hold the same string. This is likely vestigial from a design where `refresh` was exchanged for a short-lived `access` token, but it is the current behavior.

### 3.3 Auth retrieval on every request

File: `packages/opencode/src/plugin/github-copilot/copilot.ts` lines 63–153.

The plugin registers an `auth.loader` which is called by `provider.ts:1182-1201` during provider state initialization. The loader closes over a `getAuth` function, and then the returned custom `fetch` **calls `getAuth()` again on every single request** (line 75). If `auth.json` is updated externally (e.g., by a concurrent `opencode auth login`), the new token is picked up on the next request without restarting.

```ts
auth: {
  provider: "github-copilot",
  async loader(getAuth) {
    const info = await getAuth()
    if (!info || info.type !== "oauth") return {}

    const baseURL = base(info.enterpriseUrl)

    return {
      baseURL,
      apiKey: "",                                                   // line 73
      async fetch(request, init) {
        const info = await getAuth()                                // line 75 - re-read
        ...
      },
    }
  },
  methods: [ /* the OAuth method from §2.2 */ ],
}
```

The returned options object `{ baseURL, apiKey: "", fetch }` is merged into the provider's `options` in `provider.ts:1192-1200` and eventually becomes the argument passed to `createGitHubCopilotOpenAICompatible({ name: "github-copilot", baseURL, apiKey: "", fetch, ... })` at `provider.ts:1438-1441`.

`apiKey: ""` intentionally prevents the OpenAI-compatible provider factory from adding an `Authorization` header — the custom `fetch` is the sole source of auth headers.

---

## 4. Copilot API Invocation

### 4.1 Base URL

From `copilot.ts:26-28`:

- `https://api.githubcopilot.com` for github.com accounts.
- `https://copilot-api.<enterprise-domain>` for GHE.

### 4.2 `/models` endpoint — discovery and quotas-via-limits

File: `packages/opencode/src/plugin/github-copilot/models.ts`

```ts
const data = await fetch(`${baseURL}/models`, {                     // line 113
  headers,                                                          // Authorization + User-Agent
  signal: AbortSignal.timeout(5_000),
}).then(async (res) => {
  if (!res.ok) throw new Error(`Failed to fetch models: ${res.status}`)
  return schema.parse(await res.json())
})
```

Called from `copilot.ts:50-60` inside the `provider.models` hook:

```ts
return CopilotModels.get(
  base(ctx.auth.enterpriseUrl),
  {
    Authorization: `Bearer ${ctx.auth.refresh}`,
    "User-Agent":  `opencode/${Installation.VERSION}`,
  },
  provider.models,  // existing (possibly models.dev) definitions to merge into
)
```

**Response schema** (`models.ts:5-41`):

```ts
{
  data: [
    {
      model_picker_enabled: boolean,
      id: string,
      name: string,
      version: string,                    // usually "{id}-YYYY-MM-DD"
      supported_endpoints?: string[],
      capabilities: {
        family: string,
        limits: {
          max_context_window_tokens: number,
          max_output_tokens: number,
          max_prompt_tokens: number,
          vision?: {
            max_prompt_image_size: number,
            max_prompt_images: number,
            supported_media_types: string[],
          },
        },
        supports: {
          adaptive_thinking?: boolean,
          max_thinking_budget?: number,
          min_thinking_budget?: number,
          reasoning_effort?: string[],
          streaming: boolean,
          structured_outputs?: boolean,
          tool_calls: boolean,
          vision?: boolean,
        },
      },
    },
    ...
  ]
}
```

**Merge logic** (`models.ts:108-143`):

1. Start from `existing` (models.dev-sourced definitions for `github-copilot`).
2. Build a `Map` of remote models where `model_picker_enabled === true` — this effectively filters out unavailable tiers.
3. For each existing model, look it up by `api.id` in the remote map; if missing, **delete** it locally; otherwise overwrite with `build(key, remote, baseURL, prev)`.
4. For each remote model not already present, add it with `build(id, m, baseURL)`.
5. `build()` (lines 45–106) maps the Copilot response into opencode's `Model` type:
   - `providerID: "github-copilot"`
   - `api.id = remote.id`, `api.url = baseURL`, `api.npm = "@ai-sdk/github-copilot"`
   - `status: "active"`
   - `limit.context = max_context_window_tokens`, `limit.input = max_prompt_tokens`, `limit.output = max_output_tokens`
   - `capabilities.reasoning` derived from `adaptive_thinking || reasoning_effort.length || min/max_thinking_budget`
   - `capabilities.input.image` derived from `supports.vision || limits.vision.supported_media_types[*].startsWith("image/")`
   - `capabilities.toolcall = supports.tool_calls`
   - **`cost: { input: 0, output: 0, cache: { read: 0, write: 0 } }`** — Copilot is billed by GitHub subscription, not per-token.
   - `release_date` parsed from `version` by stripping `{id}-` prefix.

**There is no billing/quota/subscription endpoint called anywhere.** Grepping for `copilot.*billing`, `copilot.*usage`, `copilot.*quota`, `copilot.*subscription`, `copilot_internal`, `api.github.com.*copilot` across the full source tree returned no hits. Usage visibility for Copilot quotas (business/enterprise seat consumption) is not implemented. The token-usage numbers opencode surfaces for Copilot come from the per-request `usage` object in the streaming chat response (`openai-compatible-chat-language-model.ts:281-293`).

### 4.3 `/chat/completions` endpoint (most models)

File: `packages/opencode/src/provider/sdk/copilot/chat/openai-compatible-chat-language-model.ts`

```ts
// non-stream (lines 201-212)
await postJsonToApi({
  url: this.config.url({ path: "/chat/completions", modelId: this.modelId }),
  headers: combineHeaders(this.config.headers(), options.headers),
  body: args,
  failedResponseHandler: this.failedResponseHandler,
  successfulResponseHandler: createJsonResponseHandler(OpenAICompatibleChatResponseSchema),
  abortSignal: options.abortSignal,
  fetch: this.config.fetch,
})

// stream (lines 318-329)
await postJsonToApi({
  url: this.config.url({ path: "/chat/completions", modelId: this.modelId }),
  ...
  body: { ...args, stream: true, stream_options: this.config.includeUsage ? { include_usage: true } : undefined },
  successfulResponseHandler: createEventSourceResponseHandler(this.chunkSchema),
  ...
})
```

**Request body fields** set in `getArgs()` (lines 87–190):

```ts
{
  model: this.modelId,                  // e.g. "gpt-5.2-codex"
  user: compatibleOptions.user,
  max_tokens: maxOutputTokens,
  temperature, top_p, frequency_penalty, presence_penalty,
  response_format: /* json_schema / json_object */,
  stop: stopSequences,
  seed,
  ...spreadProviderOptions,             // see §6
  reasoning_effort: compatibleOptions.reasoningEffort,
  verbosity: compatibleOptions.textVerbosity,
  messages: convertToOpenAICompatibleChatMessages(prompt),
  tools: openaiTools,
  tool_choice: openaiToolChoice,
  thinking_budget: compatibleOptions.thinking_budget,
}
```

**Response parsing** (lines 214–302):

- `choice.message.content` → text part.
- `choice.message.reasoning_text` → reasoning part (Copilot-specific field name).
- `choice.message.reasoning_opaque` → round-tripped in `providerMetadata.copilot.reasoningOpaque` for multi-turn reasoning continuity.
- `choice.message.tool_calls[*]` → tool-call parts.
- `usage.prompt_tokens`, `usage.completion_tokens`, `usage.prompt_tokens_details.cached_tokens`, `usage.completion_tokens_details.reasoning_tokens` / `accepted_prediction_tokens` / `rejected_prediction_tokens`.

Message conversion (`convert-to-openai-compatible-chat-messages.ts`) pulls metadata from `providerOptions.copilot` for each message/part. This is the vendored OpenAI-compatible converter renamed for Copilot.

### 4.4 `/responses` endpoint (GPT-5+ models)

File: `packages/opencode/src/provider/sdk/copilot/responses/openai-responses-language-model.ts` lines 396 and 782: `path: "/responses"`.

**Selection logic** in `packages/opencode/src/provider/provider.ts:63-67`:

```ts
function shouldUseCopilotResponsesApi(modelID: string): boolean {
  const match = /^gpt-(\d+)/.exec(modelID)
  if (!match) return false
  return Number(match[1]) >= 5 && !modelID.startsWith("gpt-5-mini")
}
```

Wired into the custom model loader at `provider.ts:222-230`:

```ts
"github-copilot": () => Effect.succeed({
  autoload: false,
  async getModel(sdk, modelID, _options) {
    if (useLanguageModel(sdk)) return sdk.languageModel(modelID)
    return shouldUseCopilotResponsesApi(modelID)
      ? sdk.responses(modelID)
      : sdk.chat(modelID)
  },
  options: {},
}),
```

So:

- `gpt-5`, `gpt-5.1`, `gpt-5.2`, `gpt-5.2-codex`, `gpt-6*`, … → `/responses`
- `gpt-5-mini` → `/chat/completions` (explicitly excluded)
- `gpt-4o`, `claude-*`, `gemini-*`, `o1`, … → `/chat/completions`

### 4.5 Custom `fetch` override (headers + request classification)

File: `packages/opencode/src/plugin/github-copilot/copilot.ts` (installed via the plugin's `auth.loader` hook, lines 65–153).

#### 4.5.1 Body inspection: `isVision` / `isAgent` detection (lines 78–130)

Before headers are set, the fetch wrapper parses `init.body` (if it's a JSON string) and classifies the request against **three API shapes**:

- **Completions API** (`url.includes("completions")`, body has `messages`): last-message role determines `isAgent`; any part with `type === "image_url"` → `isVision` (lines 84–93).
- **Responses API** (body has `input` array): last-input role drives `isAgent`; any part with `type === "input_image"` → `isVision` (lines 96–105).
- **Messages API** (body has `messages`, Anthropic-shaped): checks for `part.type === "image"` including images nested in `tool_result.content[]`; `isAgent` is true unless the last message is a user message whose content has non-`tool_result` parts (lines 108–127).

All parsing is wrapped in `try {} catch {}` — classification is best-effort and defaults to `{ isVision: false, isAgent: false }` (line 129).

**The request body is NOT mutated.** Only headers change. The original `init.body` is passed through unchanged via `fetch(request, { ...init, headers })`.

#### 4.5.2 Exact headers sent (lines 132–150)

```ts
const headers: Record<string, string> = {
  "x-initiator": isAgent ? "agent" : "user",
  ...(init?.headers as Record<string, string>),
  "User-Agent": `opencode/${Installation.VERSION}`,
  Authorization: `Bearer ${info.refresh}`,
  "Openai-Intent": "conversation-edits",
}

if (isVision) {
  headers["Copilot-Vision-Request"] = "true"
}

delete headers["x-api-key"]
delete headers["authorization"]  // lowercase form

return fetch(request, { ...init, headers })
```

**Exact set of headers** always or conditionally set by this wrapper:

| Header | Value | Condition |
|---|---|---|
| `x-initiator` | `"agent"` or `"user"` | Always; default from body heuristic, but may be pre-set by `chat.headers` hook (see §5.2) |
| `User-Agent` | `opencode/${Installation.VERSION}` | Always, **overrides** any incoming UA |
| `Authorization` | `Bearer ${info.refresh}` | Always; uses the stored OAuth `refresh` token directly (not an exchanged access token) |
| `Openai-Intent` | `"conversation-edits"` | Always, hard-coded |
| `Copilot-Vision-Request` | `"true"` | Only when `isVision` is detected |

**Headers NOT set** (opencode intentionally omits them):

- `Copilot-Integration-Id` — not set anywhere
- `Editor-Version` — not set
- `Editor-Plugin-Version` — not set
- `X-GitHub-Api-Version` — not set in the Copilot fetch path
- `Openai-Organization` — not set

The spread order is important: `"x-initiator"` is placed **before** `...init?.headers`, so any `x-initiator` supplied by the upstream `chat.headers` hook will **override** the body-heuristic default. Everything listed after the spread (`User-Agent`, `Authorization`, `Openai-Intent`) **overrides** whatever the caller sent.

Lowercase `authorization` and `x-api-key` are explicitly deleted (lines 144–145) to strip anything the AI SDK may have injected (the OpenAI-compatible SDK default puts `Authorization: Bearer <apiKey>` — here `apiKey` is `""`, so those would otherwise collide case-insensitively).

#### 4.5.3 Retry logic

**There is no retry logic on the data-plane fetch.** The wrapper is a single `fetch(...)` call (line 147). The only retry loop in the file is in the OAuth device-code authorize callback (§2.5).

There is also a one-shot `AbortSignal.timeout(5_000)` on the models catalog fetch (`models.ts:115`) but that is the `/models` list endpoint, not chat.

---

## 5. Plugin Hooks: `chat.params` and `chat.headers`

File: `packages/opencode/src/plugin/github-copilot/copilot.ts:312-359`

Both hooks early-return if the model's provider ID does not include `"github-copilot"`.

### 5.1 `chat.params` (lines 312–319)

```ts
"chat.params": async (incoming, output) => {
  if (!incoming.model.providerID.includes("github-copilot")) return

  // Match github copilot cli, omit maxOutputTokens for gpt models
  if (incoming.model.api.id.includes("gpt")) {
    output.maxOutputTokens = undefined
  }
},
```

Single mutation: for any GPT model routed through Copilot, unset `maxOutputTokens` to match the official `gh copilot` CLI behavior.

### 5.2 `chat.headers` (lines 320–359)

Three distinct effects:

**a) Anthropic interleaved thinking beta** (lines 323–325):

```ts
if (incoming.model.api.npm === "@ai-sdk/anthropic") {
  output.headers["anthropic-beta"] = "interleaved-thinking-2025-05-14"
}
```

*Note: this branch is effectively dead for the Copilot provider since all Copilot-registered models have `npm: "@ai-sdk/github-copilot"` (see `models.ts:61` and the `fix()` helper at `copilot.ts:30-38`). It guards against a config path where a user points a Copilot-scoped model at the raw Anthropic SDK.*

**b) Compaction detection → force `x-initiator: agent`** (lines 327–343):

```ts
const parts = await sdk.session.message({ /* fetch message parts */ }).catch(() => undefined)

if (parts?.data.parts?.some((part) => part.type === "compaction")) {
  output.headers["x-initiator"] = "agent"
  return
}
```

If the current message is the result of context compaction, mark it as agent-initiated and short-circuit.

**c) Sub-agent session detection → force `x-initiator: agent`** (lines 345–358):

```ts
const session = await sdk.session.get({ /* fetch session */ }).catch(() => undefined)
if (!session || !session.data.parentID) return
// mark subagent sessions as agent initiated matching standard that other copilot tools have
output.headers["x-initiator"] = "agent"
```

Any session with a `parentID` is treated as a sub-agent session; the `x-initiator` is set to `"agent"` in `output.headers`, which is then merged into `init.headers` before the `fetch` wrapper runs. The `x-initiator` default in the wrapper sits *before* the `...init?.headers` spread, so the hook wins.

---

## 6. Provider Registration and `@ai-sdk/github-copilot` → `copilot` Option Mapping

### 6.1 `BUNDLED_PROVIDERS` registration (`provider.ts:127-150`)

```ts
// provider.ts:36
import { createOpenaiCompatible as createGitHubCopilotOpenAICompatible } from "./sdk/copilot"
// ...
// provider.ts:127
const BUNDLED_PROVIDERS: Record<string, (options: any) => BundledSDK> = {
  // ...
  "@ai-sdk/github-copilot": createGitHubCopilotOpenAICompatible,  // line 148
  // ...
}
```

The Copilot SDK is **not** a published npm package — it is a vendored fork of the OpenAI-compatible SDK living at `packages/opencode/src/provider/sdk/copilot/` (see `copilot-provider.ts`, `chat/`, `responses/`). The alias rename on import lets it slot into the BUNDLED map under the fake npm key `@ai-sdk/github-copilot`.

At runtime, `resolveSDK` branches on that npm key (`provider.ts:1432-1444`):

```ts
const bundledFn = BUNDLED_PROVIDERS[model.api.npm]
if (bundledFn) {
  log.info("using bundled provider", { providerID: model.providerID, pkg: model.api.npm })
  const loaded = bundledFn({
    name: model.providerID,
    ...options,
  })
  s.sdk.set(key, loaded)
  return loaded as SDK
}
```

The `options` passed here include whatever the plugin's auth `loader()` returned (`baseURL`, `apiKey: ""`, `fetch`) merged with the provider config. So the vendored `createOpenaiCompatible` (`sdk/copilot/copilot-provider.ts:52`) receives the Copilot fetch wrapper as its `options.fetch` and calls it from the `OpenAICompatibleChatLanguageModel` / `OpenAIResponsesLanguageModel`.

### 6.2 The `custom()` entry for `github-copilot` (`provider.ts:222-230`)

```ts
"github-copilot": () =>
  Effect.succeed({
    autoload: false,
    async getModel(sdk: any, modelID: string, _options?: Record<string, any>) {
      if (useLanguageModel(sdk)) return sdk.languageModel(modelID)
      return shouldUseCopilotResponsesApi(modelID) ? sdk.responses(modelID) : sdk.chat(modelID)
    },
    options: {},
  }),
```

Key points:

- **`autoload: false`** — never auto-registered from env; requires explicit OAuth credential via the plugin.
- **`getModel`** — the provider-scoped override that chooses between `sdk.chat(modelID)` and `sdk.responses(modelID)` depending on `shouldUseCopilotResponsesApi`.
- **`useLanguageModel` fallback** (`provider.ts:168-170`): if the SDK lacks both `chat` and `responses`, fall through to `sdk.languageModel(modelID)`.

### 6.3 `sdkKey()` mapping `@ai-sdk/github-copilot` → `"copilot"` (`transform.ts:24-47`)

```ts
// Maps npm package to the key the AI SDK expects for providerOptions
function sdkKey(npm: string): string | undefined {
  switch (npm) {
    case "@ai-sdk/github-copilot":
      return "copilot"
    case "@ai-sdk/azure":
      return "azure"
    case "@ai-sdk/openai":
      return "openai"
    // ...
  }
}
```

The AI SDK's `generateText`/`streamText` accept `providerOptions: { [key]: {...} }` where `key` is a short provider alias. This function translates the long bundled package id to the short alias the AI SDK internals expect. For the Copilot SDK, that key is `"copilot"`, so a call like `providerOptions: { copilot: { reasoningEffort: "high" } }` is what actually reaches the vendored Copilot responses model.

### 6.4 Other `@ai-sdk/github-copilot` branches in `transform.ts`

**Reasoning effort generation (`transform.ts:462-486`):**

```ts
case "@ai-sdk/github-copilot":
  if (model.id.includes("gemini")) {
    // currently github copilot only returns thinking
    return {}
  }
  if (model.id.includes("claude")) {
    return Object.fromEntries(WIDELY_SUPPORTED_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }]))
  }
  const copilotEfforts = iife(() => {
    if (id.includes("5.1-codex-max") || id.includes("5.2") || id.includes("5.3"))
      return [...WIDELY_SUPPORTED_EFFORTS, "xhigh"]
    const arr = [...WIDELY_SUPPORTED_EFFORTS]
    if (id.includes("gpt-5") && model.release_date >= "2025-12-04") arr.push("xhigh")
    return arr
  })
  return Object.fromEntries(
    copilotEfforts.map((effort) => [
      effort,
      {
        reasoningEffort: effort,
        reasoningSummary: "auto",
        include: ["reasoning.encrypted_content"],
      },
    ]),
  )
```

Per-model family reasoning effort variants:

- **Gemini via Copilot:** empty (thinking auto-returned)
- **Claude via Copilot:** standard low/medium/high
- **GPT-5 Codex Max / 5.2 / 5.3:** adds `"xhigh"`
- **GPT-5 ≥ 2025-12-04:** adds `"xhigh"`
- **All non-Gemini/non-Claude** get `reasoningSummary: "auto"` and `include: ["reasoning.encrypted_content"]` so reasoning traces round-trip across calls.

**Forced `store: false` (`transform.ts:752-758`):**

```ts
if (
  input.model.providerID === "openai" ||
  input.model.api.npm === "@ai-sdk/openai" ||
  input.model.api.npm === "@ai-sdk/github-copilot"
) {
  result["store"] = false
}
```

Prevents server-side response storage (zero-retention behavior).

**Small-model options (`transform.ts:864-877`):**

```ts
export function smallOptions(model: Provider.Model) {
  if (
    model.providerID === "openai" ||
    model.api.npm === "@ai-sdk/openai" ||
    model.api.npm === "@ai-sdk/github-copilot"
  ) {
    if (model.api.id.includes("gpt-5")) {
      if (model.api.id.includes("5.")) {
        return { store: false, reasoningEffort: "low" }
      }
      return { store: false, reasoningEffort: "minimal" }
    }
    return { store: false }
  }
  // ...
}
```

For small-model selection (title generation, summaries): Copilot-routed GPT-5.x gets `reasoningEffort: "low"`, plain GPT-5 gets `"minimal"`.

**Copilot small-model priority (`provider.ts:1571-1573`):**

```ts
if (providerID.startsWith("github-copilot")) {
  priority = ["gpt-5-mini", "claude-haiku-4.5", ...priority]
}
```

Pushes `gpt-5-mini` and `claude-haiku-4.5` ahead of the generic small-model priority list.

---

## 7. CLI Login Flow

File: `packages/opencode/src/cli/cmd/providers.ts`

### 7.1 Command registration (lines 199–206, 254–272)

```ts
export const ProvidersCommand = cmd({
  command: "providers",
  aliases: ["auth"],
  describe: "manage AI providers and credentials",
  builder: (yargs) =>
    yargs.command(ProvidersListCommand).command(ProvidersLoginCommand).command(ProvidersLogoutCommand).demandCommand(),
  async handler() {},
})

export const ProvidersLoginCommand = cmd({
  command: "login [url]",
  describe: "log in to a provider",
  builder: (yargs) =>
    yargs
      .positional("url", { describe: "opencode auth provider", type: "string" })
      .option("provider", { alias: ["p"], ... })
      .option("method", { alias: ["m"], ... }),
  // ...
})
```

### 7.2 Provider list assembly (lines 313–361)

```ts
const priority: Record<string, number> = {
  opencode: 0,
  openai: 1,
  "github-copilot": 2,   // line 326
  google: 3,
  anthropic: 4,
  openrouter: 5,
  vercel: 6,
}
```

`github-copilot` is at position 2 in the sorted dropdown. The list is a union of (a) providers from the `ModelsDev` database sorted by this priority and (b) plugin-only providers filtered so they don't duplicate database entries.

### 7.3 Provider selection (lines 363–388)

Either `--provider github-copilot` / `-p github-copilot` matches directly, or the user picks it from the interactive `prompts.autocomplete`.

### 7.4 Delegation to the Copilot plugin (lines 390–394)

```ts
const plugin = await Plugin.list().then((x) => x.findLast((x) => x.auth?.provider === provider))
if (plugin && plugin.auth) {
  const handled = await handlePluginAuth({ auth: plugin.auth }, provider, args.method)
  if (handled) return
}
```

`findLast` over `Plugin.list()` picks the most recently registered plugin whose `auth.provider === "github-copilot"` — that is the `CopilotAuthPlugin` exported from `plugin/github-copilot/copilot.ts`. Control transfers to `handlePluginAuth`.

### 7.5 `handlePluginAuth` (lines 19–170)

1. **Method selection** (lines 19–43): Copilot only registers one method (`"Login with GitHub Copilot"`), so `plugin.auth.methods.length > 1` is false and index `0` is used directly. `--method <label>` also accepted.

2. **Prompt loop** (lines 46–73): drives the two prompts declared in `copilot.ts:158-193`:
   - Select `deploymentType`: `github.com` or `enterprise`
   - If enterprise, conditional text prompt for `enterpriseUrl` (gated by `when`, with URL/domain validator)

3. **OAuth branch** (lines 75–148): for `method.type === "oauth"`:
   - Calls `method.authorize(inputs)` (this runs §2.3 — device code request)
   - Prints `Go to: <verification_uri>` and `Enter code: <user_code>`
   - Spins a spinner and awaits `authorize.callback()` (this runs §2.5 — the polling loop)
   - On success, calls `Auth.set("github-copilot", { type: "oauth", refresh, access, expires, enterpriseUrl? })`

---

## 8. Design Implications for fspec

This section translates the research above into concrete decisions for fspec's own Copilot provider.

### 8.1 Must-haves (core feature parity)

1. **OAuth device flow** with the GitHub OAuth App client id `Ov23li8tweQw6odWQebz` (opencode uses this same public ID — we should register our own OAuth App but the flow is identical).
   - POST `https://github.com/login/device/code` with `scope: "read:user"`.
   - Poll `https://github.com/login/oauth/access_token` with `grant_type: urn:ietf:params:oauth:grant-type:device_code`.
   - Handle `authorization_pending` (continue), `slow_down` (RFC 8628 §3.5: +5s or server interval), any other error (fail).
   - Add a 3-second safety margin to every poll sleep.
2. **Enterprise support**: a `deploymentType` select (`github.com` | `enterprise`) plus a conditional `enterpriseUrl` text input. Persist `enterpriseUrl` alongside the token.
3. **Token storage**: persist the GitHub access token as opaque (`expires: 0`), under provider key `github-copilot`. No refresh attempts.
4. **Request headers**:
   - `Authorization: Bearer <token>` (always)
   - `User-Agent: fspec/<version>` (always, overrides any caller-supplied UA)
   - `Openai-Intent: conversation-edits` (always, hard-coded — this is the magic header that makes the API treat us as Copilot Chat)
   - `x-initiator: agent|user` (derived from request body; promoted to `agent` for compaction/sub-agent contexts)
   - `Copilot-Vision-Request: true` when the request contains images
   - **Strip** `authorization` (lowercase) and `x-api-key` from any caller-supplied headers.
5. **Base URL resolution**:
   - `https://api.githubcopilot.com` for github.com
   - `https://copilot-api.<normalized-enterprise-domain>` for enterprise
6. **Model catalog**: GET `{baseURL}/models` with the same bearer + UA headers, parse the response schema, filter by `model_picker_enabled === true`, merge against any static catalog we ship. Treat Copilot `cost` as 0 because billing is per-seat on GitHub's side.
7. **Endpoint routing**: implement `shouldUseCopilotResponsesApi(modelId)` — GPT ≥ 5 except `gpt-5-mini` goes to `/responses`; everything else goes to `/chat/completions`.

### 8.2 Should-haves (nice to have, low effort)

1. **`chat.params` override** to unset `max_tokens`/`maxOutputTokens` for GPT family models via Copilot.
2. **Agent detection**: examine the outgoing body to set `x-initiator` correctly (promotes to `agent` when the last message/input role is `assistant`, or when the request is Anthropic-shaped and all last-user parts are `tool_result`).
3. **Small-model priority**: if fspec has a "fast model for title/summary" concept, prefer `gpt-5-mini` → `claude-haiku-4.5` for Copilot.
4. **`reasoning_opaque` round-tripping**: preserve the `choice.message.reasoning_opaque` field in provider metadata across turns so multi-turn reasoning continuity works.

### 8.3 Won't-haves (explicitly out of scope)

1. **Token refresh**: there is nothing to refresh. The only recovery path is "re-run login".
2. **Copilot quota/billing endpoint**: opencode does not call one, and there is no stable documented endpoint for per-user seat usage.
3. **`Copilot-Integration-Id`, `Editor-Version`, `Editor-Plugin-Version`, `X-GitHub-Api-Version`, `Openai-Organization` headers**: opencode omits all of these and the API accepts requests fine without them.
4. **Non-`read:user` scopes**: do not request anything more. The API works with `read:user` alone.

### 8.4 Risks and open questions

1. **OAuth App ID**: do we reuse opencode's `Ov23li8tweQw6odWQebz` or register a new GitHub OAuth App under the `fspec` org? (Reusing is legally dubious and couples our TOS to theirs — we should register our own.)
2. **Terms of service**: GitHub Copilot's TOS technically restricts usage to "approved editor integrations". opencode ships with this anyway; we should document the risk in the foundation doc before shipping.
3. **Rate limiting**: no retry logic in opencode's data-plane fetch. We should probably add at least one retry with exponential backoff for 429/503 to be a better citizen.
4. **Model catalog freshness**: Copilot adds/removes models frequently. We need a refresh strategy — opencode fetches on every `loader()` call (once per provider init). We could fetch-with-cache + stale-while-revalidate.
5. **Enterprise `copilot-api.<domain>` DNS**: not all GHE installs have this subdomain configured. We should document clearly how to verify.

---

## 9. Appendix: File Index for the Reference Implementation

Absolute paths in the cloned repo at `/tmp/opencode/`:

```
packages/opencode/src/plugin/github-copilot/copilot.ts
packages/opencode/src/plugin/github-copilot/models.ts
packages/opencode/src/plugin/index.ts
packages/opencode/src/auth/index.ts
packages/opencode/src/provider/provider.ts
packages/opencode/src/provider/schema.ts
packages/opencode/src/provider/transform.ts
packages/opencode/src/provider/sdk/copilot/copilot-provider.ts
packages/opencode/src/provider/sdk/copilot/chat/openai-compatible-chat-language-model.ts
packages/opencode/src/provider/sdk/copilot/chat/convert-to-openai-compatible-chat-messages.ts
packages/opencode/src/provider/sdk/copilot/responses/openai-responses-language-model.ts
packages/opencode/src/cli/cmd/providers.ts
packages/opencode/test/plugin/github-copilot-models.test.ts
packages/opencode/test/provider/copilot/convert-to-copilot-messages.test.ts
packages/opencode/test/provider/copilot/copilot-chat-model.test.ts
packages/opencode/test/provider/transform.test.ts
```
