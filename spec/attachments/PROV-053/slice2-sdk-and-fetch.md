# PROV-053 Slice 2: Provider SDK, Custom Fetch & Endpoint Routing

**Scope:** HTTP/SDK layer only. OAuth/storage and model-catalog/options mapping are handled by parallel slices.

**Reference:** `/tmp/opencode/` (anomalyco/opencode fork)
**Target:** `/Users/rquast/projects/fspec/` — specifically `codelet/providers/` (Rust workspace) plus NAPI bindings to the TUI layer.

---

## 1. Reference Architecture (opencode)

### 1.1 Vendored Copilot SDK directory layout

opencode forks the official `@ai-sdk/openai-compatible` package into `packages/opencode/src/provider/sdk/copilot/` and keeps it **source-local** rather than published. The SDK is registered under the fake npm key `@ai-sdk/github-copilot` so it slots into the same `BUNDLED_PROVIDERS` map as the real SDKs.

Directory tree at `/tmp/opencode/packages/opencode/src/provider/sdk/copilot/`:

| Path | Purpose |
|---|---|
| `README.md` | Warning: "Avoid making changes to these files unless you only want to affect the Copilot provider." |
| `index.ts` | Re-exports `createOpenaiCompatible`, `openaiCompatible`, and the `OpenaiCompatibleProvider` / `OpenaiCompatibleProviderSettings` types. |
| `copilot-provider.ts` | Top-level factory `createOpenaiCompatible(options)` returning `{chat, responses, languageModel}` factories. |
| `openai-compatible-error.ts` | Zod schema for OpenAI-compatible error envelopes + `defaultOpenAICompatibleErrorStructure`. |
| `chat/openai-compatible-chat-language-model.ts` | `OpenAICompatibleChatLanguageModel` — `POST {baseURL}/chat/completions` implementation (~28 KB). |
| `chat/convert-to-openai-compatible-chat-messages.ts` | Converts AI-SDK prompts → OpenAI chat-message wire format. |
| `chat/openai-compatible-chat-options.ts` | Schema/types for chat options. |
| `chat/openai-compatible-api-types.ts` | Wire-level TS types for chat-completions req/resp. |
| `chat/openai-compatible-prepare-tools.ts` | Tool/function-call schema prep for chat-completions. |
| `chat/openai-compatible-metadata-extractor.ts` | Provider-specific metadata extractor interface. |
| `chat/map-openai-compatible-finish-reason.ts` | Maps OpenAI finish_reason → AI SDK finish reasons. |
| `chat/get-response-metadata.ts` | Extracts id/model/timestamp from chat responses. |
| `responses/openai-responses-language-model.ts` | `OpenAIResponsesLanguageModel` — `POST {baseURL}/responses` implementation (~59 KB). |
| `responses/convert-to-openai-responses-input.ts` | Converts AI-SDK prompts → `/responses` input items. |
| `responses/openai-responses-api-types.ts` | Wire-level TS types for `/responses` req/resp. |
| `responses/openai-responses-prepare-tools.ts` | Tool schema prep for `/responses`. |
| `responses/openai-responses-settings.ts` | Settings types. |
| `responses/openai-config.ts` | Per-call config / header helpers. |
| `responses/openai-error.ts` | Zod schema for `/responses` errors. |
| `responses/map-openai-responses-finish-reason.ts` | Finish-reason mapper. |
| `responses/tool/web-search.ts`, `web-search-preview.ts`, `code-interpreter.ts`, `local-shell.ts`, `file-search.ts`, `image-generation.ts` | Built-in responses-API tool definitions. |

**Key observation:** the SDK has NO embedding model and NO image model — both are commented out in `copilot-provider.ts:44-46`.

### 1.2 `createOpenaiCompatible` factory (copilot-provider.ts)

**Input interface** (`copilot-provider.ts:11-36`):

```ts
export interface OpenaiCompatibleProviderSettings {
  apiKey?: string     // line 15
  baseURL?: string    // line 20
  name?: string       // line 25
  headers?: Record<string, string>  // line 30
  fetch?: FetchFunction              // line 35
}
```

**Factory body** (`copilot-provider.ts:52-96`):

- L53: `baseURL = withoutTrailingSlash(options.baseURL ?? "https://api.openai.com/v1")`
- L60-64: Merges default headers — `{ ...(options.apiKey && { Authorization: 'Bearer ${apiKey}' }), ...options.headers }`. If `apiKey === ""` (opencode's case), the spread-short-circuit skips the `Authorization` default entirely.
- L66: `getHeaders = () => withUserAgentSuffix(headers, "ai-sdk/openai-compatible/${VERSION}")`
- L68-75: `createChatModel(modelId)` → `new OpenAICompatibleChatLanguageModel(modelId, { provider: "${name}.chat", headers: getHeaders, url: ({path}) => baseURL + path, fetch: options.fetch })`
- L77-84: `createResponsesModel(modelId)` → same shape, but `OpenAIResponsesLanguageModel` and `.responses` suffix.
- L86: `createLanguageModel` is **just an alias** for `createChatModel`.
- L88-96: Returns a callable function with `.languageModel`, `.chat`, `.responses` attached.

**Return type** (`copilot-provider.ts:38-47`):

```ts
export interface OpenaiCompatibleProvider {
  (modelId): LanguageModelV3
  chat(modelId): LanguageModelV3
  responses(modelId): LanguageModelV3
  languageModel(modelId): LanguageModelV3
}
```

### 1.3 Custom `fetch` wrapper (copilot.ts:60-155)

Lives in `packages/opencode/src/plugin/github-copilot/copilot.ts`, injected as the `fetch` field of the object returned by `auth.loader(getAuth)`:

```ts
auth: {
  provider: "github-copilot",
  async loader(getAuth) {                     // L65
    const info = await getAuth()              // L66
    if (!info || info.type !== "oauth") return {}
    const baseURL = base(info.enterpriseUrl)  // L69
    return {
      baseURL,                                 // L72
      apiKey: "",                              // L73 — empty, forces wrapper to own auth
      async fetch(request, init) {             // L74 — THE custom fetch
        const info = await getAuth()           // L75 — re-read on every request
        // ... classification + headers ...
      },
    }
  },
}
```

`apiKey: ""` at L73 is deliberate: it prevents `copilot-provider.ts:62` from injecting its own stale `Authorization` header, leaving the custom wrapper as the sole auth source.

#### 1.3.1 Body-shape classification → `isVision` / `isAgent`

Inside an IIFE at L79-130. Parses `init.body` as JSON and walks **three mutually exclusive shapes**:

**Shape 1 — Completions API** (L83-93):
- Guard: `body.messages && url.includes("completions")`.
- `isVision`: any message has array content containing a part with `type === "image_url"`.
- `isAgent`: `last.role !== "user"` (the last turn is an assistant tool-result continuation).

**Shape 2 — Responses API** (L95-105):
- Guard: `body.input` exists.
- `isVision`: any input item has array content containing `type === "input_image"`.
- `isAgent`: `last.role !== "user"` on `body.input`.

**Shape 3 — Anthropic Messages API** (L107-127):
- Guard: `body.messages` present but not a completions URL.
- `isVision`: any message has array content containing `type === "image"` **OR** a `tool_result` whose nested `content[]` contains an `image` part (Anthropic nests images inside tool results).
- `isAgent`: `!(last.role === "user" && hasNonToolCalls)` where `hasNonToolCalls = last.content.some(p => p.type !== "tool_result")`. In other words, a user message that is ONLY tool_result parts counts as agent-initiated.

**Fallback** (L128-130): `catch {}; return { isVision: false, isAgent: false }`. Classification is best-effort; the body is NOT mutated in any branch.

#### 1.3.2 Header table (L132-142)

```ts
const headers: Record<string, string> = {
  "x-initiator": isAgent ? "agent" : "user",    // L133 — FIRST (overridable)
  ...(init?.headers as Record<string, string>), // L134 — caller/chat.headers hook
  "User-Agent": `opencode/${Installation.VERSION}`, // L135 — WINS
  Authorization: `Bearer ${info.refresh}`,       // L136 — WINS
  "Openai-Intent": "conversation-edits",         // L137 — WINS
}
if (isVision) {                                  // L140
  headers["Copilot-Vision-Request"] = "true"
}
delete headers["x-api-key"]       // L144
delete headers["authorization"]   // L145 — lowercase variant
return fetch(request, { ...init, headers })  // L147-150
```

| Header | Value | Rule | Line |
|---|---|---|---|
| `x-initiator` | `"agent"` or `"user"` | Default from body heuristic. Position: BEFORE spread, so `chat.headers` hook values (spread next) override it. | 133 |
| `User-Agent` | `opencode/${version}` | ALWAYS overrides caller-supplied UA. | 135 |
| `Authorization` | `Bearer ${info.refresh}` | ALWAYS overrides. Uses the stored OAuth "refresh" slot (opencode stores both in `refresh` and `access` fields — see research doc §2.5). | 136 |
| `Openai-Intent` | `conversation-edits` | ALWAYS, hard-coded. This is the magic header that routes the request as Copilot Chat. | 137 |
| `Copilot-Vision-Request` | `"true"` | ONLY when `isVision` is truthy. | 141 |

**Explicitly NOT set:** `Copilot-Integration-Id`, `Editor-Version`, `Editor-Plugin-Version`, `X-GitHub-Api-Version`, `Openai-Organization`.

**Why `delete headers["authorization"]` AND `delete headers["x-api-key"]`:**

1. `headers` is a plain `Record<string,string>`, not a `Headers` instance — so `"Authorization"` and `"authorization"` are two different keys in a plain JS object.
2. The spread on L134 may include a lowercase `authorization` key left over by AI-SDK base classes (notably `@ai-sdk/anthropic`, which sets `x-api-key`).
3. Without the lowercase delete, `fetch()` will normalize both entries into a `Headers` instance and depending on runtime either duplicate the header or the lowercase (stale) one wins — overriding the GitHub bearer token.
4. `x-api-key` is scrubbed because the Anthropic provider injects it; the Copilot API does not expect it.

#### 1.3.3 Retry logic

**There is none.** L147-150 is a single `fetch(request, {...init, headers})` call. No try/catch, no status check, no backoff. The wrapper is pure header-manipulation; all retry/timeout behavior is inherited from the AI SDK base classes upstream.

### 1.4 Endpoint routing — `shouldUseCopilotResponsesApi`

`packages/opencode/src/provider/provider.ts:63-67`:

```ts
function shouldUseCopilotResponsesApi(modelID: string): boolean {
  const match = /^gpt-(\d+)/.exec(modelID)
  if (!match) return false
  return Number(match[1]) >= 5 && !modelID.startsWith("gpt-5-mini")
}
```

**Routing table:**

| Model ID | Route | Reason |
|---|---|---|
| `gpt-5`, `gpt-5.1`, `gpt-5.2`, `gpt-5.2-codex`, `gpt-6*` | `/responses` | Regex `^gpt-(\d+)` captures leading int, `>= 5`, not `gpt-5-mini`. |
| `gpt-5-mini`, `gpt-5-mini-*` | `/chat/completions` | Excluded by `startsWith("gpt-5-mini")`. |
| `gpt-4`, `gpt-4o`, `gpt-3.5-*` | `/chat/completions` | Captured int `< 5`. |
| `claude-*`, `gemini-*`, `o1`, `o3` | `/chat/completions` | Regex fails (no `gpt-` prefix). |

**Single call site** — inside the `"github-copilot"` entry of `custom(dep)` at `provider.ts:222-230`:

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

`useLanguageModel(sdk)` at `provider.ts:168-170` is a fallback check: `sdk.responses === undefined && sdk.chat === undefined`. For our vendored SDK both exist, so the fallback never fires.

### 1.5 Plugin hooks — `chat.params` & `chat.headers`

Both live in `copilot.ts:312-359`. Both early-return unless `incoming.model.providerID.includes("github-copilot")`.

**`chat.params`** (L312-319) performs ONE mutation:

```ts
if (incoming.model.api.id.includes("gpt")) {
  output.maxOutputTokens = undefined
}
```

Purpose: match the official `gh copilot` CLI, which omits `max_tokens` for GPT family models.

**`chat.headers`** (L320-359) has three effects:

1. **Anthropic interleaved thinking** (L323-325): if the underlying SDK is `@ai-sdk/anthropic`, set `anthropic-beta: interleaved-thinking-2025-05-14`. Effectively dead for the vendored Copilot SDK (its `npm` key is `@ai-sdk/github-copilot`), kept as a safety net for mis-config.

2. **Compaction detection** (L327-343):
   ```ts
   const parts = await sdk.session.message({
     path: { id: incoming.message.sessionID, messageID: incoming.message.id },
     query: { directory: input.directory },
     throwOnError: true,
   }).catch(() => undefined)
   if (parts?.data.parts?.some(p => p.type === "compaction")) {
     output.headers["x-initiator"] = "agent"
     return
   }
   ```
   The parts list comes from the opencode **SDK session client** (`sdk.session.message(...)`), which queries the in-process opencode server for the message's parts. If any part is a `compaction` part, mark the request as agent-initiated and short-circuit.

3. **Sub-agent (parentID) detection** (L345-358):
   ```ts
   const session = await sdk.session.get({
     path: { id: incoming.sessionID },
     query: { directory: input.directory },
     throwOnError: true,
   }).catch(() => undefined)
   if (!session || !session.data.parentID) return
   output.headers["x-initiator"] = "agent"
   ```
   Any session whose `parentID` is set is a sub-agent session; promote to agent-initiated.

The `output.headers["x-initiator"]` values set here are merged into `init.headers` before the custom fetch runs. In the fetch wrapper, `x-initiator` sits at L133 — BEFORE the `init.headers` spread at L134 — so the hook value **wins** over the body heuristic default.

### 1.6 `BUNDLED_PROVIDERS` registration

`provider.ts:36`:
```ts
import { createOpenaiCompatible as createGitHubCopilotOpenAICompatible } from "./sdk/copilot"
```

`provider.ts:127-150`:
```ts
const BUNDLED_PROVIDERS: Record<string, (options: any) => BundledSDK> = {
  // ... 17 other entries ...
  "@ai-sdk/github-copilot": createGitHubCopilotOpenAICompatible,  // L148
  // ...
}
```

`resolveSDK` look-up (`provider.ts:1432-1444`):
```ts
const bundledFn = BUNDLED_PROVIDERS[model.api.npm]
if (bundledFn) {
  log.info("using bundled provider", { providerID: model.providerID, pkg: model.api.npm })
  const loaded = bundledFn({
    name: model.providerID,   // "github-copilot"
    ...options,               // { baseURL, apiKey: "", fetch } from auth.loader
  })
  s.sdk.set(key, loaded)
  return loaded as SDK
}
```

The `options` spread here is the object returned from the plugin's `auth.loader()` merged with any provider-level config. In practice that means `createOpenaiCompatible` receives `{ name: "github-copilot", baseURL: "https://api.githubcopilot.com", apiKey: "", fetch: <customWrapper> }` — exactly what the SDK needs to delegate every HTTP call to the wrapper.

---

## 2. fspec Current State

### 2.1 Provider layer lives in Rust, not TypeScript

fspec has a **Rust workspace** at `codelet/providers/` that owns all LLM HTTP. TypeScript under `src/` never issues outbound LLM HTTP requests directly; it calls NAPI bindings that invoke the Rust providers. Verified by searching `src/` for `fetch(`, `axios`, `openai`, `@ai-sdk/` — the only hits are:

- `src/utils/provider-config.ts` — **configuration metadata only** (base URLs, auth methods, registry entries for a dozen providers). No HTTP.
- `src/tui/**` — TUI components that read provider config and drive OAuth UX.
- Tests only.

There is **no `src/providers/` or `src/auth/` directory** in the TypeScript tree and no TypeScript AI SDK integration. This is important: porting opencode's TypeScript vendored SDK verbatim would be the wrong layer.

### 2.2 Rust provider module layout (`codelet/providers/src/`)

From `codelet/providers/src/lib.rs:1-56`:

```rust
pub mod adapter;
pub mod cache_optimization;
pub mod cache_token_extractor;
pub mod caching_client;
pub mod claude;
pub mod claude_auth;
pub mod claude_oauth;
pub mod claude_headless_login;
pub mod claude_refreshing_client;
pub mod claude_oauth_server;
pub mod codex;            // subdirectory with mod.rs, refreshing_client.rs, codex_oauth.rs, ...
mod credentials;
pub mod error;
pub mod gemini;
mod manager;
pub mod models;
pub mod oauth_crypto;
pub mod oauth_http_utils;
pub mod openai;
pub mod zai;
```

Each provider is a single `.rs` (except `codex/` and `models/` which are submodules). Line counts:

- `claude.rs` — 818 lines (largest; OAuth + API key + cache control)
- `openai.rs` — 583 lines (PROV-006 local-server support, PROV-051 session affinity)
- `codex/mod.rs` — 721 lines (Responses API via rewritten URL)
- `zai.rs` — 411 lines (newest, OpenAI-compatible via custom base_url — **best template for Copilot**)
- `gemini.rs` — 342 lines (native Gemini client)
- `adapter.rs` — 328 lines (shared conversion helpers)

### 2.3 The `LlmProvider` trait (`lib.rs:83-113`)

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool;
    fn supports_streaming(&self) -> bool;

    async fn complete(
        &self,
        messages: &[codelet_common::Message],
    ) -> Result<String, ProviderError>;

    async fn complete_with_tools(
        &self,
        messages: &[codelet_common::Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError>;
}
```

All five existing providers (`ClaudeProvider`, `OpenAIProvider`, `CodexProvider`, `GeminiProvider`, `ZAIProvider`) implement this. They all wrap a **`rig`-crate** client internally.

### 2.4 The HTTP middleware pattern: `RefreshingClaudeClient` / `RefreshingCodexClient`

fspec already has **exactly the pattern we need** for the Copilot custom fetch — just in Rust. Both of the following implement `rig::http_client::HttpClientExt`, which is `rig`'s trait for swapping the underlying HTTP backend at the client level.

**`RefreshingClaudeClient`** (`codelet/providers/src/claude_refreshing_client.rs`):
- Wraps a `reqwest::Client` (L58).
- Two modes: `OAuth { token_state }` (L44) and `ApiKey` (L48, pass-through).
- OAuth mode checks expiry with a 30s buffer (L27), double-check-locks to refresh (L119-168), then on every request calls `prepare_oauth_request` (L198-213):
  - `parts.headers.remove(http::header::AUTHORIZATION)` — strip stale.
  - `parts.headers.insert(http::header::AUTHORIZATION, "Bearer ${access_token}".parse())` — inject fresh.
- Implements `HttpClientExt::send`, `send_multipart`, `send_streaming` (L215-297) — all three variants delegate to the same token-refresh + header-inject pipeline.
- **No URL rewriting, no extra headers beyond Authorization.**

**`RefreshingCodexClient`** (`codelet/providers/src/codex/refreshing_client.rs`):
- Same shape as the Claude client but ALSO rewrites URLs and injects extra headers.
- `prepare_oauth_request` (L205-236):
  - Calls `rewrite_codex_url(&original_url)` to translate any `/v1/responses`, `/responses`, or `/chat/completions` URL to `https://chatgpt.com/backend-api/codex/responses` (`codex_oauth.rs:152-167`).
  - Strips `Authorization`, injects `Bearer ${access_token}`, sets `ChatGPT-Account-Id: {account_id}` and `originator: codelet` (L222-233).

**This is the precise shape Copilot needs.** The only differences are:
1. Copilot has NO token refresh (opaque GitHub OAuth token, `expires: 0` — see research doc §3.2). So the `token_state` can be a simple `Arc<RwLock<String>>` holding the bearer, not a full refresh state machine.
2. Copilot needs **body inspection** (isVision / isAgent) before header assembly, which neither existing client does.
3. Copilot needs conditional `Copilot-Vision-Request`, `Openai-Intent`, and dynamic `x-initiator` headers.
4. Copilot needs URL-aware endpoint routing (chat vs responses). `rig::providers::openai` has both `CompletionModel` (chat/completions) AND `ResponsesCompletionModel`, as demonstrated by Codex (`codex/mod.rs:24: type CodexResponsesModel = openai::responses_api::ResponsesCompletionModel<RefreshingCodexClient>`). So Copilot can pick at model-build time which `rig` model class to instantiate per model ID, rather than rewriting URLs in the client.

### 2.5 How ZAI uses rig's OpenAI-compatible client (`zai.rs:23-100`)

ZAI is the cleanest template:

```rust
const ZAI_API_BASE_URL: &str = "https://api.z.ai/api/paas/v4";
const ZAI_PLAN_API_BASE_URL: &str = "https://api.z.ai/api/coding/paas/v4";

pub struct ZAIProvider {
    completion_model: openai::completion::CompletionModel,
    rig_client: openai::CompletionsClient,
    model_name: String,
    is_plan_endpoint: bool,
}
```

Construction path:
```rust
openai::CompletionsClient::builder()
    .api_key(api_key)
    .base_url(ZAI_API_BASE_URL /* or ZAI_PLAN_API_BASE_URL */)
    .build()?
```

ZAI uses rig's built-in `reqwest` HTTP backend directly. There is no custom `HttpClientExt`, no fetch wrapper — just a custom `base_url`. This works because ZAI accepts a static bearer token via the standard `Authorization` header that rig sets.

**Copilot cannot use this shape directly** because (a) we need dynamic per-request headers and body-shape classification, and (b) we need endpoint routing between two different rig model classes. So Copilot will combine **ZAI's `base_url` pattern** (custom endpoint) with **Codex's `HttpClientExt` pattern** (custom client with header injection).

### 2.6 Provider manager & dispatch (`manager.rs`)

`ProviderType` enum (`manager.rs:20-27`):
```rust
pub enum ProviderType {
    Claude,
    OpenAI,
    Codex,
    Gemini,
    ZAI,
}
```

- `FromStr` (L28-45): maps lowercase strings to variants.
- `has_credentials(&ProviderCredentials)` (L60-68): checks credential presence per variant.
- `map_provider_id_to_type(provider_id)` (L330-337): maps models.dev provider IDs (`"anthropic"`, `"openai"`, `"google"`, `"zai"`, `"codex"`) to `ProviderType`.
- `get_claude`, `get_openai`, `get_codex`, `get_gemini`, `get_zai` accessor methods construct the respective provider struct on demand.
- `context_window()` and `max_output_tokens()` (L533-560) match on `current_provider` and return per-provider constants.

Adding Copilot requires: a new `ProviderType::GitHubCopilot` variant, `FromStr` entry (`"github-copilot"`), credential check, `get_github_copilot()` accessor, context/output tokens arms.

### 2.7 NAPI exposure to TypeScript

`codelet/napi/src/credentials/resolver.rs:14-39` lists `get_provider_env_vars` for known provider IDs. Copilot is **not in that list** — it will need to be added (opencode uses `"github-copilot"` as the key). But since Copilot stores its token in an opencode-style `auth.json` rather than an env var, this mapping may return `None` and the resolver will fall through to a new disk-read path (handled by the parallel OAuth/storage slice — out of scope here).

`codelet/providers/src/credentials.rs:7-15` has a `ProviderCredentials` struct with hard-coded booleans per provider. That struct also needs a `copilot_available` field — but again, that's the storage slice's problem.

TypeScript side: `src/utils/provider-config.ts:77-94` has a `SUPPORTED_PROVIDERS` const array. `src/utils/agentRegistry.ts:92-104` already has a `copilot` entry but that's for the GitHub-Copilot **editor extension** as a TARGET agent, not as an upstream provider. The name collision will need disambiguation (likely `"github-copilot"` for the provider and keep `"copilot"` for the agent registry).

### 2.8 There is no TS hook system for `chat.params` / `chat.headers`

The fspec hook system at `src/hooks/` and the `runCommandWithHooks` wrapper are for **fspec CLI command lifecycle hooks** (pre/post hooks for state transitions), not for LLM request mutation. There is no analogue to opencode's `chat.params` / `chat.headers` plugin hooks anywhere in fspec. The `chat.params` / `chat.headers` mutations must therefore be implemented **inline inside the Rust Copilot provider**, not as a pluggable hook layer. That is a conscious scope reduction for slice 2.

---

## 3. Proposed fspec Design

### 3.1 New module layout

```
codelet/providers/src/
├── github_copilot/
│   ├── mod.rs                      # GitHubCopilotProvider struct + LlmProvider impl
│   ├── refreshing_client.rs        # CopilotHttpClient (HttpClientExt impl)
│   ├── body_classify.rs            # isVision / isAgent body inspection
│   └── endpoint.rs                 # should_use_copilot_responses_api(&str) -> bool
```

Naming follows the `codex/` submodule convention. **This slice does NOT touch:**

- `copilot_auth.rs` / `copilot_oauth.rs` / `copilot_device_auth.rs` — owned by the OAuth/storage slice.
- `github_copilot/models.rs` or `models/registry.rs` — owned by the model catalog slice.

### 3.2 Public API surface (signatures only — NO implementation)

**`codelet/providers/src/github_copilot/mod.rs`:**

```rust
use async_trait::async_trait;
use rig::providers::openai;
use crate::{LlmProvider, ProviderAdapter, ProviderError, CompletionResponse};

pub const DEFAULT_BASE_URL: &str = "https://api.githubcopilot.com";
pub const DEFAULT_CONTEXT_WINDOW: usize = 128_000;
pub const DEFAULT_MAX_OUTPUT_TOKENS: usize = 16_384;

pub const CONTEXT_WINDOW: usize = DEFAULT_CONTEXT_WINDOW;
pub const MAX_OUTPUT_TOKENS: usize = DEFAULT_MAX_OUTPUT_TOKENS;

/// Which rig model class is active for this provider instance.
#[derive(Clone)]
enum CopilotModel {
    Chat(openai::completion::CompletionModel),
    Responses(openai::responses_api::ResponsesCompletionModel<CopilotHttpClient>),
}

#[derive(Clone)]
pub struct GitHubCopilotProvider {
    model: CopilotModel,
    rig_client_chat: Option<openai::Client<CopilotHttpClient>>,
    rig_client_responses: Option<openai::Client<CopilotHttpClient>>,
    model_name: String,
    base_url: String,
}

impl GitHubCopilotProvider {
    /// Construct from a stored bearer token + enterprise URL.
    /// `model_id` drives the chat-vs-responses routing.
    pub fn from_token(
        access_token: &str,
        enterprise_url: Option<&str>,
        model_id: &str,
    ) -> Result<Self, ProviderError>;

    /// Session-aware constructor (parity with PROV-051 OpenAI session affinity).
    pub fn from_token_with_session(
        access_token: &str,
        enterprise_url: Option<&str>,
        model_id: &str,
        session_id: uuid::Uuid,
    ) -> Result<Self, ProviderError>;

    /// Get the configured rig client (chat or responses, whichever is active).
    pub fn model_name(&self) -> &str;
    pub fn base_url(&self) -> &str;
}

impl ProviderAdapter for GitHubCopilotProvider {
    fn provider_name(&self) -> &'static str { "github-copilot" }
}

#[async_trait]
impl LlmProvider for GitHubCopilotProvider {
    fn name(&self) -> &str;
    fn model(&self) -> &str;
    fn context_window(&self) -> usize;
    fn max_output_tokens(&self) -> usize;
    fn supports_caching(&self) -> bool { false }
    fn supports_streaming(&self) -> bool { true }

    async fn complete(
        &self,
        messages: &[codelet_common::Message],
    ) -> Result<String, ProviderError>;

    async fn complete_with_tools(
        &self,
        messages: &[codelet_common::Message],
        tools: &[ToolDefinition],
    ) -> Result<CompletionResponse, ProviderError>;
}
```

**`codelet/providers/src/github_copilot/endpoint.rs`:**

```rust
/// Exact port of opencode `shouldUseCopilotResponsesApi`.
/// See /tmp/opencode/packages/opencode/src/provider/provider.ts:63-67.
pub fn should_use_copilot_responses_api(model_id: &str) -> bool {
    use regex::Regex;
    // OnceCell-cached in real impl
    let re = Regex::new(r"^gpt-(\d+)").unwrap();
    let captures = match re.captures(model_id) {
        Some(c) => c,
        None => return false,
    };
    let n: u32 = captures[1].parse().unwrap_or(0);
    n >= 5 && !model_id.starts_with("gpt-5-mini")
}

/// Base URL selection for github.com vs GitHub Enterprise.
/// Parity: /tmp/opencode/packages/opencode/src/plugin/github-copilot/copilot.ts:26-28.
pub fn resolve_base_url(enterprise_url: Option<&str>) -> String {
    match enterprise_url {
        None => "https://api.githubcopilot.com".to_string(),
        Some(url) => {
            let normalized = url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
                .trim_end_matches('/');
            format!("https://copilot-api.{normalized}")
        }
    }
}
```

**`codelet/providers/src/github_copilot/body_classify.rs`:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RequestClassification {
    pub is_vision: bool,
    pub is_agent: bool,
}

/// Inspect a serialized request body and URL, mirroring opencode's three-shape walk
/// at /tmp/opencode/packages/opencode/src/plugin/github-copilot/copilot.ts:79-130.
/// Returns `Default::default()` on any parse failure.
pub fn classify(url: &str, body_bytes: &[u8]) -> RequestClassification;
```

Shape detection logic is a straight 1:1 port of opencode L83-127:

| Shape | Guard | isVision predicate | isAgent predicate |
|---|---|---|---|
| Completions | `url.contains("completions") && body.messages` | any message content part `type == "image_url"` | `last.role != "user"` |
| Responses | `body.input` exists | any input content part `type == "input_image"` | `last.role != "user"` on body.input |
| Messages (Anthropic) | `body.messages` (and not completions URL) | any part `type == "image"` OR `type == "tool_result"` with nested `type == "image"` | `!(last.role == "user" && any(p => p.type != "tool_result"))` |

**`codelet/providers/src/github_copilot/refreshing_client.rs`:**

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

/// Wraps a reqwest::Client and implements rig::http_client::HttpClientExt.
/// On every send, classifies the body, assembles Copilot headers, strips
/// stale auth, and injects the fresh bearer.
#[derive(Debug, Clone)]
pub struct CopilotHttpClient {
    inner: reqwest::Client,
    token: Arc<RwLock<String>>,
    user_agent: String,
    /// PROV-053 sub-agent hint (parity with opencode chat.headers hook).
    /// If set, forces `x-initiator: agent` regardless of body heuristic.
    force_agent: Arc<std::sync::atomic::AtomicBool>,
}

impl CopilotHttpClient {
    pub fn new(access_token: String, user_agent: String) -> Self;

    /// Hot-reload the bearer token (called by the storage slice when auth.json changes).
    pub async fn set_token(&self, new_token: String);

    /// Promote subsequent requests to `x-initiator: agent`.
    /// Called by the provider when it detects a sub-agent session or compaction.
    pub fn force_agent_initiator(&self, forced: bool);
}

impl rig::http_client::HttpClientExt for CopilotHttpClient {
    fn send<T, U>(&self, req: http::Request<T>) -> ... ;
    fn send_multipart<U>(&self, req: http::Request<MultipartForm>) -> ... ;
    fn send_streaming<T>(&self, req: http::Request<T>) -> ... ;
}
```

Internally, each `send*` variant will:

1. Extract `body_bytes` from the request (buffer the `Bytes` before the async move — same trick `RefreshingClaudeClient` uses at L231).
2. Call `body_classify::classify(url, body_bytes)`.
3. If `force_agent` is set, override `classification.is_agent = true`.
4. Strip `http::header::AUTHORIZATION` and the custom `"x-api-key"` header from `parts.headers`.
5. Insert the table in §3.3 below.
6. Re-attach the buffered body and call `self.inner.send(...)`.

### 3.3 Header assembly table (Rust parity)

Exact port of opencode `copilot.ts:132-145`:

| Header | Source | Condition |
|---|---|---|
| `authorization` | `format!("Bearer {}", token.read().await)` | Always. Injected LAST (wins over anything caller set). |
| `user-agent` | `self.user_agent` (e.g. `"fspec/0.9.3"`) | Always. Overrides. |
| `openai-intent` | `"conversation-edits"` | Always, hard-coded literal. |
| `x-initiator` | `"agent"` if `force_agent_flag \|\| classification.is_agent` else `"user"` | Always. |
| `copilot-vision-request` | `"true"` | Only if `classification.is_vision`. |

**Explicitly stripped from incoming headers** (must be removed before insertion to match opencode L144-145):

- `http::header::AUTHORIZATION` (case-insensitive in `HeaderMap`, so one call handles both cases — unlike opencode's plain-object hack).
- `"x-api-key"` (custom header name, must be stripped explicitly).

**Explicitly NOT set** (parity with opencode):
- `Copilot-Integration-Id`
- `Editor-Version`
- `Editor-Plugin-Version`
- `X-GitHub-Api-Version`
- `Openai-Organization`

**No retry logic.** Each `send*` makes exactly one call to `self.inner.send(...)`. Rate-limit and transient-failure handling are inherited from rig's own retry layer (if any) or the session-level retry wrapper — NOT from this client.

### 3.4 Endpoint routing — how chat vs responses maps onto rig

`rig::providers::openai` exposes two distinct model classes that both work over `HttpClientExt`:

- `openai::completion::CompletionModel` — posts to `/chat/completions`, request shape has `messages[]`.
- `openai::responses_api::ResponsesCompletionModel<HC>` — posts to `/responses`, request shape has `input[]` + `instructions`. Confirmed in use by Codex (`codex/mod.rs:24`).

Because the same `CopilotHttpClient` satisfies the `HttpClientExt` bound for both, the Copilot provider can pick per-model at **construction time**:

```rust
if should_use_copilot_responses_api(model_id) {
    let responses_client = openai::Client::<CopilotHttpClient>::builder()
        .api_key("dummy")              // ignored — CopilotHttpClient strips & replaces
        .base_url(&base_url)
        .http_client(http_client.clone())
        .build()?;
    let rm = openai::responses_api::ResponsesCompletionModel::new(
        responses_client.clone(),
        model_id,
    );
    CopilotModel::Responses(rm)
} else {
    let chat_client = openai::CompletionsClient::builder()
        .api_key("dummy")
        .base_url(&base_url)
        .http_client(http_client.clone())
        .build()?;
    let cm = openai::completion::CompletionModel::new(chat_client.clone(), model_id);
    CopilotModel::Chat(cm)
}
```

`complete_with_tools` then matches on `CopilotModel::{Chat,Responses}` and builds the appropriate `CompletionRequestBuilder`. Mirrors Codex's approach at `codex/mod.rs:140-172` and `145-164`.

**No URL rewriting.** Unlike Codex (which rewrites any URL to `chatgpt.com/backend-api/codex/responses`), Copilot uses `rig`'s native base_url for both endpoints — the base URL is `https://api.githubcopilot.com` and rig's `CompletionsClient` / `responses_api::Client` append `/chat/completions` or `/responses` respectively. The custom client only touches headers, not URLs.

### 3.5 Provider manager integration

`codelet/providers/src/manager.rs`:

1. `ProviderType` enum (L20-27): add `GitHubCopilot,`.
2. `FromStr` (L28-45): `"github-copilot" | "copilot" => Ok(ProviderType::GitHubCopilot)`.
3. `as_str` (L46-55): `ProviderType::GitHubCopilot => "github-copilot"`.
4. `has_credentials` (L60-68): `ProviderType::GitHubCopilot => credentials.has_github_copilot()` — the storage slice adds that method to `ProviderCredentials`.
5. `map_provider_id_to_type` (L330-337): `"github-copilot" => Ok(ProviderType::GitHubCopilot)`.
6. New accessor method `get_github_copilot(&self, session_id: uuid::Uuid) -> Result<GitHubCopilotProvider, ProviderError>` modeled on `get_codex` (L443-456). Reads bearer from the storage layer, calls `GitHubCopilotProvider::from_token_with_session(...)`.
7. `context_window()` / `max_output_tokens()` (L533-560): add `ProviderType::GitHubCopilot => github_copilot::CONTEXT_WINDOW` / `MAX_OUTPUT_TOKENS` arms.

`codelet/providers/src/lib.rs`: add `pub mod github_copilot;` and re-export `pub use github_copilot::GitHubCopilotProvider;`.

### 3.6 NAPI / TypeScript surface

This slice introduces **no new NAPI bindings**. The existing `ProviderManager` accessor pattern already has `get_claude`, `get_openai`, `get_codex`, `get_gemini`, `get_zai`; adding `get_github_copilot` follows the same shape and the NAPI layer auto-exposes it via whatever provider-resolution pipeline currently dispatches from JS.

`src/utils/provider-config.ts`:
- Add `'github-copilot'` to `SUPPORTED_PROVIDERS` (L77-94).
- Add a `PROVIDER_REGISTRY` entry (L101-264): `baseUrl: 'https://api.githubcopilot.com'`, `authMethod: 'bearer'`, `authType: 'oauth'`, `requiresApiKey: false`, `description: 'GitHub Copilot via GitHub OAuth device flow'`.

`src/utils/agentRegistry.ts`: **leave the existing `copilot` entry at L92-104 alone** — it refers to the editor extension as a target agent and is orthogonal to the provider.

### 3.7 What this slice does NOT do

- **No OAuth device flow code** — that's slice 1. This slice receives a bearer token from `resolve_credential("github-copilot", ...)` and treats it as opaque.
- **No `/models` catalog fetch, no merge logic, no `cost = 0` mapping** — that's slice 3.
- **No `reasoning_effort` / `reasoningSummary` / `store: false` providerOptions translation** — that's slice 3.
- **No plugin hook system** — fspec doesn't have one, and building one is a larger architectural change. Sub-agent/compaction `x-initiator` promotion is instead exposed via `CopilotHttpClient::force_agent_initiator(bool)`, to be called by the session manager when it enters a sub-agent context. See Open Question Q4.

---

## 4. Open Questions for the Product Owner

1. **Q1 — Retry policy:** opencode's data-plane fetch has ZERO retry logic. Should fspec match this (minimal parity) or add a retry layer for 429/503 (good citizen)? Research doc §8.4 item 3 flags this as a recommendation. If yes — blocking hook here or post-response retry in the session manager?

2. **Q2 — `x-initiator` promotion signal:** opencode derives "agent mode" from (a) last-message role heuristic, (b) compaction-parts-in-message probe, (c) parentID on session. fspec has sessions with parent/child via the session manager, but NO compaction-parts introspection at the provider layer. Is the parent-session signal alone sufficient, or does the session manager need a new `is_compacting` flag that the provider can read?

3. **Q3 — User-Agent string:** opencode hard-codes `opencode/${Installation.VERSION}`. Should fspec send `fspec/${VERSION}` (honest) or spoof a known editor UA (more permissive with GitHub's TOS)? This interacts with the TOS question in research doc §8.4 item 2.

4. **Q4 — Hook system ambition:** should we build a proper `chat.params` / `chat.headers` middleware system in Rust, or stay inline-mutations-only for slice 2? Building it now blocks the slice; deferring it means future providers (future Copilot variants, Azure OpenAI, Vertex) will each hand-roll header injection inside their own `HttpClientExt` impl.

5. **Q5 — Which HTTP crate for classification body-peek:** the body arrives at the middleware as `http::Request<bytes::Bytes>`. We need to `serde_json::from_slice::<Value>(&body)` to classify. Are we OK with a per-request JSON parse on every LLM call, or should we gate classification behind a content-type check + cache?

6. **Q6 — Model routing override:** is a model ID like `gpt-5-mini-codex-preview` routed to chat or responses? Opencode's regex catches `gpt-5-mini*` (starts with), so it goes to `/chat/completions`. fspec should probably match, but the PO should confirm.

---

## 5. Acceptance Criteria Candidates

Target Gherkin scenarios for slice 2. All of these are observable over-the-wire HTTP behaviors and can be tested with a mock reqwest server.

1. **Chat vs Responses routing for GPT-5 family**
   - Given a GitHub Copilot provider configured with model `gpt-5.2-codex`
   - When I issue a completion request
   - Then the HTTP request should target `/responses`
   - And the request body should contain an `input` array (not `messages`)

2. **Chat routing for `gpt-5-mini` exception**
   - Given a GitHub Copilot provider configured with model `gpt-5-mini`
   - When I issue a completion request
   - Then the HTTP request should target `/chat/completions`
   - And the request body should contain a `messages` array

3. **Chat routing for non-GPT models**
   - Given a GitHub Copilot provider configured with model `claude-sonnet-4.5`
   - When I issue a completion request
   - Then the HTTP request should target `/chat/completions`

4. **Mandatory header set on every request**
   - Given a GitHub Copilot provider with an access token `gho_xyz123`
   - When I issue any completion request
   - Then the request should include `Authorization: Bearer gho_xyz123`
   - And `Openai-Intent: conversation-edits`
   - And `User-Agent: fspec/<version>`
   - And `x-initiator` set to either `user` or `agent`
   - And NO `x-api-key` header
   - And NO duplicate `Authorization` header (case-insensitive)

5. **`x-initiator: user` for fresh user-initiated chat**
   - Given a Copilot request whose body is a `messages` array with `last.role == "user"` and only text content
   - When the request is sent
   - Then the `x-initiator` header should equal `"user"`

6. **`x-initiator: agent` for tool-loop continuation**
   - Given a Copilot request whose body is a `messages` array with `last.role == "assistant"` (tool call continuation)
   - When the request is sent
   - Then the `x-initiator` header should equal `"agent"`

7. **`Copilot-Vision-Request` only on image requests**
   - Given a Copilot chat request whose messages contain a part with `type: "image_url"`
   - When the request is sent
   - Then the header `Copilot-Vision-Request: true` should be present
   - And given the same provider on a text-only request, that header should NOT be present

8. **Enterprise base URL resolution**
   - Given a Copilot provider configured with `enterprise_url: "https://github.acme.corp"`
   - When I issue any completion request
   - Then the HTTP request host should be `copilot-api.github.acme.corp`
   - And given no enterprise URL, the host should be `api.githubcopilot.com`

9. **Sub-agent session promotion**
   - Given a Copilot provider whose `force_agent_initiator(true)` has been called by the session manager
   - When I issue a completion request whose body would normally classify as user-initiated
   - Then the `x-initiator` header should equal `"agent"`
