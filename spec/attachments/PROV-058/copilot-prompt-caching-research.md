# Copilot Prompt Caching Research

> Research conducted 2026-04-10 by reverse-engineering the GitHub Copilot CLI binary
> and analyzing the opencode codebase at /tmp/opencode.

## 1. GitHub Copilot CLI Architecture

### Binary Structure
- **Location**: `/opt/homebrew/bin/copilot` (Homebrew cask `copilot-cli` v1.0.11)
- **Format**: Node.js v24.11.1 **Single Executable Application (SEA)** — a Mach-O arm64 binary embedding a full Node.js runtime
- **Source**: Microsoft/GitHub (Copyright notice: `© Microsoft Corporation`)
- **Repository**: `github.com/github/copilot-cli` (private)

### How the Binary Works
1. The SEA binary contains a **loader** (`sea-loader.js`, ~89KB) that extracts a bundled `copilot.tgz`
2. Extracted to: `~/Library/Caches/copilot/pkg/universal/<version>/`
3. The main application code is in **`app.js`** (~14MB minified JavaScript)
4. Also ships tree-sitter WASM binaries, ripgrep, sharp image processing, and a copilot-sdk

### Extracted Package Layout
```
~/Library/Caches/copilot/pkg/universal/1.0.22/
├── index.js          # Entry point (5KB)
├── app.js            # Main application (~14MB minified)
├── copilot-sdk/      # SDK for Copilot API communication
│   ├── client.d.ts
│   ├── extension.js
│   ├── generated/
│   └── index.js
├── tree-sitter-*.wasm  # Language parsers
├── ripgrep/            # Search binary
├── builtin-skills/     # Built-in agent skills
├── schemas/            # JSON schemas
└── definitions/        # Tool definitions
```

---

## 2. How Copilot CLI Handles Prompt Caching

### The `copilot_cache_control` Field

Copilot uses a **proprietary `copilot_cache_control` field** on messages and tools. This is NOT the same as Anthropic's `cache_control` — it's a Copilot proxy-specific field that the proxy translates to the appropriate backend format.

#### Cache Control Value
```javascript
// Always the same value:
{ type: "ephemeral" }
```

#### Feature Gate
```javascript
// Cache control is opt-in via clientOptions.enableCacheControl
enableCacheControl: e?.enableCacheControl ?? false  // defaults to OFF
```

### Where Cache Control Is Applied

#### 1. System Message — ALWAYS gets cache breakpoint
```javascript
d = [
  { role: "system", content: e, copilot_cache_control: u },  // ← always cached
  ...r.map(_7)  // rest of messages
]
```

#### 2. Last Non-Deferred Tool — gets cache breakpoint
```javascript
// Find the last tool that isn't deferred-loading
let S = u ? w.findLastIndex(_ => !_.deferLoading) : -1;

// Apply cache control only to that last tool
w.map((_, R) => {
  let T = R === S ? u : void 0;
  return {
    type: "function",
    function: { name: _.name, description: _.description, parameters: _.input_schema },
    copilot_cache_control: T,           // ← only on last non-deferred tool
    copilot_defer_loading: _.deferLoading || void 0,
    copilot_mcp_server_name: _.mcpServerName
  }
})
```

#### 3. Last Non-Skippable Message — gets cache breakpoint
```javascript
// Find the last message that shouldn't be skipped
let ne = d.findLastIndex(xe => !m6i(xe));

// Apply cache control to that message
ce = d.map((xe, He) => {
  let { outputTokens: At, ...he } = xe;
  return He === ne ? { ...he, copilot_cache_control: u } : he
})
```

### Which Models Enable Cache Control

Only **Claude/Anthropic models** routed through the Copilot proxy have `enableCacheControl: true`:

```javascript
// ZEe is the base config for Anthropic models on Copilot
ZEe = {
  ...nf,
  supports: { ...nf.supports, tool_choice: false },
  clientOptions: {
    ...nf.clientOptions,
    enableCacheControl: true,   // ← enabled for Claude models
    thinkingBudget: 1024
  }
}

// Used by these model configs:
// claude-sonnet-4, claude-sonnet-4.5, claude-opus-4.5
// claude-sonnet-4.6, claude-opus-4.6, claude-opus-4.6-1m
// claude-haiku-4.5
```

GPT models (`GBe`, `mW` configs) do NOT enable cache control.

### For Direct Anthropic API (Not Through Copilot Proxy)

When copilot talks directly to Claude (not via the proxy), it uses the standard Anthropic `cache_control`:

```javascript
system: [{
  type: "text",
  text: e,
  cache_control: { type: "ephemeral" }   // ← standard Anthropic format
}],
tools: a.map((D, X, me) => ({
  name: D.name,
  description: D.description,
  input_schema: ZNn(D.input_schema),
  ...(X === me.length - 1 ? { cache_control: { type: "ephemeral" } } : {})
}))
```

---

## 3. How opencode Handles Prompt Caching with Copilot

### Custom Forked SDK

opencode has a **custom forked AI SDK provider** specifically for Copilot at:
```
packages/opencode/src/provider/sdk/copilot/
├── copilot-provider.ts          # Provider factory
├── index.ts                      # Exports
├── openai-compatible-error.ts    # Error handling
├── chat/                         # Chat completions API
│   ├── convert-to-openai-compatible-chat-messages.ts
│   ├── openai-compatible-chat-language-model.ts
│   ├── openai-compatible-chat-options.ts
│   └── ...
└── responses/                    # Responses API
    ├── openai-responses-language-model.ts
    ├── convert-to-openai-responses-input.ts
    └── ...
```

### Transform Layer — `applyCaching()` in `transform.ts`

opencode's caching strategy tags **4 messages** with cache breakpoints:

```typescript
function applyCaching(msgs: ModelMessage[], model: Provider.Model): ModelMessage[] {
  // Cache the first 2 system messages
  const system = msgs.filter((msg) => msg.role === "system").slice(0, 2)
  // Cache the last 2 non-system messages
  const final = msgs.filter((msg) => msg.role !== "system").slice(-2)

  const providerOptions = {
    anthropic: { cacheControl: { type: "ephemeral" } },
    openrouter: { cacheControl: { type: "ephemeral" } },
    bedrock: { cachePoint: { type: "default" } },
    openaiCompatible: { cache_control: { type: "ephemeral" } },
    copilot: { copilot_cache_control: { type: "ephemeral" } },   // ← Copilot-specific
  }

  for (const msg of unique([...system, ...final])) {
    // For Anthropic/Bedrock: apply at message level
    // For Copilot/others: apply on last content part
    if (shouldUseContentOptions) {
      const lastContent = msg.content[msg.content.length - 1]
      lastContent.providerOptions = mergeDeep(lastContent.providerOptions ?? {}, providerOptions)
    } else {
      msg.providerOptions = mergeDeep(msg.providerOptions ?? {}, providerOptions)
    }
  }
  return msgs
}
```

### How `copilot_cache_control` Flows to the API Request

The message converter (`convert-to-openai-compatible-chat-messages.ts`) extracts provider metadata:

```typescript
function getOpenAIMetadata(message) {
  return message?.providerOptions?.copilot ?? {}
}

// In message building:
messages.push({
  role: "system",
  content: content,
  ...metadata,  // ← spreads copilot_cache_control onto the message JSON
})
```

So `providerOptions.copilot.copilot_cache_control` → becomes `copilot_cache_control` at the top level of the message object in the API request.

### Responses API — `prompt_cache_key`

For the Responses API path (used by GPT-5+ models), opencode uses a session-level prompt cache key:

```typescript
// In openai-responses-language-model.ts
const baseArgs = {
  model: this.modelId,
  input,
  prompt_cache_key: openaiOptions?.promptCacheKey,  // ← session ID as cache key
  // ...
}
```

This is set in `transform.ts` → `options()`:
```typescript
if (input.model.providerID === "openai" || input.providerOptions?.setCacheKey) {
  result["promptCacheKey"] = input.sessionID
}
```

### When opencode Applies Caching

The `applyCaching()` function is only called for Anthropic-family models:

```typescript
if (
  model.providerID === "anthropic" ||
  model.api.id.includes("claude") ||
  model.api.npm === "@ai-sdk/anthropic"
  // ... more conditions
) {
  msgs = applyCaching(msgs, model)
}
```

**Note**: This means Copilot-routed Claude models DO get caching applied, but only because they match `model.api.id.includes("claude")`.

### Token Tracking

Both Chat and Responses API paths report cached token usage:

```typescript
// Chat completions: usage.prompt_tokens_details.cached_tokens
// Responses API: usage.input_tokens_details.cached_tokens

// Mapped to SDK token usage:
usage: {
  inputTokens: {
    total: response.usage?.prompt_tokens,
    cacheRead: response.usage?.prompt_tokens_details?.cached_tokens ?? undefined,
    cacheWrite: undefined,  // Copilot doesn't report cache writes
  }
}
```

---

## 4. Comparison Table

| Aspect | Copilot CLI | opencode | fspec (current) |
|--------|------------|----------|-----------------|
| **Cache field name** | `copilot_cache_control` | `copilot_cache_control` | ❌ None |
| **Value** | `{ type: "ephemeral" }` | `{ type: "ephemeral" }` | N/A |
| **System msg** | Always cached | First 2 system msgs | ❌ Not cached |
| **Messages** | Last non-skippable msg | Last 2 non-system msgs | ❌ Not cached |
| **Tools** | Last non-deferred tool | Not cached separately | ❌ Not cached |
| **GPT models** | No cache control | `prompt_cache_key` (Responses API) | ❌ Not cached |
| **Anthropic direct** | Standard `cache_control` | Standard `cacheControl` | ✅ Already works |
| **Cached token tracking** | `prompt_tokens_details.cached_tokens` | Same | Unknown |

---

## 5. Implementation Plan for fspec

### What Needs to Change

fspec's Rust agent core (`codelet`) builds API requests for the Copilot provider. We need to:

1. **Add `copilot_cache_control: { type: "ephemeral" }` to the system message** when the active provider is `github-copilot` and the model is a Claude model
2. **Add `copilot_cache_control: { type: "ephemeral" }` to the last tool definition** in the tools array
3. **Add `copilot_cache_control: { type: "ephemeral" }` to the last user/assistant message** before the final user turn
4. **Track `cached_tokens`** from `usage.prompt_tokens_details.cached_tokens` in the response to display cache hit rates

### Key Files to Modify (Rust Side)

Based on PROV-053 (the Copilot provider implementation):

- **`codelet/core/src/providers/copilot/`** — The Copilot-specific provider code
- Message serialization — where messages are turned into JSON for the API request
- Tool serialization — where tool definitions are serialized
- Response parsing — where usage stats are parsed back

### Serialization Format

The `copilot_cache_control` field must appear as a **top-level field on message objects** and **tool objects**:

```json
{
  "model": "claude-sonnet-4",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful assistant...",
      "copilot_cache_control": { "type": "ephemeral" }
    },
    {
      "role": "user",
      "content": "Hello"
    },
    {
      "role": "assistant",
      "content": "Hi there!",
      "copilot_cache_control": { "type": "ephemeral" }
    },
    {
      "role": "user",
      "content": "What is 2+2?"
    }
  ],
  "tools": [
    {
      "type": "function",
      "function": { "name": "read_file", "description": "...", "parameters": {...} }
    },
    {
      "type": "function",
      "function": { "name": "write_file", "description": "...", "parameters": {...} },
      "copilot_cache_control": { "type": "ephemeral" }
    }
  ]
}
```

### Response Format for Cache Tracking

```json
{
  "usage": {
    "prompt_tokens": 1500,
    "completion_tokens": 200,
    "total_tokens": 1700,
    "prompt_tokens_details": {
      "cached_tokens": 1200
    }
  }
}
```

### Gate Condition

Only apply `copilot_cache_control` when:
- Provider is `github-copilot`
- Model ID contains `claude` (i.e., Anthropic models routed through Copilot)

GPT models on Copilot do NOT use `copilot_cache_control`.

---

## 6. Raw Evidence

### Copilot CLI Source Extraction Method
```bash
# The SEA blob is at offset 88981504 in the Mach-O binary
otool -l /opt/homebrew/bin/copilot | grep -A 5 "__NODE_SEA_BLOB"
# sectname __NODE_SEA_BLOB, offset 88981504, size 17283219

# JS extracted from ~/Library/Caches/copilot/pkg/universal/1.0.22/app.js (14MB)
# Relevant grep patterns used:
# grep -oE '.{0,200}copilot_cache_control.{0,200}' app.js
# grep -oE '.{0,200}enableCacheControl.{0,200}' app.js
# grep -oE '.{0,200}cache_control.{0,200}' app.js
```

### opencode Source Locations
```
/tmp/opencode/packages/opencode/src/provider/transform.ts        # applyCaching()
/tmp/opencode/packages/opencode/src/provider/provider.ts         # Provider registry
/tmp/opencode/packages/opencode/src/provider/sdk/copilot/        # Custom Copilot SDK
/tmp/opencode/packages/opencode/src/plugin/github-copilot/       # Copilot plugin
```
