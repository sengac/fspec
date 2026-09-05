# Provider Configuration

fspec works with any AI provider that supports tool calling. Providers are configured
through the **Provider Settings** screen (opened with `/provider`) and models are
selected through the **Model Selector** screen (opened with `/model`).

---

## Supported Providers

fspec ships with 17 built-in providers. Each can be configured via the `/provider`
screen — you can set API keys, test connections, and manage OAuth logins entirely
from the TUI.

| Provider | Environment Variable | Auth Type |
|----------|---------------------|-----------|
| Anthropic | `ANTHROPIC_API_KEY` or `CLAUDE_CODE_OAUTH_TOKEN` | API Key / OAuth |
| OpenAI | `OPENAI_BASE_URL` + `OPENAI_API_KEY` (bridged from the selected profile) | API Key (local servers, via profiles) |
| Google Gemini | `GOOGLE_GENERATIVE_AI_API_KEY` | API Key |
| Z.AI | `ZAI_API_KEY` or `ZAI_PLAN_API_KEY` | API Key |
| Codex | OAuth (`~/.codex/auth.json`) | OAuth |
| GitHub Copilot | OAuth (`~/.fspec/credentials/copilot_auth.json`) | OAuth |
| Cohere | `COHERE_API_KEY` | API Key |
| Mistral | `MISTRAL_API_KEY` | API Key |
| xAI | `XAI_API_KEY` | API Key |
| Together AI | `TOGETHER_API_KEY` | API Key |
| Hugging Face | `HF_TOKEN` | API Key |
| OpenRouter | `OPENROUTER_API_KEY` | API Key |
| Groq | `GROQ_API_KEY` | API Key |
| DeepSeek | `DEEPSEEK_API_KEY` | API Key |
| Moonshot | `MOONSHOT_API_KEY` | API Key |
| Galadriel | `GALADRIEL_API_KEY` | API Key |
| Azure OpenAI | `AZURE_OPENAI_API_KEY` | API Key |

**OpenAI-compatible APIs** — Ollama, vLLM, LM Studio, and any server implementing
the OpenAI API format work via the OpenAI provider. The `OpenAI` provider is for
**local OpenAI-protocol-compatible servers only** (vLLM, Ollama, sglang, RunPod,
Fireworks) — cloud OpenAI models (GPT-5.x, o3, GPT-4o) are NOT listed under the
`OpenAI API` section; they belong exclusively under the `Codex (ChatGPT)` provider.
You configure local servers through **OpenAI profiles** (see
[OpenAI API Profile Screen](#openai-api-profile-screen) below).

---

## The `/provider` View

Type `/provider` to open the **Provider Settings** screen
(`rust/fspec-tui/src/views/provider_settings/`). It shows every built-in provider
as a flat, expandable tree:

```
▼ Anthropic (12 models)
      ✓ sk-ant-••••••••mnop [env]
      Logout from OAuth [Anthropic]
      🔑 Sign in with browser
      🔑 Sign in with code
▶ OpenAI (2 profiles)
      📁 sglang → http://localhost:18003
      📁 runpod → https://runpod.io
      + Create new profile
▼ Codex (ChatGPT) (6 models)
      ✓ OAuth [ChatGPT]
      Logout from OAuth [Codex (ChatGPT)]
      🔑 Sign in with browser
      🔑 Sign in with device code
...
```

- Provider header rows (white, `▶`/`▼` glyph) show the credential state
  (`✓ {masked key} [source]` in green when configured, `(not configured)` in gray
  otherwise). The `OpenAI` header appends a dim `(N profile(s))` badge when it
  has stored profiles.
- Child rows appear only when the provider is expanded, in a fixed order:
  1. **OAuth status** row — `Logout from OAuth [Provider]` (only when logged in)
  2. **OAuth login** rows (magenta 🔑) — the available login methods
  3. **API key** row (yellow 🔑) — every provider *except* `openai`
  4. **Profile** rows (cyan 📁) — `openai` only: one per stored profile,
     labeled `name → baseUrl`
  5. **Add Profile** pseudo-row (green `+`, labeled "Create new profile") —
     `openai` only, always present

The `OpenAI` provider has **no API-key row**: because it is for local servers only,
its credentials are set per profile (in the profile form) instead.

### Keybindings

| Key | Effect |
|-----|--------|
| `↑` / `↓` | Navigate the flat tree (clamped, no wrap) |
| `PgUp` / `PgDn` | Page up / down by one viewport |
| `Home` / `End` | Jump to top / bottom |
| `Enter` | Contextual: expand/collapse a provider · open the API-key editor · open the profile **Edit** form · open the **Create Profile** form · start an OAuth login · open the OAuth logout (disconnect) confirm |
| `→` / `←` | Expand / collapse the focused provider header |
| `d` | Delete: provider/API-key rows → "Delete credentials?" confirm (only when configured) · profile rows → "Delete profile?" confirm (removes just that profile) · logout rows → OAuth disconnect confirm |
| `/` | Enter filter mode (case-insensitive substring match on provider name or id; a provider and its children are shown or hidden together). `Esc` clears the filter; `Enter` exits filter mode |
| `Tab` | Switch to the Model Selector (`/model`) |
| `Esc` | Close the screen (first clears an active filter) |
| `Ctrl+C` | In a text-entry mode (API-key editor, profile form field), copy the focused field's value to the clipboard — API-key fields are copied **masked** (`••••`), other fields copy plaintext |

Footer hints at the bottom of the screen always show the keybindings available
for the focused row.

### OAuth Providers

OAuth-capable providers are `anthropic`, `codex` and `github-copilot`:

| Provider | Login methods |
|----------|---------------|
| Anthropic | Browser · headless code entry (authorize URL + code) |
| Codex (ChatGPT) | Browser · device code (shows a user code + verification URL) |
| GitHub Copilot | Device code only, preceded by a deployment-type prompt (GitHub.com vs. GitHub Enterprise + host entry) |

Browser login rows only appear on transports that can run the local OAuth HTTP
server. On failure the error screen offers `Enter` to retry the last method or
`Esc` to go back; `Esc` cancels an in-flight login (late results are discarded).

### API Key Editor

`Enter` on an API-key row opens an inline editor: type the key (printable ASCII
only; `Ctrl+V` paste is supported and newlines/control characters are dropped),
`Enter` saves it to `~/.fspec/credentials/credentials.json`, `Esc` cancels.
The key renders masked (one bullet per character) while you type. Pressing
`Enter` on an empty key silently cancels.

---

## OpenAI API Profile Screen

Each **profile** points fspec at one local OpenAI-protocol-compatible server.
Open the screen from the `OpenAI` provider in `/provider`:

- `+ Create new profile` → opens the **create** form (name field focused, base
  URL pre-filled with `http://localhost:8888`)
- A `📁 name → baseUrl` row → opens the **edit** form, prefilled with the stored
  values (the name is editable here — `↑` from the first field returns to it, so
  a profile can be renamed; the original name is used as the delete key)

### Profile Form Fields

| # | Field | Type | Empty ⇒ | Saved as (`fspec-config.json`) | Effect at runtime |
|---|-------|------|---------|-------------------------------|-------------------|
| — | **Name** | text | required | map key under `providers.openai.profiles.<name>` | Identifies the profile; shown as `openai: <name>` in the Model Selector |
| 1 | **Base URL** | text | required | `baseUrl` | Bridged to `OPENAI_BASE_URL` when the profile is selected |
| 2 | **API Key** | text (masked) | required | `apiKey` | Bridged to `OPENAI_API_KEY` (only when non-empty). Also used for the `/v1/models` reachability probe |
| 3 | **Context Window** | number | registry value / 128 000 fallback | `contextWindow` | Bridged to `OPENAI_CONTEXT_WINDOW`; fallback context window for models discovered via the probe |
| 4 | **Max Output Tokens** | number | registry / provider default | `maxOutputTokens` | Per-profile cap override; deliberately NOT bridged to an env var |
| 5 | **Compaction Threshold** | `80%` or `200000` | provider default | `compactionThreshold: { type, value }` | When context compaction triggers for this profile's sessions |
| 6 | **Streaming** | toggle (default **Enabled**) | enabled | `streaming` | `OPENAI_STREAMING=true/false` is exported when the profile is selected; absent ⇒ streaming stays on |
| 7 | **Auto-Continue** | number (default: off) | off | `autoContinue` | Sessions started on this profile continue as if `/continue n` had been run; `0` = explicitly off |
| 8 | **Preserve Thinking** | toggle (default **Disabled**) | disabled | `preserveThinking` | `false` (default): reasoning blocks are **stripped** from the chat history sent back to the LLM; `true`: preserved |
| 9 | **Max Images** | number (default 4) | 4 | `maxImages` | Image budget for the Read tool: `0` = no-vision profile (Read fails image reads), `n ≥ 1` = at most `n` images per Read result |
| 10 | **Loop Detection** | toggle (default **Enabled**) | enabled | `loopDetectionEnabled` | On/off for the streaming loop detector (RIG-014) in this profile's sessions |
| 11 | **Loop Window** | number (default 160) | 160 | `loopDetectionWindow` | Detector sliding window size, in words |
| 12 | **Loop Repeat** | number (default 10) | 10 | `loopDetectionMaxRepeats` | Tail n-gram repeat threshold before a loop is declared |
| 13 | **Loop Retries** | number (default 10) | 10 | `loopDetectionMaxRetries` | Max auto-continue retries after a loop abort; `0` = never retry |

**Form navigation:** `↑`/`↓` move between fields (the name field sits above the
first field and is reached with `↑` from Base URL), printable characters edit
the focused field, and `Space`/`←`/`→` flip the boolean toggles (Streaming,
Preserve Thinking, Loop Detection) — typing while a toggle is focused is
swallowed. `Ctrl+V` pastes into the focused text field (newlines/control
characters dropped). `Enter` saves, `Esc` cancels, `Tab` is ignored.

**Validation on save:**

- Base URL, API key, or the trimmed name empty → the save is rejected
  **silently** and the form stays open.
- A non-numeric value in any numeric field (Auto-Continue, Max Images, Loop
  Window/Repeat/Retries) → the save is rejected with a hint shown in the status
  line, e.g. `Auto-Continue must be 0 (off) or a positive integer budget (e.g. 300)`,
  and the form stays open with your edits intact. Nothing is persisted.
- Compaction Threshold out of range (percentage `1..=100`, tokens `≥ 1000`) →
  the value is omitted from the saved profile rather than rejected.

### Profile Storage

Profiles are persisted to `~/.fspec/fspec-config.json` (or the `FSPEC_USER_DIR`
override), deep-merged with the project-level `<cwd>/spec/fspec-config.json`
(**project overrides user** by profile name):

```json
{
  "providers": {
    "openai": {
      "profiles": {
        "sglang": {
          "baseUrl": "http://localhost:18003",
          "apiKey": "test",
          "contextWindow": 262144,
          "maxOutputTokens": 32768,
          "compactionThreshold": { "type": "percentage", "value": 80 },
          "streaming": true,
          "autoContinue": 300,
          "preserveThinking": false,
          "maxImages": 4,
          "loopDetectionEnabled": true,
          "loopDetectionWindow": 160,
          "loopDetectionMaxRepeats": 10,
          "loopDetectionMaxRetries": 10
        }
      }
    }
  }
}
```

Saving is a **read-modify-write** that touches only the target profile:

- The profile's `customModels` array (owned by the Model Selector CRUD, see
  below) and every unrelated key (sibling profiles, top-level config) are
  preserved verbatim.
- Absent optional fields are **removed** from the profile object, so the
  defaults documented in the table above apply on reload.
- Saving a changed name renames the profile (the original entry is replaced).

**Custom models** (`customModels`) are NOT a field of this form — they are
managed via the Model Selector (`/model`) on an `openai: <profile>` section and
are preserved by every profile save/delete. Each entry is
`{ id, displayName?, facade?, contextWindow?, maxOutputTokens?, compactionThreshold?, reasoning?, hasVision? }`.

### How a Profile Behaves at Runtime

- **Listing** — when fspec assembles the provider list (`list_providers`), each
  profile's `baseUrl` is probed via `GET /v1/models` (using the profile's
  `apiKey`). Discovered model ids are merged with the profile's `customModels`
  (custom entries override discovered ones with the same id). The section is
  rendered as `openai: <name>`; it is marked *unreachable* only when the probe
  fails **and** the profile has no custom models — a profile with custom models
  is never flagged unreachable, and unlike cloud sections, an unreachable
  profile section is never dropped from the list.
- **Credential bridging** — when a profile's model is selected, the profile's
  stored credentials are bridged into the process environment before dispatch
  (`apply_profile_env_vars`):
  - `OPENAI_BASE_URL` ← `baseUrl`
  - `OPENAI_API_KEY` ← `apiKey` (only when present and non-empty)
  - `OPENAI_CONTEXT_WINDOW` ← `contextWindow` (only when set)
  - `OPENAI_STREAMING` ← `"true"`/`"false"` when `streaming` is set; **removed**
    when absent (so a profile without the flag never forces streaming off)
  - `maxOutputTokens` is deliberately **not** bridged to an env var
- **Session defaults** — `autoContinue`, `preserveThinking`, `maxImages` and the
  four loop-detection values seed the agent-loop / tool-layer settings for
  sessions created on this profile.

---

## Configuring Providers

### Via the TUI (`/provider`)

The **Provider Settings** screen (see [The `/provider` View](#the-/provider-view)
and [OpenAI API Profile Screen](#openai-api-profile-screen) above) is the primary
way to configure providers — you can set API keys, manage OAuth logins, and
configure local-server profiles without touching environment variables.

### Via Environment Variables

You can also configure providers by setting environment variables before starting
fspec. The Provider Settings screen will detect these automatically:

```bash
export ANTHROPIC_API_KEY=sk-ant-...
export OPENAI_API_KEY=sk-...
fspec
```

Credentials can come from:
- **Environment variables** — Set before starting fspec
- **`~/.fspec/credentials/credentials.json`** — Persisted via the TUI
- **`.env` file** — Loaded from the current working directory at startup

### API Key Masking

Keys are masked for safe display using prefix preservation:
- `sk-ant-api03-...mnop` → `sk-ant-••••••••mnop`
- `sk-test-...cdef` → `sk-••••••••cdef`
- Keys shorter than 12 characters → `••••••••`

---

## Selecting Models

### Via the TUI (`/model`)

Type `/model` in the input bar to open the **Model Selector** screen. This shows
all providers and their available models in a navigable tree.

**Keybindings:**
- `↑` / `↓` — Navigate the model list
- `Enter` — Select the highlighted model for the current session
- `Esc` — Close and return to the agent view
- `r` — Refresh the provider/model list
- `/` — Filter models by name
- `Tab` — Switch to the Provider Settings view

Models are fetched from the [models.dev](https://models.dev) registry and cached
locally at `~/.fspec/cache/models.json`.

### Tool Call Requirement

fspec requires `tool_call` capability. Models without tool calling support will
be rejected with an error.

---

## Custom Providers

Custom providers let you connect fspec to any LLM API. They are configured via
JSON files in:

- **Global:** `~/.fspec/providers/*.json`
- **Project-local:** `.fspec/providers/*.json` (overrides global)

Custom providers support **two modes**:

1. **Facade mode** — Route through a built-in provider (e.g., `openai`, `claude`)
2. **Rhai mode** — Full control via a `.rhai` script that defines the request/response lifecycle

### Facade Mode (Simple)

When `facade` is set to a built-in provider name, requests are routed through that
provider's code path. No Rhai script is needed.

```json
{
  "name": "my-provider",
  "display_name": "My Custom LLM",
  "base_url": "https://api.example.com/v1",
  "facade": "openai",
  "api_key_env_var": "MY_API_KEY",
  "models": {
    "default": {
      "id": "my-model",
      "context_window": 200000,
      "max_output_tokens": 4096,
      "supports_tools": true,
      "supports_streaming": true,
      "supports_thinking": false,
      "supports_vision": false
    }
  },
  "tool_style": "openai",
  "api_style": "openai_chat"
}
```

### Rhai Mode (Full Control)

When `facade` is `null` and a `script` path is provided, the provider uses a
Rhai script that defines the complete request/response lifecycle. This gives you
full control over HTTP requests, headers, URL construction, and response parsing.

```json
{
  "name": "my-provider",
  "display_name": "My Custom LLM",
  "base_url": "https://api.example.com/v1",
  "facade": null,
  "script": "my-provider.rhai",
  "api_key_env_var": "MY_API_KEY",
  "models": {
    "default": {
      "id": "my-model",
      "context_window": 200000,
      "max_output_tokens": 4096,
      "supports_tools": true,
      "supports_streaming": true,
      "supports_thinking": false,
      "supports_vision": false
    }
  }
}
```

The `.rhai` script is resolved relative to the config file's directory.

#### Required Functions

A Rhai script **must** define these 7 functions:

| Function | Purpose |
|----------|---------|
| `build_request(request)` | Build the JSON request body from `request` map (`messages`, `tools`, `thinking_config`) |
| `build_headers(config)` | Build HTTP headers map |
| `build_url(config)` | Build the request URL string |
| `parse_response(raw)` | Parse the API response into `#{ content, stop_reason, usage }` |
| `parse_stream_chunk(raw)` | Parse a streaming chunk |
| `build_stream_request(request)` | Build the streaming request body |
| `map_error(status_code, body)` | Map error responses to an error message string |

#### Optional Hooks

| Function | Purpose |
|----------|---------|
| `get_model_limits(config)` | Override `context_window`, `max_output_tokens`, and optionally `compaction_threshold` |
| `define_tools(config)` | Define custom tool schemas (PROV-098) |
| `transform_preamble(config)` | Customize the system prompt |
| `identity_prefix(config)` | Set a custom identity prefix for the system prompt |

#### Available Modules

Rhai scripts have access to these sandboxed modules:

| Module | Functions |
|--------|-----------|
| `http` | `http::post(url, body, headers)`, `http::get(url, headers)` |
| `crypto` | `crypto::sha256(data)`, `crypto::base64url_encode(data)` |
| `json` | `json::parse(s)`, `json::stringify(value)` |
| `oauth` | `oauth::generate_pkce()`, `oauth::generate_state()`, `oauth::urlencoded(s)` |
| `log` | `log::debug(msg)`, `log::info(msg)`, `log::warn(msg)`, `log::error(msg)` |
| `cred` | `cred::read()`, `cred::write(data)`, `cred::delete()`, `cred::path()` (provider-scoped) |

The `cred` module is **provider-scoped** — a script for provider `foo` can only
access credentials for `foo`, preventing cross-provider credential leakage.

#### Sandbox Limits

Rhai scripts run in a sandboxed engine with:
- **Operation limit:** 50,000 operations per call
- **Call depth limit:** 32 levels
- **Size limit:** Prevents oversized data structures

#### Example Rhai Script

```rhai
// my-provider.rhai

func build_url(config) {
  `${config.base_url}/chat/completions`
}

func build_headers(config) {
  #{
    Authorization: `Bearer ${json::parse(cred::read()).api_key}`,
    "Content-Type": "application/json",
  }
}

func build_request(request) {
  #{
    model: config.model,
    messages: request.messages,
    tools: request.tools,
    temperature: 0.7,
  }
}

func parse_response(raw) {
  let parsed = json::parse(raw);
  #{
    content: parsed.choices[0].message.content,
    stop_reason: parsed.choices[0].finish_reason,
    usage: parsed.usage,
  }
}

func parse_stream_chunk(raw) {
  // Parse SSE chunk
  let line = raw.trim();
  if line.starts_with("data: ") {
    let data = json::parse(line[6..]);
    #{ type: "content", content: data.choices[0].delta.content }
  } else {
    #{ type: "ignore" }
  }
}

func build_stream_request(request) {
  #{
    model: config.model,
    messages: request.messages,
    tools: request.tools,
    stream: true,
  }
}

func map_error(status_code, body) {
  `API error ${status_code}: ${body}`
}
```

#### Model Limits Hook

The `get_model_limits(config)` hook lets scripts override per-model limits:

```rhai
func get_model_limits(config) {
  #{
    context_window: 400000,
    max_output_tokens: 128000,
    compaction_threshold: #{
      type: "percentage",
      value: 75,
    },
  }
}
```

The `compaction_threshold` can be either:
- `#{ type: "tokens", value: 200000 }` — Fixed token count
- `#{ type: "percentage", value: 75 }` — Percentage of context window (1..=100)

---

### Configuration Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Provider slug (matches `^[a-z][a-z0-9-]*$`) |
| `display_name` | string | Yes | Human-readable name |
| `base_url` | string | Yes | API endpoint URL |
| `facade` | string \| null | No | Route through built-in provider (`openai`, `claude`) |
| `script` | string | No | Rhai script path (required if no facade) |
| `api_key_env_var` | string | No | Environment variable for API key |
| `auth` | object | No | Authentication configuration |
| `models` | object | Yes | Model definitions |
| `tool_style` | string | No | Tool calling format (`claude`, `openai`, `gemini`, `codex`) |
| `api_style` | string | No | API format (`openai_chat`, `anthropic_messages`) |

### Shadowing Built-in Providers

Custom providers can shadow built-in provider names (e.g., a custom `claude.json`).
The custom config takes precedence by default. Disable shadowing with:

```bash
FSPEC_DISABLE_SCRIPT_SHADOWING=1 fspec
```

---

## Model Limits

Context window and max output tokens are resolved via priority chain:

1. **Script override** — `get_model_limits(config)` in Rhai scripts (PROV-095)
2. **User override** (clamped by provider hard max)
3. **Registry value** from models.dev (clamped by provider hard max)
4. **Provider default** constant

---

## Troubleshooting

### "No providers configured"

Open `/provider` and configure at least one provider with an API key or OAuth login.

### Provider shows as unconfigured

Check that the environment variable is set, or use the `/provider` screen to
manually enter your API key.

### "Model does not support tool_call"

fspec requires tool calling. Choose a model with `tool_call` capability.

### Custom provider not detected

Check:
1. JSON is valid and in the correct directory
2. `name` matches `^[a-z][a-z0-9-]*$`
3. Either `facade` or `script` is configured
4. Required environment variable is set

### Rhai script errors

- Check that all 7 required functions are defined
- Use `log::debug()` / `log::info()` in your script for debugging
- The sandbox limits are 50,000 operations and 32 call depth levels
- Script path is resolved relative to the config file's directory
