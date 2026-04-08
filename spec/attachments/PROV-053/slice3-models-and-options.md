# PROV-053 Slice 3: Model Catalog, Provider Options & Reasoning Effort

**Author:** Slice 3 research agent
**Date:** 2026-04-07
**Scope:** Model catalog merge, provider-options transformation, per-family reasoning effort variants, `store: false` enforcement, small-model priority for the GitHub Copilot provider in fspec.
**Out of scope:** OAuth/storage (slice 1), SDK/fetch layer (slice 2).
**Reference:** opencode at `/tmp/opencode/`, fspec at `/Users/rquast/projects/fspec/`.

---

## 1. Reference Behavior (opencode)

### 1.1 `/models` HTTP client (`packages/opencode/src/plugin/github-copilot/models.ts`)

#### Request shape — `CopilotModels.get()` lines 108–121

```ts
const data = await fetch(`${baseURL}/models`, {
  headers,
  signal: AbortSignal.timeout(5_000),
}).then(async (res) => {
  if (!res.ok) throw new Error(`Failed to fetch models: ${res.status}`)
  return schema.parse(await res.json())
})
```

- **URL**: `${baseURL}/models` (line 113). Caller supplies `baseURL` (`https://api.githubcopilot.com` or `https://copilot-api.<enterprise>`).
- **Headers**: pass-through `HeadersInit` from caller; `models.ts` injects nothing. The plugin's `provider.models` hook (`copilot.ts:50-60`) injects `Authorization: Bearer ${ctx.auth.refresh}` and `User-Agent: opencode/${Installation.VERSION}`.
- **Timeout**: 5000 ms via `AbortSignal.timeout(5_000)` (line 115). Single attempt — no retry, no backoff.
- **Error handling**: non-2xx → `throw new Error("Failed to fetch models: <status>")` (line 118). Body is JSON-parsed and run through Zod (line 120).

#### Full Zod schema — lines 5–41

```ts
export const schema = z.object({
  data: z.array(
    z.object({
      model_picker_enabled: z.boolean(),                    // REQUIRED
      id: z.string(),                                        // REQUIRED
      name: z.string(),                                      // REQUIRED
      version: z.string(),                                   // REQUIRED — pattern `{id}-YYYY-MM-DD`
      supported_endpoints: z.array(z.string()).optional(),   // optional
      capabilities: z.object({
        family: z.string(),                                  // REQUIRED
        limits: z.object({
          max_context_window_tokens: z.number(),             // REQUIRED
          max_output_tokens: z.number(),                     // REQUIRED
          max_prompt_tokens: z.number(),                     // REQUIRED
          vision: z.object({
            max_prompt_image_size: z.number(),
            max_prompt_images: z.number(),
            supported_media_types: z.array(z.string()),
          }).optional(),                                     // optional sub-object
        }),
        supports: z.object({
          adaptive_thinking: z.boolean().optional(),
          max_thinking_budget: z.number().optional(),
          min_thinking_budget: z.number().optional(),
          reasoning_effort: z.array(z.string()).optional(),
          streaming: z.boolean(),                            // REQUIRED
          structured_outputs: z.boolean().optional(),
          tool_calls: z.boolean(),                           // REQUIRED
          vision: z.boolean().optional(),
        }),
      }),
    }),
  ),
})
```

#### Merge algorithm — lines 123–142

```ts
const result = { ...existing }
const remote = new Map(
  data.data.filter((m) => m.model_picker_enabled).map((m) => [m.id, m] as const)
)

// prune existing models whose api.id isn't in the endpoint response
for (const [key, model] of Object.entries(result)) {
  const m = remote.get(model.api.id)
  if (!m) {
    delete result[key]
    continue
  }
  result[key] = build(key, m, baseURL, model)
}

// add new endpoint models not already keyed in result
for (const [id, m] of remote) {
  if (id in result) continue
  result[id] = build(id, m, baseURL)
}

return result
```

Step-by-step:

1. **Shallow-clone** `existing` (the prior catalog, e.g. from models.dev) into `result` (line 123).
2. **Build remote map** (line 124):
   - Filter `data.data` to entries with `model_picker_enabled === true` — anything `false` is **dropped before** the map is built and cannot survive into the merge.
   - Key the map by **remote `id`** (e.g. `"gpt-5.2"`).
3. **Iterate existing models** (lines 127–134), keyed by local key:
   - Look up `remote.get(model.api.id)` — match is by **`api.id`**, not by local key.
   - **DELETE path (line 130)**: if no remote match exists (model removed from catalog OR `model_picker_enabled: false`), `delete result[key]`. This is the only place a local model gets removed from the merged set.
   - **REFRESH path (line 133)**: if remote match exists, call `build(key, m, baseURL, model)` — passing `prev = model` so opinionated locally-set fields win.
4. **Add net-new remote models** (lines 137–140): for every entry in the remote map, skip if `id in result` (already refreshed in step 3), otherwise insert as `result[id] = build(id, m, baseURL)` with `prev = undefined`. **The local key for new models is the remote id verbatim.**
5. Return `result`.

`build()` is the only entry point that constructs a `Model`. It is called from exactly two places: line 133 (refresh) and line 139 (insert). Both code paths flow through it — there is no other field-mapping code.

#### `build()` field mapping — lines 45–106

Pre-computation (lines 46–53):

```ts
const reasoning =
  !!remote.capabilities.supports.adaptive_thinking ||
  !!remote.capabilities.supports.reasoning_effort?.length ||
  remote.capabilities.supports.max_thinking_budget !== undefined ||
  remote.capabilities.supports.min_thinking_budget !== undefined

const image =
  (remote.capabilities.supports.vision ?? false) ||
  (remote.capabilities.limits.vision?.supported_media_types ?? [])
    .some((item) => item.startsWith("image/"))
```

Field-by-field assignment table (with line numbers and "who wins"):

| Model field | Source | Line | Who wins |
|---|---|---|---|
| `id` | `key` (merge map key) | 56 | n/a |
| `providerID` | hard-coded `"github-copilot"` | 57 | hard-coded |
| `api.id` | `remote.id` | 59 | remote always |
| `api.url` | `url` param (= `baseURL`) | 60 | param |
| `api.npm` | hard-coded `"@ai-sdk/github-copilot"` | 61 | hard-coded |
| `status` | hard-coded `"active"` | 64 | "API response wins" comment line 63 |
| `limit.context` | `remote.capabilities.limits.max_context_window_tokens` | 66 | remote |
| `limit.input` | `remote.capabilities.limits.max_prompt_tokens` | 67 | remote |
| `limit.output` | `remote.capabilities.limits.max_output_tokens` | 68 | remote |
| `capabilities.temperature` | `prev?.capabilities.temperature ?? true` | 71 | existing |
| `capabilities.reasoning` | `prev?.capabilities.reasoning ?? reasoning` | 72 | existing |
| `capabilities.attachment` | `prev?.capabilities.attachment ?? true` | 73 | existing |
| `capabilities.toolcall` | `remote.capabilities.supports.tool_calls` | 74 | remote always |
| `capabilities.input.text` | `true` | 76 | hard-coded |
| `capabilities.input.audio` | `false` | 77 | hard-coded |
| `capabilities.input.image` | derived `image` | 78 | derived |
| `capabilities.input.{video,pdf}` | `false` | 79–80 | hard-coded |
| `capabilities.output.text` | `true` | 83 | hard-coded |
| `capabilities.output.{audio,image,video,pdf}` | `false` | 84–87 | hard-coded |
| `capabilities.interleaved` | `false` | 89 | hard-coded |
| `family` | `prev?.family ?? remote.capabilities.family` | 92 | existing |
| `name` | `prev?.name ?? remote.name` | 93 | existing |
| `cost` | `{ input: 0, output: 0, cache: { read: 0, write: 0 } }` | 94–98 | hard-coded **always** (no `prev?.cost ??`) |
| `options` | `prev?.options ?? {}` | 99 | existing |
| `headers` | `prev?.headers ?? {}` | 100 | existing |
| `release_date` | `prev?.release_date ?? (version.startsWith(\`${id}-\`) ? version.slice(id.length+1) : version)` | 101–103 | existing → strip `{id}-` prefix from version |
| `variants` | `prev?.variants ?? {}` | 104 | existing |

#### Why `cost` is hard-coded to zero (lines 94–98)

The Copilot `/models` endpoint does **not** return per-token pricing. Copilot is a flat-fee subscription product (Pro / Business / Enterprise) — billing happens at the GitHub seat level, not per request. Setting all four cost fields to `0` makes the opencode usage tracker register zero incremental cost on every Copilot call. Note this is the **only** field where `prev?.cost ??` is omitted: any user-customized cost is clobbered back to zero on every refresh.

#### `release_date` parsing (lines 101–103)

Comment at line 11: `// every version looks like: {model.id}-YYYY-MM-DD`. Algorithm:

1. If `prev?.release_date` exists, use it (existing wins).
2. Else if `remote.version.startsWith(remote.id + "-")`, return `remote.version.slice(remote.id.length + 1)` → e.g. `"gpt-4o-2024-05-13"` → `"2024-05-13"`.
3. Else (defensive fallback) return the raw `remote.version` verbatim.

No date validation is performed.

### 1.2 `sdkKey()` function (`transform.ts:23–47`)

```ts
function sdkKey(npm: string): string | undefined {
  switch (npm) {
    case "@ai-sdk/github-copilot":          return "copilot"
    case "@ai-sdk/azure":                   return "azure"
    case "@ai-sdk/openai":                  return "openai"
    case "@ai-sdk/amazon-bedrock":          return "bedrock"
    case "@ai-sdk/anthropic":
    case "@ai-sdk/google-vertex/anthropic": return "anthropic"
    case "@ai-sdk/google-vertex":           return "vertex"
    case "@ai-sdk/google":                  return "google"
    case "@ai-sdk/gateway":                 return "gateway"
    case "@openrouter/ai-sdk-provider":     return "openrouter"
  }
  return undefined
}
```

Every case verbatim (line → npm → sdk key):
- L26/27: `@ai-sdk/github-copilot` → `"copilot"`
- L28/29: `@ai-sdk/azure` → `"azure"`
- L30/31: `@ai-sdk/openai` → `"openai"`
- L32/33: `@ai-sdk/amazon-bedrock` → `"bedrock"`
- L34/35/36: `@ai-sdk/anthropic` AND `@ai-sdk/google-vertex/anthropic` → `"anthropic"` (shared case)
- L37/38: `@ai-sdk/google-vertex` → `"vertex"`
- L39/40: `@ai-sdk/google` → `"google"`
- L41/42: `@ai-sdk/gateway` → `"gateway"`
- L43/44: `@openrouter/ai-sdk-provider` → `"openrouter"`
- Default → `undefined`.

#### Why `@ai-sdk/github-copilot` → `"copilot"`

The Vercel AI SDK convention is that each provider package internally reads `providerOptions.<short-slug>` when building requests. The vendored Copilot SDK at `packages/opencode/src/provider/sdk/copilot/copilot-provider.ts` extends `@ai-sdk/openai-compatible` and **expects its options namespace to be keyed `"copilot"`**, not the long bundled package name `"github-copilot"`.

Concrete proof inside `transform.ts` itself: the cache-control map at lines 209–211 uses the bare `"copilot"` key:

```ts
copilot: { copilot_cache_control: { type: "ephemeral" } },
```

`sdkKey()` is consumed by `ProviderTransform.message()` (the providerOptions key remapper) and by `ProviderTransform.providerOptions()` (the outgoing request builder). When fspec receives a message whose `providerOptions` is keyed under the locally-named providerID `"github-copilot"`, the transform rewrites it to `"copilot"` so the SDK picks it up. The lock-down test for this behavior is `transform.test.ts:1589–1605` ("copilot remaps providerID to 'copilot' key").

### 1.3 Reasoning-effort variants — `transform.ts:462–486`

Constants at lines 361–362:

```ts
const WIDELY_SUPPORTED_EFFORTS = ["low", "medium", "high"]
const OPENAI_EFFORTS = ["none", "minimal", ...WIDELY_SUPPORTED_EFFORTS, "xhigh"]
```

Pre-condition: at line 365, `if (!model.capabilities.reasoning) return {}` — non-reasoning models get empty variants regardless. `id` (lowercase model id) is computed at line 367.

The `@ai-sdk/github-copilot` case (lines 462–486):

```ts
case "@ai-sdk/github-copilot":
  if (model.id.includes("gemini")) {
    // currently github copilot only returns thinking
    return {}
  }
  if (model.id.includes("claude")) {
    return Object.fromEntries(
      WIDELY_SUPPORTED_EFFORTS.map((effort) => [effort, { reasoningEffort: effort }])
    )
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

Three branches:

**(a) Gemini-via-Copilot (lines 463–466)** → returns `{}`. Inline comment explains: Copilot's Gemini proxy currently emits raw thinking text and doesn't honor a reasoning-effort enum, so exposing variants would be a lie.

**(b) Claude-via-Copilot (lines 467–469)** → exactly `WIDELY_SUPPORTED_EFFORTS`, simple shape: `{ low: { reasoningEffort: "low" }, medium: { reasoningEffort: "medium" }, high: { reasoningEffort: "high" } }`. Note: NO `reasoningSummary`, NO `include` — Claude-over-Copilot uses the simpler schema (Anthropic's thinking blocks are inlined in the assistant message, no opaque encrypted blob needed).

**(c) GPT-5 / other Copilot (lines 470–486)** — wrapped in `iife()` (utility from `@/util/iife`, runs lambda immediately):

1. **Fast path (line 471)**: If `id` contains any of `"5.1-codex-max"`, `"5.2"`, or `"5.3"`, return `["low","medium","high","xhigh"]`. These three families bypass the date gate because xhigh is **known** to be supported regardless of release date.
2. **Generic gpt-5 (line 474)**: Start with `[...WIDELY_SUPPORTED_EFFORTS]`. Append `"xhigh"` only if BOTH:
   - `id.includes("gpt-5")` AND
   - `model.release_date >= "2025-12-04"` (lexical string compare on `YYYY-MM-DD`, which collates correctly).
   The `2025-12-04` cutoff matches the date xhigh became generally available across the GPT-5 line on Copilot. Any `gpt-5*` released earlier gets only the three standard efforts.
3. **Other**: a non-`gpt-5`, non-`5.1-codex-max`, non-`5.2`, non-`5.3` Copilot model that is neither Gemini nor Claude → returns just `WIDELY_SUPPORTED_EFFORTS`.

For every effort in `copilotEfforts`, the variant body is:

```ts
{
  reasoningEffort: effort,
  reasoningSummary: "auto",
  include: ["reasoning.encrypted_content"],
}
```

#### Why `reasoningSummary: "auto"` and `include: ["reasoning.encrypted_content"]` are defaults

These two options come from the **OpenAI Responses API** protocol (which GPT-5 uses end-to-end, even when proxied by Copilot — see slice 2's `shouldUseCopilotResponsesApi()` routing):

1. **`reasoningSummary: "auto"`** — instructs the Responses API to return a *summary* of the hidden reasoning chain alongside the main output. Without this, Copilot-proxied GPT-5 returns no user-visible reasoning content at all (the raw chain-of-thought stays hidden). `"auto"` lets the API choose the summary length.
2. **`include: ["reasoning.encrypted_content"]`** — asks the Responses API to return an opaque encrypted blob of the full reasoning trace. This blob is what opencode replays on subsequent turns to reconstruct the multi-turn reasoning state. Because `store: false` is also forced (§1.4), the Responses API will not persist the reasoning server-side between turns — the client **must** request the encrypted blob to round-trip it. Together, `store: false` + `include: ["reasoning.encrypted_content"]` form the "zero-retention with client-side reasoning replay" pattern.

Gemini doesn't use this protocol (hence `{}`), and Claude inlines thinking blocks directly in the assistant message (no separate include needed).

### 1.4 Forced `store: false` enforcement — `transform.ts:751–758`

```ts
// openai and providers using openai package should set store to false by default.
if (
  input.model.providerID === "openai" ||
  input.model.api.npm === "@ai-sdk/openai" ||
  input.model.api.npm === "@ai-sdk/github-copilot"
) {
  result["store"] = false
}
```

This sits at the top of `ProviderTransform.options()` (the function that builds the outgoing `providerOptions` object). It triggers under three conditions joined by OR:

1. `providerID === "openai"` — the canonical OpenAI provider id
2. `api.npm === "@ai-sdk/openai"` — anything using the official OpenAI SDK package (e.g. an Azure or compat provider routed through it)
3. `api.npm === "@ai-sdk/github-copilot"` — the vendored Copilot SDK

**Why `store: false` is forced:** the OpenAI Responses API supports server-side response storage (the `/responses` endpoint can keep a conversation thread on OpenAI's infrastructure). For privacy / zero-retention reasons, opencode opts out of storage on every request — both for direct OpenAI calls and for Copilot-proxied OpenAI calls. The Copilot endpoints inherit the underlying OpenAI Responses API semantics, so the same flag applies. This is the upstream half of the "client-side reasoning replay" pattern: server doesn't keep state, client carries the encrypted reasoning blob.

### 1.5 `smallOptions(model)` — `transform.ts:864–897`

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
  if (model.providerID === "google") { ... }
  if (model.providerID === "openrouter") { ... }
  if (model.providerID === "venice") { ... }
  return {}
}
```

For Copilot-routed models (the OpenAI/copilot branch at lines 865–877):

| Model id pattern | Returns | Lines |
|---|---|---|
| Contains `"gpt-5"` AND contains `"5."` (i.e. `gpt-5.0`, `gpt-5.1`, `gpt-5.2`, `gpt-5.3`, `gpt-5.4`...) | `{ store: false, reasoningEffort: "low" }` | 871–873 |
| Contains `"gpt-5"` but NOT `"5."` (i.e. plain `gpt-5`, `gpt-5-mini`, `gpt-5-nano`) | `{ store: false, reasoningEffort: "minimal" }` | 874 |
| Anything else (claude, gemini, gpt-4*, o1, etc.) | `{ store: false }` | 876 |

Rationale: small-model usage (title generation, summaries, autoname) wants the **fastest** response. GPT-5 base/mini/nano supports `"minimal"` reasoning (no chain-of-thought at all). GPT-5.x models (5.1+) require at least `"low"` because they removed the `"minimal"` tier. Both still need `store: false` to inherit the zero-retention rule.

### 1.6 Copilot small-model priority injection — `provider.ts:1547–1573`

Context from `getSmallModel()` (lines 1547–1573):

```ts
const getSmallModel = Effect.fn("Provider.getSmallModel")(function* (providerID: ProviderID) {
  const cfg = yield* config.get()
  if (cfg.small_model) {
    const parsed = parseModel(cfg.small_model)
    return yield* getModel(parsed.providerID, parsed.modelID)
  }

  const s = yield* InstanceState.get(state)
  const provider = s.providers[providerID]
  if (!provider) return undefined

  let priority = [
    "claude-haiku-4-5",
    "claude-haiku-4.5",
    "3-5-haiku",
    "3.5-haiku",
    "gemini-3-flash",
    "gemini-2.5-flash",
    "gpt-5-nano",
  ]
  if (providerID.startsWith("opencode")) {
    priority = ["gpt-5-nano"]
  }
  if (providerID.startsWith("github-copilot")) {
    priority = ["gpt-5-mini", "claude-haiku-4.5", ...priority]
  }
  for (const item of priority) { /* find first matching model id */ }
})
```

Key observations:

- The function returns the "fast" model used for title generation, summaries, and other ancillary tasks.
- User-configured `cfg.small_model` always wins (lines 1550–1553).
- The default priority list (lines 1559–1567) walks haiku → flash → nano fallbacks across providers.
- **Copilot override (lines 1571–1573)**: prepend `["gpt-5-mini", "claude-haiku-4.5", ...]` to the default list. The reasoning is twofold:
  1. **`gpt-5-mini`** is the cheapest tier on Copilot's seat-based pricing, fastest TTFT, and **explicitly excluded from the `/responses` API routing** by `shouldUseCopilotResponsesApi()` (slice 2) — so it goes through `/chat/completions`, which has lower per-turn latency than `/responses`.
  2. **`claude-haiku-4.5`** is the next-best Copilot fast model; preferring Anthropic over Google for this tier matches what the official `gh copilot` CLI does for autocomplete-style tasks.

The rest of the priority list is preserved as a fallback if neither preferred model is available in the Copilot catalog at lookup time.

### 1.7 Test coverage inventory

#### `test/plugin/github-copilot-models.test.ts`

| Test name | Behavior locked down |
|---|---|
| `preserves temperature support from existing provider models` (single test, lines 10–117) | The merge `prev?.capabilities.temperature ?? true` rule. Mocks `fetch` with one existing-but-refreshed model (`gpt-4o`) and one brand-new model (`brand-new`). Asserts both end up with `temperature === true` — the existing one because `prev` had `temperature: true`, the new one because the default is `true`. |

This single test exercises the full happy-path merge: `model_picker_enabled === true`, refresh existing model, add net-new remote model, default-fill capability fields. Notably it does **not** test the DELETE path (existing model whose `api.id` is missing from remote response) or the `model_picker_enabled === false` filter — both are gaps in opencode's coverage we should fill.

#### `test/provider/transform.test.ts` — relevant test cases

**`describe("ProviderTransform.options - setCacheKey")`** (lines 7–105)
- L88 `should set store=false for openai provider` — locks down lines 752–758. (No dedicated copilot test, but the same code path triggers.)

**`describe("ProviderTransform.providerOptions")`** (lines 293–442)
- L329 `uses sdk key for non-gateway models`
- L344 `uses gateway model provider slug for gateway models`
- L412 `maps amazon slug to bedrock for provider options`

**`describe("ProviderTransform.message - providerOptions key remapping")`** (lines 1515–1624)
- L1542 `azure keeps 'azure' key and does not remap to 'openai'`
- L1560 `azure cognitive services remaps providerID to 'azure' key`
- **L1589 `copilot remaps providerID to 'copilot' key`** — locks down `sdkKey("@ai-sdk/github-copilot") === "copilot"`. The test creates a `github-copilot` providerID model with `npm: "@ai-sdk/github-copilot"`, asserts that `providerOptions.copilot` is preserved and `providerOptions["github-copilot"]` becomes `undefined`.
- L1607 `bedrock remaps providerID to 'bedrock' key`

**`describe("ProviderTransform.variants") > describe("@ai-sdk/github-copilot")`** (lines 2128–2251) — **the key reasoning-effort lock-down block**:

| Line | Test name | Asserts |
|---|---|---|
| 2129 | `standard models return low, medium, high` | `gpt-4.5` (no `gpt-5` substring) returns exactly `["low","medium","high"]`; spot-checks `result.low === { reasoningEffort: "low", reasoningSummary: "auto", include: ["reasoning.encrypted_content"] }` |
| 2148 | `gpt-5.1-codex-max includes xhigh` | Substring `"5.1-codex-max"` triggers fast-path → `["low","medium","high","xhigh"]` |
| 2162 | `gpt-5.1-codex-mini does not include xhigh` | Substring `"5.1-codex-mini"` does NOT match `"5.1-codex-max"` or `"5.2"`/`"5.3"`, so it returns only `["low","medium","high"]` |
| 2176 | `gpt-5.1-codex does not include xhigh` | Same — `"5.1-codex"` doesn't match `"5.2"`/`"5.3"`/`"5.1-codex-max"` |
| 2190 | `gpt-5.2 includes xhigh` | Substring `"5.2"` triggers fast-path; explicitly asserts `result.xhigh === { reasoningEffort: "xhigh", reasoningSummary: "auto", include: ["reasoning.encrypted_content"] }` |
| 2209 | `gpt-5.2-codex includes xhigh` | Substring `"5.2"` triggers fast-path |
| 2223 | `gpt-5.3-codex includes xhigh` | Substring `"5.3"` triggers fast-path |
| 2237 | `gpt-5.4 includes xhigh` | Date-gated branch: model has `release_date: "2026-03-05"` which is `>= "2025-12-04"`, so xhigh is appended |

**Coverage gaps in opencode's tests** (worth filling in fspec):
- No test for the date-gated branch with a release_date BEFORE `2025-12-04` (negative case for the cutoff).
- No test for `gemini`-via-Copilot returning `{}`.
- No test for `claude`-via-Copilot returning the simpler 3-effort shape (without `reasoningSummary`/`include`).
- No test for `smallOptions()` for any Copilot model variant.
- No test for the `provider.ts:1571-1573` priority injection.
- No test for the `models.ts` DELETE path or `model_picker_enabled: false` filter.
- No dedicated test for `store: false` triggered by `@ai-sdk/github-copilot`.

---

## 2. fspec Current State

**Tooling note:** DeepSearch failed mid-investigation; findings below come from direct file reads, glob/grep across `/Users/rquast/projects/fspec/src` and `/Users/rquast/projects/fspec/codelet`.

### 2.1 Model catalog — Rust-side, sourced from models.dev

fspec already has a complete model catalog implementation, but it lives in **Rust**, not TypeScript. The TypeScript layer consumes it via NAPI bindings.

**Rust types** — `codelet/providers/src/models/types.rs`:

```rust
// lines 9–34
pub struct ModelsDevResponse {
    #[serde(flatten)]
    pub providers: HashMap<String, ProviderInfo>,
}

pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    #[serde(default)] pub env: Vec<String>,
    pub npm: Option<String>,
    pub api: Option<String>,
    pub doc: Option<String>,
    #[serde(default)] pub models: HashMap<String, ModelInfo>,
}

// lines 37–88
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub family: Option<String>,
    #[serde(default)] pub release_date: Option<String>,
    #[serde(default)] pub attachment: bool,
    #[serde(default)] pub reasoning: bool,
    #[serde(default)] pub tool_call: bool,
    #[serde(default)] pub temperature: bool,
    #[serde(default)] pub interleaved: Option<InterleavedConfig>,
    pub modalities: Option<Modalities>,
    pub cost: Option<CostInfo>,
    pub limit: LimitInfo,
    pub status: Option<ModelStatus>,
    #[serde(default)] pub experimental: Option<bool>,
    #[serde(default)] pub options: HashMap<String, serde_json::Value>,
    #[serde(default)] pub headers: HashMap<String, String>,
}
```

**Comparison to opencode's `Model` type:**

| opencode field | fspec equivalent | Match? |
|---|---|---|
| `id` | `id` | ✓ |
| `providerID` | implicit in HashMap key (`providers[providerID]`) | partial — fspec stores providers and models separately |
| `api.id`, `api.url`, `api.npm` | `ProviderInfo.api`, `ProviderInfo.npm` (no per-model `api.id` — model id IS the api id) | partial — fspec keeps api info on the provider, opencode duplicates per model |
| `status` | `ModelStatus { Alpha, Beta, Deprecated }` (Option) | similar |
| `limit.context`, `limit.input`, `limit.output` | `LimitInfo { context, output }` — **`input` is missing** | gap — no `max_prompt_tokens` field |
| `capabilities.reasoning/toolcall/attachment/temperature` | flat `reasoning`, `tool_call`, `attachment`, `temperature` bools | ✓ |
| `capabilities.input.image` | derived from `modalities.input.contains(Modality::Image)` | ✓ |
| `cost.input/output/cache.{read,write}` | `CostInfo { input, output, cache_read, cache_write, context_over_200k }` | ✓ (richer in fspec) |
| `release_date` | `release_date: Option<String>` | ✓ |
| `family` | `family: Option<String>` | ✓ |
| `options` (provider-specific) | `options: HashMap<String, serde_json::Value>` | ✓ |
| `headers` | `headers: HashMap<String, String>` | ✓ |
| `variants` | **MISSING** | gap — no concept of reasoning-effort variants in fspec model type |

**Key gaps:**
1. fspec's `LimitInfo` has only `context` and `output` — there is no `input` field for `max_prompt_tokens`. Copilot's `/models` endpoint returns this distinct from `max_context_window_tokens` (input is the max prompt size, context is the max sliding window).
2. fspec has no `variants` field, so per-effort options cannot be attached to a model row.
3. fspec has no `model_picker_enabled` analog; the Rust catalog comes from models.dev and doesn't carry per-model availability flags from a live endpoint.

**Catalog source** — `codelet/providers/src/models/cache.rs:20`:
```rust
const MODELS_DEV_URL: &str = "https://models.dev/api.json";
```

The cache fetches the entire models.dev JSON dump on first use and persists it to `{data_dir}/cache/models.json`. Refresh strategy (lines 1–8 doc comment): "Indefinite cache, only refetch when: cache file is missing, cache file is corrupted, or user explicitly requests refresh." The `models_refresh_cache()` NAPI binding (`codelet/napi/src/models/napi_bindings.rs:184–200`) also calls `invalidate_registry_cache()` so subsequent `models_list_all()` calls rebuild from the new disk cache (locked down by `provider-specific-model-listing-and-refresh-coverage-gaps.feature` MODEL-003).

**Registry layer** — `codelet/providers/src/models/registry.rs`:
- `ModelRegistry::from_response()` builds a HashMap of providers + a capability index for fast filtering by `Reasoning | Vision | ToolCall | Attachment`.
- `parse_model_string()` parses `"provider/model-id"` format.
- `get_model()` does prefix matching + Levenshtein-distance suggestions.
- `validate_model_for_use()` enforces `tool_call === true` ("fspec requires tool_call capability").
- `list_providers()`, `list_models(provider_id)`, `filter_by_capability()`, `search()`.

**Filtering at NAPI boundary** — `codelet/napi/src/models/napi_bindings.rs:103–131`:
```rust
pub async fn models_list_all() -> Result<Vec<NapiProviderModels>> {
    let registry = get_registry().await.map_err(Error::from_reason)?;
    Ok(registry.list_providers().iter().map(|provider_info| {
        let mut models: Vec<_> = provider_info.models.values()
            .filter(|m| is_current_model(m))                  // filters deprecated + >18mo old
            .collect();
        models.sort_by(|a, b| {                                // newest-first
            let date_a = a.release_date.as_deref().unwrap_or("1970-01-01");
            let date_b = b.release_date.as_deref().unwrap_or("1970-01-01");
            date_b.cmp(date_a)
        });
        NapiProviderModels {
            provider_id: provider_info.id.clone(),
            provider_name: provider_info.name.clone(),
            models: models.into_iter().map(to_napi_model_info).collect(),
        }
    }).collect())
}
```

This is the existing analog of opencode's merge logic — but instead of merging a remote `/models` response with a local catalog, fspec applies static filters (deprecated + age) to the cached models.dev snapshot.

**TypeScript NAPI types** — `codelet/napi/index.d.ts:1402–1464`:
```ts
export interface NapiModelInfo {
  id: string;
  name: string;
  family?: string;
  reasoning: boolean;
  toolCall: boolean;
  attachment: boolean;
  temperature: boolean;
  contextWindow: number;     // u32
  maxOutput: number;         // u32
  hasVision: boolean;
}
export interface NapiProviderModels {
  providerId: string;
  providerName: string;
  models: Array<NapiModelInfo>;
}
```

This is the **flattened** view exposed to TypeScript. It has dropped the rich Rust `ModelInfo` (cost, limit.input, options, headers, variants, status, modalities) — the JS layer sees a minimum-viable shape.

**TypeScript-side types** — `src/tui/types/provider.ts`:

```ts
import type { NapiModelInfo } from '@sengac/codelet-napi';
export type ProviderModel = NapiModelInfo;             // line 12, just a re-export

export interface ProviderSection {                       // lines 18–30
  providerId: string;
  providerName: string;
  internalName: string;
  models: ProviderModel[];
  hasCredentials: boolean;
  profileName?: string;
  profileConfig?: ProfileConfig;
  isUnreachable?: boolean;
}
```

There is no TypeScript-side `Model` type that matches opencode's shape. `NapiModelInfo` is the canonical type and it has none of the fields needed for slice 3 (no `api.npm`, no `release_date`, no `cost`, no `variants`, no `options`).

### 2.2 Provider-options transform — does NOT exist in fspec

**There is no fspec equivalent of opencode's `transform.ts`.**

Direct evidence:
- `Glob src/**/transform*.ts` → no matches.
- `Grep -l "providerOptions|sdkKey|reasoningEffort|reasoning_effort" src/` → **no matches**.
- `Grep -l "@ai-sdk/" src/` → **no matches**.
- `Grep -l "model_picker_enabled" src/` → **no matches**.
- No file in `src/` maps npm package names to short SDK aliases.
- No file in `src/` constructs `providerOptions` objects, splits routing options from provider-specific options, or rewrites providerOptions key namespaces.

The reason: fspec **does not use the Vercel AI SDK at all**. Provider integration is entirely Rust-side via the `rig` crate (see `codelet/providers/src/openai.rs:18–19`: `use rig::providers::openai;`). The TypeScript layer never builds an outgoing API request directly — it calls into the Rust session manager via NAPI, which constructs the request inside `rig`.

This means slice 3's "transform layer" is structurally **unnecessary** for fspec in the way it exists in opencode. There is no `providerOptions` object travelling through TypeScript that needs key remapping. The mental model has to be inverted: **the equivalent functionality must live where the provider client actually constructs requests**, which is `codelet/providers/src/<provider>.rs`.

### 2.3 Reasoning-effort handling — Rust-side facade pattern

fspec already has a model-aware reasoning configuration system, but it's organized as a **per-provider facade** in Rust, not a per-npm-package switch in a TypeScript file.

**`codelet/napi/src/thinking_config.rs`** — the canonical entry point:

```rust
// lines 19–25
pub enum JsThinkingLevel {
    Off, Low, Medium, High,
}

// lines 124–170
#[napi]
pub fn get_thinking_config(provider: String, level: JsThinkingLevel) -> napi::Result<String> {
    let level: ThinkingLevel = level.into();
    let config = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.request_config(level)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.request_config(level)
    } else if is_claude_provider(&provider) {
        // PROV-005: per-model exact-match dispatch (4.6 → adaptive, others → budgeted)
        ClaudeThinkingFacade.request_config_for_model(&provider, level)
            .unwrap_or(serde_json::json!({}))
    } else if is_codex_provider(&provider) {
        // PROV-037: OpenAI Responses API format
        match level {
            ThinkingLevel::Off => serde_json::json!({}),
            ThinkingLevel::Low => serde_json::json!({ "reasoning": { "effort": "low",    "summary": "auto" } }),
            ThinkingLevel::Medium => serde_json::json!({ "reasoning": { "effort": "medium", "summary": "auto" } }),
            ThinkingLevel::High => serde_json::json!({ "reasoning": { "effort": "high",   "summary": "auto" } }),
        }
    } else {
        serde_json::json!({})
    };
    serde_json::to_string(&config).map_err(...)
}
```

**Existing facades** (referenced in `codelet/tools/src/facade/thinking_config.rs` per `thinking-config-facade-for-provider-specific-reasoning.feature` TOOL-009):
- `Gemini3ThinkingFacade` — uses `thinkingLevel` enum
- `Gemini25ThinkingFacade` — uses `thinkingBudget` (token count)
- `ClaudeThinkingFacade` — uses `thinking.budget_tokens` for 4.5 and earlier; adaptive for 4.6 (PROV-005 `claude-opus-4-6-adaptive-thinking.feature`)
- Codex (inline in `thinking_config.rs`) — uses Responses API `reasoning: { effort, summary }` (PROV-037 `codex-provider-reasoning-configuration.feature`)

**Key observations vs opencode's variants() function:**

1. fspec uses a **3-level enum** (`Off | Low | Medium | High`) where opencode uses a **per-model variable list** of named effort tiers (`["low","medium","high"]`, `["low","medium","high","xhigh"]`, `["minimal","low","medium","high"]`, etc.). fspec cannot currently express opencode's `xhigh` tier or `minimal` tier without extending the enum.
2. fspec dispatches by **provider name** (`is_codex_provider`, `is_claude_provider`), not by **AI SDK npm package**. This is a different axis but morally equivalent.
3. fspec returns a **single config JSON** for the chosen level. opencode returns a **map of all available variants** keyed by effort name. fspec's design assumes the UI chooses a single level and the backend returns one config; opencode's design exposes the menu of available efforts to the UI.
4. fspec has **no `store: false` enforcement** anywhere. The Codex branch sends `reasoning: { ... }` only — there is no `store` field. (This might already be a latent gap for OpenAI Responses API zero-retention compliance, regardless of Copilot.)
5. fspec has **no `include: ["reasoning.encrypted_content"]`** in any provider. The Codex branch sends `reasoning.summary` but does not request the encrypted content blob. **This means fspec's existing Codex provider does not currently support multi-turn reasoning continuity** — it just summarizes per-turn.

**TS consumer:** `src/tui/components/AgentView.tsx:1858–1868`:
```ts
let thinkingConfig: string | null = null;
if (effectiveLevel !== JsThinkingLevel.Off) {
  thinkingConfig = getThinkingConfig(currentProvider, effectiveLevel);
  // …
}
```
The TUI just calls `getThinkingConfig(provider, level)` → opaque JSON string → forwards into the Rust session manager. There is no JS-side variants menu.

### 2.4 Small-model concept — does NOT exist in fspec

**`grep -r "small_model|smallModel|titleModel|title_model|fastModel|summaryModel" src/ codelet/`** returned **zero matches**.

There is no concept of a fast model for title generation, summaries, or any other ancillary task. The single user-selected model is used for everything. opencode's `getSmallModel()` priority list and the `smallOptions()` function have no analog.

### 2.5 Model fetch & refresh at runtime

**TypeScript orchestration** — `src/tui/services/modelInitializationService.ts:386–502` (`initializeModels()`):
1. Skip if `useModelStore.modelsInitialized === true` (lines 390–402) — single-shot init.
2. Set `isLoading: true` (line 404).
3. **Cloud models** (lines 408–409): `await modelsListAll()` → `buildCloudSections()`. The NAPI call returns the full models.dev catalog filtered to current models.
4. **Profile sections** (line 412): `loadProfileSections()` walks `~/.fspec/profiles/openai/` for vLLM/Ollama/etc. profiles, calls `modelsListLocalOpenai(baseUrl, apiKey)` per profile.
5. Combines sections, filters out unreachable+empty, restores persisted last-used model, falls back to first available.
6. Updates Zustand store (`useModelStore`).

**Refresh path** — `src/tui/hooks/useModelSelectorState.ts` and `src/tui/store/modelStore.ts:96–155`:
- The store has `isRefreshing` state and a `setIsRefreshing` action.
- Refreshing calls `modelsRefreshCache()` (NAPI) followed by `modelsListAll()` (NAPI).
- The Rust side does the actual /api.json fetch + cache invalidation (`codelet/providers/src/models/cache.rs::refresh()`).

**There is no per-provider `/models` endpoint client in TypeScript.** All model discovery goes through models.dev. The only TypeScript-side fetch of a non-models.dev endpoint is `modelsListLocalOpenai()` for vLLM profiles, which calls the local server's `/v1/models` endpoint via NAPI (still Rust-side under the hood).

**Bundled overrides** — `src/tui/data/codex-models.json` is a bundled allowlist of Codex-supported models (slug + visibility + priority), used by `src/tui/services/codexAllowlistService.ts` to filter the OpenAI cloud models down to only those Codex actually accepts. This file is the closest existing analog to a "static catalog" but it's a **filter/priority overlay** on top of models.dev, not a separate source of truth.

User override at `~/.fspec/codex-models.json` is supported (`codexAllowlistService.ts:52–75`).

### 2.6 Existing provider implementations

| Provider | File | Notes |
|---|---|---|
| Claude | `codelet/providers/src/claude.rs` (~34k LOC) | Direct Anthropic API |
| OpenAI | `codelet/providers/src/openai.rs` (~24k LOC) | Used for vLLM/Ollama/cloud OpenAI via base URL override |
| Codex | `codelet/providers/src/codex/mod.rs` (~33k LOC) | Codex/ChatGPT OAuth |
| Gemini | `codelet/providers/src/gemini.rs` (~16k LOC) | Direct Google API |
| ZAI | `codelet/providers/src/zai.rs` (~16k LOC) | GLM/zhipu |

Provider registration is via the `ProviderType` enum in `codelet/providers/src/manager.rs:20–67`:
```rust
pub enum ProviderType { Claude, OpenAI, Codex, Gemini, ZAI }
```

**There is no `GitHubCopilot` variant.** Adding one will require extending this enum, the `FromStr` impl, the `as_str()` impl, the `has_credentials()` check, and wiring through the manager.

### 2.7 Gaps that slice 3 must fill

| Concept | opencode | fspec today | Gap |
|---|---|---|---|
| Per-model `/models` endpoint client | `CopilotModels.get()` w/ Zod + 5s timeout | None — only models.dev fetch | **Must build new fetcher** |
| Merge remote catalog with prior models | `models.ts:108–143` | None — fspec replaces wholesale on refresh | **Must build merge function** |
| `model_picker_enabled` filter | line 124 | n/a | **Must add filter** |
| Hard-coded zero `cost` for subscription billing | lines 94–98 | n/a | **Must hard-code** |
| `release_date` parsed from `{id}-YYYY-MM-DD` | lines 101–103 | n/a (fspec stores release_date directly from models.dev JSON) | **Must add parser** |
| `Model.variants` field for per-effort options | yes | **NO** — `NapiModelInfo` has no variants field | **Must extend NAPI type OR keep variants in a separate Rust map keyed by model id** |
| Per-family reasoning effort generation | `transform.ts:462–486` switch on `npm` and model id | facade pattern dispatching on provider name; no `xhigh`/`minimal` tiers | **Must extend ThinkingLevel enum and add Copilot facade OR a model-aware variant table** |
| Forced `store: false` for OpenAI/Copilot | `transform.ts:751–758` | **NO** — Codex branch never sets `store` | **Must add (also fixes a latent Codex gap)** |
| Reasoning encrypted content round-trip (`include: ["reasoning.encrypted_content"]`) | yes | **NO** — Codex branch doesn't request it | **Must add (also fixes a latent Codex gap for multi-turn reasoning)** |
| `smallOptions(model)` per-family overrides | yes | n/a — no small-model concept | **Defer or build new — depends on §3.6 decision** |
| Small-model priority list | `provider.ts:1559–1573` | n/a | **Defer or build new — depends on §3.6 decision** |
| `sdkKey()` alias remapping | yes | n/a — no AI SDK in fspec | **Not needed — fspec uses rig, not Vercel AI SDK** |

---
