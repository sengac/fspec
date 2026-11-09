# Model-specific thinking / reasoning levels: research for a work card

Date: 2026-11-03. Local code refs are `rust/<path>:<line>` relative to `/home/rquast/projects/fspec`.
External sources were read with web tools. Where a page came through a summarizer, that is noted.
Anything marked **GUESS** is an inference, not a verified fact.

---

## 0. TL;DR

- fspec has a single fixed 4-value enum, `Off | Low | Medium | High`, defined in three parallel places. It is persisted as an integer `0..=3`. Nothing in the pipeline knows which levels a given model supports.
- Real providers now use 7+ distinct values: `none, minimal, low, medium, high, xhigh, max`, plus Codex-local `ultra` and `persistent`, and Claude thinking modes `adaptive | enabled+budget | disabled | between_tools`. Support differs **per model**, not per provider.
- **The models.dev cache fspec already downloads (`~/.fspec/cache/models.json`) carries a per-model `reasoning_options` field that lists the exact supported effort values, budget ranges, or a toggle.** fspec's `ModelInfo` deserializer silently drops it (`rust/providers/src/models/types.rs:38-88` has only `reasoning: bool`). This is the cheapest source of truth.
- The current mapping has several silent drops and silent defaults, which conflicts with the no-silent-defaults rule:
  - The OpenAI Chat Completions provider ignores the level entirely.
  - Codex sends nothing for non-`-codex` model ids.
  - Z.AI and Copilot ignore it.
  - Local-server profiles (vLLM/SGLang) get nothing.
  - Gemini 3 forces `high` when unset.
  - Claude `Off` on always-on models still thinks.
  - Claude Haiku 4.5 is routed to adaptive thinking, which the API rejects with a 400.
  - Codex capabilities are never populated in production, so the "conservative" list is always used.

---

## 1. fspec today

### 1.1 The enum(s)

There are three copies of the same 4-variant enum, and they are converted by hand:

| Type | Location | Variants |
|---|---|---|
| `codelet_rpc_types::ThinkingLevel` (wire/TUI/RPC, `#[default] Off`) | `rust/rpc-types/src/chrome.rs:38-47` | Off, Low, Medium, High |
| `codelet_tools::facade::ThinkingLevel` (provider facades) | `rust/tools/src/facade/thinking_config.rs:91-107` | Off, Low, Medium, High |
| `codelet_agent_loop::thinking_config::JsThinkingLevel` (NAPI legacy mirror) | `rust/agent-loop/src/thinking_config.rs:24-33` (+ `From` at 35-44) | Off, Low, Medium, High |

Storage on a live session is a `u8`, clamped to `0..=3`:

```rust
// rust/sessions/src/background_session/metrics.rs:62-65
pub fn set_base_thinking_level(&self, level: u8) {
    let clamped = level.min(3);
```

The `u8` is converted back to the enum in three places:
- `rust/agent-loop/src/thinking_level_detection.rs:155-161`: `_ => Off`
- `rust/sessions/src/handle_impl/handle_impl_4.rs:223-232`: `_ => High`. This is inconsistent with the first.
- `rust/sessions/src/default_thinking_level_persistence.rs:38-46`: `_ => Off`

Per-turn keyword detection can raise or lower the level. It is implemented in `rust/agent-loop/src/thinking_level_detection.rs:33-87`:
- `ultrathink` / `think harder` → High
- `megathink` / `think hard` → Medium
- `think about` → Low
- `quickly` / `nothink` → Off

### 1.2 Per-turn routing to a provider

`rust/agent-loop/src/agent_loop.rs:125-133` calls `routing::resolve_turn_routing` and then `thinking::resolve_turn_thinking`.

The routing key is the **model id**, or for custom (Rhai) providers the resolved real model id (`rust/agent-loop/src/agent_loop/routing.rs:14-23`). It falls back to the provider name when there is no model (`rust/agent-loop/src/agent_loop/thinking.rs:18,73`).

`get_thinking_config(key, level)` (`rust/agent-loop/src/thinking_config.rs:88-119`) dispatches on **string patterns** of that key:
- `is_gemini3_provider` matches an exact list: `gemini-3`, `gemini-3-pro`, `gemini-3-flash`, `*-preview` (`:59-68`). So `gemini-3.1-pro-preview` and `gemini-3.5-flash` do **not** match.
- `is_gemini25_provider` matches an exact list (`:72-82`).
- `is_claude_provider` is `starts_with("claude")`.
- `is_codex_provider` is `provider == "codex" || ends_with("-codex")` (`:48-50`).
- Everything else returns `{}`, with no warning:

```rust
// rust/agent-loop/src/thinking_config.rs:115-117
    } else {
        serde_json::json!({})
    };
```

The resulting JSON goes into the provider's `create_rig_agent(.., thinking_config)` through `rust/agent-loop/src/agent_loop/providers.rs:39-122`.

### 1.3 Per-provider mapping

| Provider path | What is sent per level | Evidence |
|---|---|---|
| **Claude, adaptive** (`is_adaptive_thinking_model`: any `claude-*` that is not `claude-3*`, `claude-opus-4-5*` or `claude-sonnet-4-5*`) | Off → no `thinking` key. Low/Med/High → `{"thinking":{"type":"adaptive","display":"summarized"},"output_config":{"effort":"low\|medium\|high"}}` | `rust/tools/src/facade/thinking_config.rs:34,48-65,241-269` |
| **Claude, budgeted** (Opus/Sonnet 4.5, 3.x) | Off → `{}`. Low → `{"thinking":{"type":"enabled","budget_tokens":4096}}`, Med → 16000, High → 32000 | `rust/tools/src/facade/thinking_config.rs:277-298` |
| Claude merge | The facade JSON is merged into `additional_params` next to `system`. The interleaved-thinking beta header is added only for non-adaptive models. | `rust/providers/src/claude.rs:590-605`, `:97-99` |
| **Gemini 3** | `thinkingConfig.thinkingLevel` low/medium/high with `includeThoughts`. Off → `{}`. **If nothing is sent and the model contains `gemini-3`, it is forced to `high`** ("You cannot disable thinking for Gemini 3 Pro") | `rust/tools/src/facade/thinking_config.rs:127-150`. Silent default at `rust/providers/src/gemini.rs:243-262` |
| **Gemini 2.5** | `thinkingBudget` 2048 / 4096 / 8192 | `rust/tools/src/facade/thinking_config.rs:174-200` |
| **Codex** (ChatGPT backend, Responses API) | Only when the model id is `codex` or ends in `-codex`: `{"reasoning":{"effort":"low\|medium\|high","summary":"auto"}}`. Any other model id (e.g. `gpt-5.5` via Codex) gets `{}` | `rust/agent-loop/src/thinking_config.rs:100-112`. Injected at `rust/providers/src/codex/agent.rs:108-110` |
| Codex request validation | `resolve()` checks `reasoning.effort`, `summary`, `context`, `verbosity` and `service_tier` against `CodexCapabilities`. An explicit unsupported value is an error (`RequestOptionsError::Unsupported`). An absent value falls back to `defaults` (effort `high`) | `rust/providers/src/codex/request_options.rs:16-69,139-160`, `types.rs:5-15,126-157` |
| Codex capabilities source | `set_capabilities` is called **only from tests** (`rust/providers/tests/prov152_*`). Production always uses `CodexCapabilities::conservative()`: efforts `["none","minimal","low","medium","high"]` (no `xhigh`/`max`), default effort `high`. That is a silent fallback whenever capabilities are absent or stale | `rust/providers/src/codex/request_options.rs:20-21`, `types.rs:139-156`. Grep for `set_capabilities` finds no non-test caller |
| `Effort` wire type | Already open-ended: `#[serde(untagged)] enum Effort { Named(String), Numeric(u64) }` | `rust/providers/src/codex/request_options/types.rs:17-33` |
| **OpenAI (API key)**, rig `openai::completion::CompletionModel` (Chat Completions) | **Ignored**: `_thinking_config` is "currently unused for OpenAI". No `reasoning_effort` is ever sent | `rust/providers/src/openai/agent.rs:71,86`, `rust/providers/src/openai/construction.rs:145` |
| **Local-server OpenAI profiles** (`providers.openai.profiles.*`: your `sglang` and runpod SGLang profiles) | Same OpenAI path, so **nothing** is sent. No `reasoning_effort` and no `chat_template_kwargs`. The only thinking-related profile field is `preserveThinking`, which strips prior reasoning from history | `LocalServerProfile` at `rust/sessions/src/profile_sections/config.rs:9-87` (`preserveThinking` :43-44). `CustomModelDef` (`reasoning: Option<bool>`, `facade`) at :99-124. Strip logic at `rust/core/src/history_strip.rs:19-29`. A grep for `chat_template_kwargs\|enable_thinking` in `rust/` finds nothing |
| **Z.AI (GLM)** | Ignored: "GLM models handle reasoning internally". Only temperature/top_p are sent | `rust/providers/src/zai.rs:219-226,312-319` |
| **GitHub Copilot** | Ignored: "Copilot's chat/completions endpoint has no thinking budget primitive". However, Copilot's `/models` returns `capabilities.supports.reasoning_effort`, which fspec stores verbatim in `ModelInfo.options["reasoning_variants"]` and never uses | `rust/providers/src/copilot/rig_agent.rs:26-28,44-59`, `rust/providers/src/copilot/models/schema.rs:81-97`, `rust/providers/src/copilot/models/builder.rs:16-17,51-56,94` |
| **Custom Rhai providers** | The facade JSON (chosen by the *resolved real model id*) is passed raw as `request.thinking_config`, or `()` when absent. The script decides what to do with it. For a non-Claude/Gemini/Codex id the script receives `{}` | `rust/providers/src/custom/request_bridge.rs:53-88`, `rust/providers/src/custom/custom_provider.rs:221-224`, `rust/agent-loop/src/agent_loop/routing.rs:14-23` |

### 1.4 TUI selection

- The dialog uses a hard-coded 4-row list (`rust/fspec-tui/src/components/thinking_level_dialog.rs:26-31`):

  ```rust
  (ThinkingLevel::Off, "Off", "No extended thinking"),
  (ThinkingLevel::Low, "Low", "~4K tokens, quick analysis"),
  (ThinkingLevel::Medium, "Medium", "~10K tokens, balanced"),
  (ThinkingLevel::High, "High", "~32K tokens, deep reasoning"),
  ```

  These token hints match neither the Claude budgets (4096 / 16000 / 32000) nor the Gemini 2.5 budgets (2048 / 4096 / 8192). They mean nothing for effort-based models.
- The dialog opens with the current session's level (`rust/fspec-tui/src/app/dispatch_model_thinking_dialogs.rs:18-37`). `D` sets the default. The `(default)` marker comes from `load_default_thinking_level_opt`.
- The inline `/thinking off|low|med|medium|high` command is handled at `rust/fspec-tui/src/app/slash_parser.rs:128-134`. Anything else returns `InvalidThinkingLevel`.
- The RPC surface is `set_thinking_level(session_id, ThinkingLevel)` (`rust/rpc/src/service_declaration/providers_sessions.rs:100`), `get_thinking_level` (`rust/rpc/src/service_declaration/workspace.rs:129`) and `set_thinking_level_default` (`rust/rpc/src/service_declaration/supervision_credentials.rs:97`).
- The model picker shows only a boolean `[R]` badge, from `ModelEntry.supports_reasoning`:
  - cloud: `m.reasoning` from models.dev (`rust/sessions/src/cloud_models.rs:112`)
  - profiles: `CustomModelDef.reasoning` (`rust/sessions/src/profile_sections.rs:123`)
  - Rhai: `supports_thinking` (`rust/sessions/src/handle_impl/handle_impl_8.rs:54`)

  The thinking dialog does not consult it. Switching models does not touch the session level.

### 1.5 Persistence

- The default level is stored in `fspec-config.json` → `tui.defaultThinkingLevel` as an int `0..=3` (`rust/sessions/src/default_thinking_level_persistence.rs:19,34-36,87-90`). Your user config currently has `"defaultThinkingLevel": 3`.
- **Silent degradation:** "A missing / malformed config, or an out-of-range value, loads as `ThinkingLevel::Off`" (doc comment `:20-21`; `load_default_thinking_level_with_dirs` at `:126-138`).
- Every new session applies it (`rust/sessions/src/session_creation_helper.rs:228-235`), whatever the model is.

### 1.6 Model catalog: per-model reasoning capability info

The catalog has more than fspec reads.

- fspec `ModelInfo` (`rust/providers/src/models/types.rs:38-88`) has `reasoning: bool` and `interleaved`, but **no field for `reasoning_options`**. serde ignores unknown keys, so the data is dropped.
- The cache (`~/.fspec/cache/models.json`, 5.9 MB, 8,932 models) has `reasoning_options` on 6,845 models. Every model with `reasoning: true` has the key, though 1,135 have an empty list. Three shapes appear (counted from the cache):
  - `{"type":"effort","values":[...]}` on 4,586 models
  - `{"type":"toggle"}` on 1,427
  - `{"type":"budget_tokens","min"?,"max"?}` on 609
- The effort vocabulary in the cache is: `none, minimal, low, medium, high, xhigh, max`. The cache has 40+ distinct value combinations, for example `[low,high,max]`, `[none,high]`, `[high]`, `[medium,high,xhigh]`.
- Examples from the cache:
  - `openai/gpt-5`: `effort [minimal,low,medium,high]`
  - `openai/gpt-5.1`: `[none,low,medium,high]`
  - `openai/gpt-5.2`, `gpt-5.5`: `[none,low,medium,high,xhigh]`
  - `openai/gpt-5.6*`, `gpt-6-luna/sol`, `gpt-6.1-terra`: `[none,low,medium,high,xhigh,max]`
  - `openai/gpt-6-astra`, `gpt-6.1-sol`: `[low,medium,high,xhigh,max]`
  - `openai/gpt-5-pro`: `[high]`
  - `openai/*-pro` (5.2+): `[medium,high,xhigh]`
  - `openai/o1`, `o3`, `o4-mini`: `[low,medium,high]`
  - `anthropic/claude-opus-4-5`: `[effort low,medium,high] + budget_tokens{min:1024}`
  - `anthropic/claude-opus-4-6`, `sonnet-4-6`: `[low,medium,high,max] + budget_tokens{min:1024}`
  - `anthropic/claude-opus-4-7` … `opus-5-5`, `sonnet-5-5`, `fable-5*`: `[low,medium,high,xhigh,max]`
  - `anthropic/claude-haiku-4-5`, `sonnet-4-5`: `budget_tokens{min:1024}` only
  - `anthropic/claude-sonnet-5`, `haiku-5-5`: `toggle` + `effort [low..max]`
  - `google/gemini-2.5-pro`: `budget_tokens{min:128,max:32768}`
  - `google/gemini-2.5-flash`: `toggle` + `budget_tokens{0..24576}`
  - `google/gemini-3-flash-preview`, `3.5-flash`: `[minimal,low,medium,high]`
  - `google/gemini-3.1-pro-preview`: `[low,medium,high]`
  - `zai/glm-4.5` … `glm-5.1`: `toggle`
  - `zai/glm-5.2`: `toggle + [high,max]`
  - `zai/glm-5.3`: `[low,high,max]`
- Codex: fspec builds the Codex model list by re-parenting the **models.dev OpenAI catalog** (`rust/sessions/src/cloud_models.rs:179-181`). It does **not** call the Codex backend `/models` endpoint, so it never sees `supported_reasoning_levels` / `default_reasoning_level` (see §3). A grep for `backend-api/codex/models` in `rust/providers/src` finds nothing.

### 1.7 Bugs and conflicts with the no-silent-defaults rule found while tracing

1. Claude Haiku 4.5 is not in `BUDGETED_THINKING_MODELS` (`rust/tools/src/facade/thinking_config.rs:34`), so `is_adaptive_thinking_model("claude-haiku-4-5")` is true and fspec sends `type: adaptive`. Anthropic's docs list Haiku 4.5 as "Extended only", rejecting `adaptive` with a 400: "adaptive thinking is not supported on this model" (see §4).
2. Claude `Off` on always-on models (Opus 5.5, Fable 5/5.1, Sonnet 5.5, Haiku 5.5, Sonnet 5, Opus 5): fspec omits `thinking`. Per the docs, thinking is then *on* at the model's default effort, so the user's "Off" is silently not honoured. The model-correct expressions are `between_tools` (Sonnet 5.5), `disabled` (Opus 5 / Haiku 5.5, only at effort ≤ high), or not possible at all (Opus 5.5).
3. Claude `xhigh` / `max` cannot be reached, even though they are supported (§4).
4. OpenAI Chat Completions provider, local profiles, Z.AI and Copilot ignore the level. The TUI still offers all 4 levels, and the header shows the level as active.
5. Codex: no reasoning params for non-`-codex` ids, and production capabilities are always `conservative()`, so `xhigh` / `max` would be rejected by fspec's own validator.
6. Gemini 3: forced `high` when unset. Newer Gemini ids (`gemini-3.1-*`, `3.5-*`, `gemini-pro-latest`) don't match the exact list, so they get `{}` and then the forced `high`.
7. Inconsistent `u8 → enum` fallbacks (`_ => Off` vs `_ => High`) and lossy load-as-Off.

---

## 2. OpenAI API

Source: openai-python `src/openai/types/shared/reasoning_effort.py` on `main`:

```python
ReasoningEffort: TypeAlias = Optional[Literal["none", "minimal", "low", "medium", "high", "xhigh", "max"]]
```

https://github.com/openai/openai-python/blob/main/src/openai/types/shared/reasoning_effort.py

openai-openapi `components.schemas.ReasoningEffort`: `enum: [none, minimal, low, medium, high, xhigh, max]`, `default: medium` (https://github.com/openai/openai-openapi/blob/main/openapi.yaml).

The docstring (chat `completion_create_params.py` and `shared/reasoning.py`) says: "Currently supported values are `none`, `minimal`, `low`, `medium`, `high`, `xhigh`, and `max`… Not all reasoning models support every value."

- **Chat Completions:** top-level `reasoning_effort`.
- **Responses API:** `reasoning: { effort, summary, context, mode }`.
  - `summary: "auto" | "concise" | "detailed"` ("`concise` is supported for `computer-use-preview` models and all reasoning models after `gpt-5`").
  - `context: auto | current_turn | all_turns`.
  - `mode: standard | pro`.
  - `generate_summary` is deprecated.
- **When each value was added** (git history of `reasoning_effort.py`):
  - `minimal`: 2025-08-07 (GPT-5)
  - `none`: 2025-11-13 (gpt-5.1)
  - `xhigh`: 2025-12-04 (gpt-5.1-codex-max)
  - `max`: 2026-07-09 (SDK 2.45.0)
- **Per-model rules** (older SDK docstring, commit `752a293da5`):
  - "`gpt-5.1` defaults to `none`… supported reasoning values for `gpt-5.1` are `none`, `low`, `medium`, and `high`"
  - "All models before `gpt-5.1` default to `medium`… and do not support `none`"
  - "The `gpt-5-pro` model defaults to (and only supports) `high`"
  - "`xhigh` is supported for all models after `gpt-5.1-codex-max`"

  The current guide (https://developers.openai.com/api/docs/guides/reasoning, read via summarizer, so these are near-verbatim) says:
  - "Supported values are model-dependent… Defaults are also model-dependent rather than universal."
  - "`gpt-5.5` defaults to `medium`"
  - "GPT-6 Astra does not support `none`"
  - "GPT-6.1 Sol does not support `none` or `minimal` and defaults to `medium`"
- **Errors:** the guide says setting `none` returns HTTP 400 (on models that don't support it). No generic statement for other unsupported values was found.
- This matches the models.dev `reasoning_options` lists in §1.6.

## 3. OpenAI Codex CLI (codex-rs, `main` @ `036e160181`)

`C = https://github.com/openai/codex/blob/main/codex-rs`

- **Enum** (`$C/protocol/src/openai_models.rs:71-170`): `None, Minimal, Low, #[default] Medium, High, XHigh, Max, Ultra, Persistent, Custom(String)`.
  - `Custom` is documented as "A model-defined effort value that this client does not know yet."
  - The serde impl is hand-written through `as_str` / `FromStr`, so `XHigh` goes over the wire as `"xhigh"`, unknown strings become `Custom`, and an empty string is an error.
  - The JSON schema is an open string: "A non-empty reasoning effort value advertised by the model."
- **Discovery**: `ModelInfo`, "Model metadata returned by the Codex backend `/models` endpoint", has these fields:
  - `default_reasoning_level: Option<ReasoningEffort>`
  - `supported_reasoning_levels: Vec<ReasoningEffortPreset { effort, description }>`
  - `supports_reasoning_summary_parameter`
  - `default_reasoning_summary`
  - `multi_agent_reasoning_effort`

  The endpoint is `GET {base}/models?client_version=…` (`$C/codex-api/src/endpoint/models.rs`, base e.g. `https://chatgpt.com/backend-api/codex`). A bundled fallback lives at `$C/models-manager/models.json`. Example entry:

  ```json
  {"slug":"gpt-5.5","default_reasoning_level":"medium",
   "supported_reasoning_levels":[{"effort":"low",...},{"effort":"medium",...},{"effort":"high",...},
     {"effort":"xhigh","description":"Extra high reasoning depth for complex problems"}], ...}
  ```

  Bundled levels per model:
  - gpt-6* / gpt-5.6*: `low, medium, high, xhigh, max[, ultra]`
  - gpt-5.5: `low, medium, high, xhigh`
  - No bundled entry lists `none` or `minimal`.
- **Sending**: the wire struct is `Reasoning { effort, summary, context }` (`$C/codex-api/src/common.rs:160`). `build_reasoning` (`$C/core/src/client.rs:1004-1022`) does:

  ```
  effort.or_else(|| model_info.default_reasoning_level).map(resolve_reasoning_effort)
  ```

  It sends `summary` only if the model supports it and the value is not `None`.
  - `Ultra` resolves to `multi_agent_reasoning_effort`, else `Max`, else the highest level.
  - `Persistent` resolves to `"disabled"` on the wire.
- **Unsupported level handling**: there are three behaviours.
  - On a model switch (`$C/core/src/session/turn_context.rs:794-813`), Codex silently picks the **middle** supported level, `supported[(len-1)/2]`, and then falls back to the default.
  - For sub-agents (`$C/core/src/agent/child_config.rs:355`) it is a hard error: "Reasoning effort `{requested}` is not supported for model `{model}`. Supported reasoning efforts: {supported}".
  - No client-side check of the startup config value was found.

  Codex's silent middle-pick is exactly what fspec's rule forbids. Don't copy it.
- **TUI** (`$C/tui/src/chatwidget/model_popups.rs:475,719`): a per-model popup lists `supported_reasoning_efforts` with a "(default)" tag and a "(current)" tag.
  - `Max` and `Ultra` sit behind "More reasoning…".
  - With only one choice, the popup is skipped.
  - Labels are None/Minimal/Low/Medium/High/"Extra high"/Max/Ultra/Persistent, or the raw custom string.

## 4. Anthropic Claude

Sources: https://platform.claude.com/docs/en/build-with-claude/extended-thinking, …/thinking, …/effort, …/thinking-troubleshooting, …/api/errors, and anthropic-sdk-python types.

- **Extended (budget) thinking**: `thinking: {type:"enabled", budget_tokens}`.
  - "Minimum of 1,024 tokens." It must be less than `max_tokens`, except with interleaved manual mode.
  - It is deprecated: "Claude 4.7 and later models do not support it and reject requests that use it, returning a 400 error." On Opus/Sonnet 4.6, "`budget_tokens` is deprecated."
- **Adaptive thinking**: `thinking: {type:"adaptive", display?: "summarized"|"omitted"}`.
  - Depth is controlled by `output_config: {effort}`.
  - "On Claude 5 and later models… thinking is already on and needs no configuration."
  - "On Claude Opus 4.8, 4.7, 4.6, and Sonnet 4.6, thinking is off until you set `thinking: {type: "adaptive"}`".
- **Effort** (`output_config.effort`, no beta header): SDK `Literal["low","medium","high","xhigh","max"]`.
  - `max`: Fable 5.1, Mythos 5.1, Fable 5, Mythos 5, Mythos Preview, Opus 5.5, Opus 5, Opus 4.8, 4.7, 4.6, Sonnet 5.5, 5, 4.6, Haiku 5.5.
  - `xhigh`: the same list **minus** Opus 4.6, Sonnet 4.6 and Mythos Preview ("Not every model that supports `max` supports `xhigh`").
  - Opus 4.5 supports only low/medium/high (effort plus budget). Haiku 4.5 and Sonnet 4.5 have no effort support.
  - Defaults: `high` everywhere except Opus 5.5 and Haiku 5.5, which default to `medium`.
- **Thinking off differs per model** (thinking-troubleshooting table):

  | Model | Off behaviour |
  |---|---|
  | Fable 5.1 / 5, Mythos 5.1 / 5, Opus 5.5 | `enabled` and `disabled` both rejected with a 400 (cannot turn off) |
  | Opus 5, Haiku 5.5 | `disabled` only at effort ≤ high |
  | Sonnet 5.5 | `disabled` rejected; use `{type:"between_tools"}` (effort ≤ high) |
  | Opus 4.8 / 4.7 | default Off; `enabled` rejected |
  | Opus 4.6 / Sonnet 4.6 | adaptive and (deprecated) extended both accepted |
  | Opus 4.5 / Haiku 4.5 / Sonnet 4.5 | extended only; `adaptive` rejected |

- **Error examples** (400 `invalid_request_error`):
  - `"thinking.type.enabled" is not supported for this model. Use "thinking.type.adaptive" and "output_config.effort"…`
  - `adaptive thinking is not supported on this model`
  - `output_config.effort 'xhigh' is not supported when thinking is disabled on this model…`

  No exact error string was found for sending `xhigh` to Opus 4.6.
- **Programmatic capability**: the Models API `capabilities` exposes `thinking.types.disabled` and `thinking.types.between_tools`, each with `.supported` (models overview). Per-message effort changes need the beta header `mid-conversation-output-config-2026-07-01`.
- Implication: for Claude, "level" is really two settings, a thinking **mode** (adaptive / enabled+budget / disabled / between_tools) and an **effort**. Whether "Off" is possible depends on the model.

## 5. OpenAI-compatible local servers: vLLM and SGLang

All references are to `main` as of 2026-11-03, fetched from raw.githubusercontent.com.

### 5.1 vLLM (github.com/vllm-project/vllm)

- **`reasoning_effort`** is a request field: `vllm/entrypoints/openai/chat_completion/protocol.py` L269, with the description "Currently supported values are none, minimal, low, medium, high, xhigh, and max… Note that 'max' is specific to the DeepSeek V4 series". The type is defined in `vllm/renderers/params.py` L27:

  ```python
  ReasoningEffort = Literal["none", "minimal", "low", "medium", "high", "xhigh", "max"]
  ```

- **It is passed to the template** in `build_chat_params` (protocol.py ~L649-678):

  ```python
  reasoning_effort = map_reasoning_effort(self.reasoning_effort, reasoning_effort_map)
  extra_kwargs = dict(add_generation_prompt=..., documents=self.documents, reasoning_effort=reasoning_effort)
  user_kwargs = self.chat_template_kwargs or {}
  if reasoning_effort is not None and "enable_thinking" not in user_kwargs:
      extra_kwargs["enable_thinking"] = reasoning_effort != "none"
  ```

  The server-side `--reasoning-effort-map` (`vllm/entrypoints/common/cli_args.py` L110) "rewrites a request's reasoning effort to a value the chat template supports… Example: '{"medium": "low", "xhigh": "max"}'". In other words, the remapping is an operator decision on the server, not something the client does silently.
- **`chat_template_kwargs: dict | None`** is at protocol.py L381. The server-side `--default-chat-template-kwargs` is merged, with request values taking precedence. Per-model toggles, from `docs/features/reasoning_outputs.md` L36-42:
  - "IBM Granite 3.2 and DeepSeek-V3.1 reasoning is disabled by default; to enable it, you must also pass `thinking=True` in your `chat_template_kwargs`."
  - "The reasoning feature for the Qwen3 series is enabled by default. To disable it, you must pass `enable_thinking=False`."
  - Gemma 4: `enable_thinking=True`, or set `reasoning_effort`.

  Auto-injection sets only `enable_thinking`, never `thinking`. So DeepSeek-V3.1 still needs an explicit `chat_template_kwargs.thinking`.
- **Parsers**: `--reasoning-parser <name>`, e.g. `deepseek_r1`, `qwen3`, `glm45`, `gemma4`, `granite`. The response field is now **`reasoning`**. From the doc: "`reasoning` used to be called `reasoning_content`". The protocol.py L88 field is `reasoning: str | None`. Request fields include `include_reasoning` and `thinking_token_budget` (L280-281).
- **Responses API**: `ResponsesRequest.reasoning.effort` goes through the same map and `enable_thinking` injection (`vllm/entrypoints/openai/responses/protocol.py` L497-520).
- **gpt-oss / Harmony** (`vllm/entrypoints/openai/parser/harmony_utils.py` L68-71, L123-133) accepts only low/medium/high:

  ```python
  REASONING_EFFORT = {"high": ReasoningEffort.HIGH, "medium": ReasoningEffort.MEDIUM, "low": ReasoningEffort.LOW}
  if reasoning_effort not in REASONING_EFFORT: raise VLLMValidationError(... "is not supported by Harmony ...")
  ```

  `none` is also rejected (`vllm/renderers/online_renderer.py` ~L575: "Harmony does not support {reasoning_effort=}").

### 5.2 SGLang (github.com/sgl-project/sglang, `python/sglang/srt/entrypoints/openai/`)

- **`reasoning_effort`** (protocol.py L986): "Accepts string levels ('none', 'minimal', 'low', 'medium', 'high', 'xhigh', 'ultra', 'max') or a float in [0.0, 0.99]… 'none' defaults thinking and enable_thinking to false in chat_template_kwargs… Not supported in the harmony path." It also accepts an OpenRouter-style `reasoning: {effort | enabled}` object.
- **Normalisation** (`normalize_reasoning_inputs`, protocol.py L1108-1166) sets **both** model-family keys:

  ```python
  if effort is not None: thinking = effort != "none"
  if thinking is not None:
      # - "thinking" for deepseek-v3, kimi_k2
      # - "enable_thinking" for qwen3, glm45, nemotron_3, interns1
      ctk.setdefault("thinking", thinking)
      ctk.setdefault("enable_thinking", thinking)
  ```

  Because it uses `setdefault`, explicit `chat_template_kwargs` win.
- **`serving_chat.py`**:
  - `chat_template_kwargs["reasoning_effort"]` is promoted to `request.reasoning_effort` (L1313-1329).
  - At render (L2186-2189), `extra_template_kwargs["reasoning_effort"] = request.reasoning_effort`, then `chat_template_kwargs` is merged in.
  - There are model-specific shims, e.g. Kimi K3 maps to `thinking_effort` (L775-789).
- **Output**: `--reasoning-parser` (`server_args.py` L370) plus `separate_reasoning` (default true) put the text in **`reasoning_content`** (protocol.py L1310, L1371). Request fields `separate_reasoning`, `stream_reasoning` and `chat_template_kwargs` are at L1030-1032.
- **Responses API**: only `effort == "none"` is translated to `{"thinking": False, "enable_thinking": False}` (L2044-2049).

### 5.3 Takeaways for fspec's OpenAI-profile path

- Sending top-level `reasoning_effort` is the most portable single knob. vLLM, SGLang and Strata all accept it. SGLang derives `thinking`/`enable_thinking` from it. vLLM derives only `enable_thinking`.
- The *legal values* depend on the model's chat template. For example, gpt-oss accepts only low/medium/high; Qwen3 has a binary toggle; DeepSeek V4 accepts up to `max`.
- None of these servers advertise per-model supported levels. **GUESS / not verified for vLLM/SGLang**: their `/v1/models` doesn't carry it. For Strata this was checked: its `/v1/models` payload is `id`, `status`, `meta.n_ctx` and `architecture` modalities only (`serve/server.py` ~L7773-7790). So the supported set for a local model must be **declared in the fspec profile / `CustomModelDef`**.
- fspec currently sends nothing on this path (§1.3).

## 6. "strata"

**Identification: `github.com/Niko1221/Strata`, high confidence.**
- I verified it via the GitHub API: 38,481 stars, last pushed 2026-11-02. Its description reads "Qwen3.8-Flash-Next on any consumer hardware… Strata inference engine, OpenAI/Anthropic API on localhost".
- It serves `/v1/chat/completions`, `/v1/responses`, `/v1/messages` and `/v1/models`, and has its own ecosystem: strata-manager, the PolyStrata wrapper, and forks such as talus-public and Xolotl.
- Other candidates checked and judged less likely:

  | Repo | Stars | What it is |
  |---|---|---|
  | RustingStudio/RustingStrata | 14 | Rust MoE engine |
  | BlockYugaLabs/Strata | 6 | C++ Qwen3.8 engine |
  | ai-deeptech/stratarouter | — | semantic router |
  | Klavis AI Strata | — | MCP router, not inference |
  | OpenGamma/Strata, mjackson/strata, lgse/strata, Advtik/Strata | — | unrelated |

- Caveat: your saved profiles are named "sglang" and "…on SGLang" (`~/.fspec/fspec-config.json`). It is possible "strata" refers to something else you run. Please confirm.

**How it passes thinking settings** (`serve/frontend.py`, `serve/responses.py`, `serve/server.py`, main):

- The effort table (frontend.py L132-135, verified):

  ```python
  # Thinking levels.  The model's template knows low, medium and xhigh (its default; "high" means xhigh), and ...
  EFFORT = {"none": None, "off": None, "minimal": None, "disabled": None, "false": None,
            "low": "low", "medium": "medium", "high": "xhigh", "xhigh": "xhigh", "max": "xhigh", "maximum": "xhigh"}
  ```

  `EFFORT_DEEPSEEK` (L140-141) maps high→high, xhigh/max→max, and also accepts integers 1..100. Unknown values give a `ValueError` and a 400 ("unknown reasoning effort").
- **Strata itself remaps silently.** `high` becomes `xhigh`, `max` becomes `xhigh`, `minimal` becomes off. So the same string means different things on Strata than on OpenAI, which is another argument for per-model declared levels rather than one global vocabulary.
- **Chat Completions** (frontend.py ~L607-617): it reads `reasoning_effort`, else `reasoning.effort`. It honours `chat_template_kwargs.enable_thinking=false` and `chat_template_kwargs.reasoning_effort`, and ignores other ctk keys such as DeepSeek `thinking`.
- **Responses API** (responses.py ~L456-460): `reasoning.effort`.
- **Anthropic `/v1/messages`**: `thinking.type=disabled`, `budget_tokens` mapped to a level ("under 2K low, under 8K medium, else high"), and `output_config.effort`.
- A server-config `reasoning_effort` applies when the request sends none (server.py ~L5222-5244). The bundled Qwen template defaults to `xhigh`. The server-side settings validator says `"reasoning_effort: none, low, medium or high"` (server.py ~L8900-8902).
- **Output field**: `reasoning_content` (server.py ~L6618, L6839), the same as SGLang. vLLM uses `reasoning`.

---

## 7. Recommendation: modelling levels in fspec

These are options and trade-offs. Items marked **DECISION** need the product owner.

### 7.1 Core model

1. **Replace the closed `Off/Low/Medium/High` enum with an open, validated level identifier.** For example, a newtype `ReasoningLevel(String)` with a known-ordering table: `none < minimal < low < medium < high < xhigh < max < ultra`. Unknown strings are allowed only when the model metadata advertises them, like Codex's `Custom(String)` (§3). A closed enum would need a new release for every new provider value: Codex `ultra`/`persistent`, SGLang floats, Strata integers, Kimi `thinking_effort`.
2. **Introduce a per-model `ReasoningCapability` descriptor**, resolved alongside model limits (the PROV-165 `ResolvedModelLimits` pattern). Rough shape:
   - an ordered list of supported effort values
   - an optional budget range `{min, max}`
   - a toggle flag
   - whether "off" is possible, and how: `effort none` | `thinking disabled` | `between_tools` | template toggle | not possible
   - an optional default
   - the *source* of each item, for diagnostics

   Sources, in precedence order. Each one is explicit, and a missing descriptor means "no reasoning control offered", never a guessed default.
   - a. **Live provider metadata**:
     - Codex backend `GET /models` → `supported_reasoning_levels`, `default_reasoning_level` (§3). fspec doesn't call it today, and `set_capabilities` has no production caller.
     - Copilot `/models` → `capabilities.supports.reasoning_effort`. Already stored as `options["reasoning_variants"]`, unused.
     - Anthropic Models API `capabilities.thinking.types.*` (§4).
   - b. **models.dev `reasoning_options`**. It is already in the cache and needs one new serde field on `providers/src/models/types.rs::ModelInfo`. It covers OpenAI, Anthropic, Google, Z.AI and thousands of others (§1.6).
   - c. **User-declared in profile config** for local servers (`CustomModelDef`, e.g. a new `reasoningEfforts: ["none","low","medium","high"]`, and optionally `reasoningWire`, see 7.2). This is the only option for vLLM, SGLang and Strata (§5.3, §6).
   - **DECISION:** whether a profile *server-level* declaration is allowed as well as per-model, and how conflicts between (a) and (b) are reported. Recommendation: (a) wins, and a mismatch is logged.
3. **Provider wire "dialects" as an explicit mapping table**, replacing the string-pattern routing in `rust/agent-loop/src/thinking_config.rs:48-119`. Dispatch on provider type plus capability descriptor, not on model-id substrings.

   | Dialect | Wire |
   |---|---|
   | openai-chat | `reasoning_effort: <level>` |
   | openai-responses / codex | `reasoning: {effort, summary}` |
   | anthropic-adaptive | `thinking: {type: adaptive, display}` + `output_config.effort` |
   | anthropic-budget | `thinking: {type: enabled, budget_tokens}` |
   | anthropic-off | `disabled`, or `between_tools` for Sonnet 5.5 |
   | gemini-level | `thinkingConfig.thinkingLevel` |
   | gemini-budget | `thinkingConfig.thinkingBudget` |
   | local-openai | `reasoning_effort`, plus profile-declared `chat_template_kwargs` for toggles like `enable_thinking` / `thinking` |
   | rhai | pass `{level, capability}` to the script, which owns the wire |

   **GUESS / not researched:** the Z.AI GLM wire format (`thinking: {type: enabled|disabled}` per Z.AI docs) needs verifying before Z.AI is wired in.
4. **Budget-only models** (Claude Haiku 4.5 / Sonnet 4.5, Gemini 2.5). models.dev gives only `{min,max}`. Two options:
   - expose named levels with an explicit fspec-owned budget table per dialect (today's 4096/16000/32000 and 2048/4096/8192, hard-coded)
   - expose a numeric budget picker

   **DECISION.** Note that the current dialog labels ("~4K/~10K/~32K") match neither table.

### 7.2 No-silent-defaults implications

Every case below is a silent behaviour today. Each should become either "not offered" or an explicit error.
- If the selected level is not in the model's supported list, the request is refused before send, with an error naming the model and its supported values. The Codex request validator already does this for explicit values (`RequestOptionsError::Unsupported`, `rust/providers/src/codex/request_options/types.rs:9-14`). Do **not** copy Codex's middle-entry pick on model switch (`$C/core/src/session/turn_context.rs:794-813`), and do not copy Strata's high→xhigh remap.
- If the provider path cannot carry a level (e.g. a model with an empty `reasoning_options` list, or a profile with no declaration), the TUI hides or disables the thinking control and the header shows that explicitly, rather than accepting a level and dropping it. This applies today to the OpenAI provider, Z.AI, Copilot and local profiles.
- Remove or replace the following:
  - the Gemini 3 forced `high` (`rust/providers/src/gemini.rs:243-262`)
  - the Codex `conservative()` fallback with default `high` (`rust/providers/src/codex/request_options/types.rs:139-156`, `rust/providers/src/codex/request_options.rs:20-21`)
  - the `_ => {}` routing (`rust/agent-loop/src/thinking_config.rs:115-117`)
  - load-as-Off and the inconsistent `u8` fallbacks (§1.7)
- "Off" must be honest. Where the model cannot disable thinking (Opus 5.5, Fable 5/5.1, Mythos 5/5.1 per §4), "Off" is not offered. Where off is only legal at effort ≤ high (Opus 5, Haiku 5.5, Sonnet 5.5 `between_tools`), the combination must be validated.

### 7.3 TUI selector and persisted default

- The selector lists only the **current model's** supported levels, in canonical order, with a `(default)` marker for the model default where known and a `(saved default)` marker.
  - If the model has a single level, or none, show it read-only or disable `/thinking`. Codex skips the popup in this case (§3).
  - Rare or expensive levels could sit behind "More…" (Codex puts `max`/`ultra` there). **DECISION.**
  - `/thinking <x>` accepts any advertised value. An unadvertised value is an error listing the valid ones.
- Persisted default: migrate `tui.defaultThinkingLevel` from int `0..=3` to a string. The migration must be explicit (0→off, 1→low, 2→medium, 3→high). A failed parse must surface an error, not load as Off.
  - **DECISION:** keep a single global default, or add a per-model map (e.g. `tui.thinkingLevels["anthropic/claude-opus-5-5"] = "xhigh"`). A single global value is ambiguous across vocabularies: for example, "high" on Strata is really xhigh, and "off" doesn't exist on Opus 5.5.
- When the saved default, or the session's level after a model switch, is not supported by the new model, there are three candidate behaviours. **DECISION:**
  - (a) The session starts with **no explicit level**. fspec sends no reasoning parameter, so the provider's documented default applies, and the header and dialog show that plainly, e.g. "model default". This is arguably not a *silent* default if it is displayed and logged, but the product owner should confirm that it satisfies the rule.
  - (b) Block the first turn until the user picks a supported level, via the dialog.
  - (c) Refuse the model switch.

  Recommendation: (a) with a visible notice, or (b). Never pick a nearest or middle value.
- Prompt keywords (`ultrathink`, `megathink`, `think about`, `nothink`, at `rust/agent-loop/src/thinking_level_detection.rs`) currently produce fixed Low/Medium/High. **DECISION:** what they mean on a model with `[low, high, max]` or `[none, high]`. Two options:
  - map them to relative positions on that model's list, e.g. ultrathink → highest non-"More…" level, as an explicit documented rule
  - disable them when the target is absent

### 7.4 Separate bugs worth their own cards

These are independent of the redesign:
1. Claude Haiku 4.5 is routed to adaptive thinking, and the API rejects it with a 400 (`rust/tools/src/facade/thinking_config.rs:34`).
2. Claude "Off" is ignored on always-on models (§1.7 #2).
3. The OpenAI provider never sends `reasoning_effort` (`rust/providers/src/openai/agent.rs:71,86`).
4. Codex sends no reasoning for non-`-codex` model ids, and production capabilities are never set.
5. The Gemini 3.x id matcher is an exact list and misses `gemini-3.1-*` / `3.5-*`.

### 7.5 Open questions for the product owner

- Confirm "strata" = Niko1221/Strata, and whether your current local runs are on Strata or SGLang.
- Should fspec fetch Codex `/models` (needs ChatGPT auth plus `client_version`)? Or is models.dev's OpenAI list acceptable for Codex? The Codex catalog differs: no `none`/`minimal` in the bundled Codex presets vs `none` in models.dev for gpt-5.5.
- Should Claude effort and thinking mode be exposed as one combined "level" list (e.g. `off*, low, medium, high, xhigh, max`, with `*` only where legal) or as two controls?
- Should SGLang's float efforts and Strata's integer efforts (1..100) be in scope?
- Should per-turn effort changes on Claude (beta `mid-conversation-output-config-2026-07-01`, with cache invalidation) be in scope? Today fspec can change the level per turn via keywords.

## 8. Product decisions (2026-11-04, product owner)

1. Local servers: both SGLang and Strata (Niko1221/Strata) are supported; Strata is the one currently in use.
2. Codex levels: fetch Codex `/models` (ChatGPT auth + client_version) as the authority.
3. Claude: one combined level list (e.g. off*, low, medium, high, xhigh, max; off only where legal).
4. Numeric efforts (SGLang floats, Strata 1..100): in scope.
5. Per-turn Claude effort changes (beta mid-conversation-output-config-2026-07-01, cache invalidation): in scope.
6. Default level: remembered per model.
7. A saved level the model does not support: the session's level is "not set" and the user is told why.
8. Prompt keywords (ultrathink / megathink / think about / nothink): removed. Levels change only through the thinking dialog or `/thinking`.
9. Budget-only models (Claude Haiku 4.5 / Sonnet 4.5, Gemini 2.5): the user picks a numeric token budget within the model's advertised min..max. fspec invents no named levels or budget table for them.
10. The thinking dialog lists every level the model supports, in canonical order; nothing is hidden behind "More…".
11. The work is split: MODEL-009 is the core slice (open level type, per-model capability, refusal of unsupported levels); the wire dialects, TUI selector and per-model default, keyword removal, Codex `/models`, Claude per-turn effort and local-server levels are separate MODEL cards.
12. MODEL-016 covers vLLM and every other major self-hosted inference engine (llama.cpp server, Ollama, LM Studio, TGI, TensorRT-LLM, LMDeploy), not only SGLang and Strata.
13. MODEL-016 research must clone each engine's repository to /tmp and confirm its reasoning parameters from source, with file:line references, before scenarios are written.
