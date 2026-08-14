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
| OpenAI | `OPENAI_API_KEY` | API Key |
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
the OpenAI API format work via the OpenAI provider.

---

## Configuring Providers

### Via the TUI (`/provider`)

Type `/provider` in the input bar to open the **Provider Settings** screen. This is
the primary way to configure providers — you can set API keys, test connections,
and manage OAuth logins without touching environment variables.

**List mode (default):**
- `↑` / `↓` — Navigate providers
- `Enter` — Open detail for the selected provider
- `d` — Delete credentials (with confirmation dialog)
- `/` — Enter filter mode to search providers
- `Esc` — Close the screen (or clear filter first)
- `Tab` — Switch to the Model Selector

**Detail mode (after pressing Enter on a provider):**
- `Enter` — Open API key editor (for API-key providers) or OAuth login flow (for OAuth providers)
- `r` — Refresh the provider's model cache
- `Esc` — Return to list mode

**API key editor:**
- Type your API key
- `Enter` — Save credentials
- `Esc` — Cancel

**OAuth providers** (Codex, GitHub Copilot, Claude):
- Select the provider in list mode, press `Enter`
- Choose login method (browser, device code, or headless)
- Follow the OAuth flow prompts

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
