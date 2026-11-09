# MODEL-016 research: how self-hosted inference engines take reasoning / thinking parameters

Date: 2026-11-04. This is the REQUIRED FIRST STEP of MODEL-016 (product owner decision 13, MODEL-009 research §8).
Every engine with public source was cloned with `git clone --depth 1` into `/tmp/asst/model016-repos/` and read
from source. References are `<repo>/<path>:<line>` at the commit listed. Anything not read from source is marked
**DOCS** (public documentation) or **GUESS** (inference).

| Engine | Repository | Commit (date) |
|---|---|---|
| vLLM | vllm-project/vllm | `0db3734` (2026-11-04) |
| SGLang | sgl-project/sglang | `851642a` (2026-11-04) |
| Strata | Niko1221/Strata | `ffa80d7` (2026-11-02) |
| llama.cpp server | ggml-org/llama.cpp | `4c4ca5b` (2026-11-04) |
| Ollama | ollama/ollama | `e9fde36` (2026-11-03) |
| LM Studio | no public server source; lmstudio-ai/lmstudio-js (SDK only) | `4b9af90` (2026-10-28) |
| Hugging Face TGI | huggingface/text-generation-inference | `b4adbf2` (2026-03-21) |
| TensorRT-LLM (trtllm-serve) | NVIDIA/TensorRT-LLM | `9b955693` (2026-11-04) |
| LMDeploy | InternLM/lmdeploy | `3228395` (2026-11-03) |

All engines below are reached by fspec's local-server profile path, which speaks OpenAI Chat Completions
(`/v1/chat/completions`). So the Chat Completions request is the main subject. Other endpoints (Responses,
Anthropic Messages, native APIs) are noted where they change the picture.

---

## 0. Summary

| Engine | `reasoning_effort` (Chat Completions) | Numeric effort | `chat_template_kwargs` | Token budget field | Advertises per-model levels? |
|---|---|---|---|---|---|
| vLLM | `none minimal low medium high xhigh max`; anything else is a 400 | no | yes; `enable_thinking` auto-set from effort, `thinking` never | `thinking_token_budget` (int, -1 = unlimited) | no (`/v1/models` has none) |
| SGLang | `none minimal low medium high xhigh ultra max`, or a float 0.0..0.99 | float 0.0..0.99 | yes; both `thinking` and `enable_thinking` set from effort | `custom_params.thinking_budget` (needs a server flag) | no |
| Strata | `none off minimal disabled false low medium high xhigh max maximum`, remapped onto the template's levels; else 400 | integer 1..100, DeepSeek-V4.1 family only | only `enable_thinking: false` and `reasoning_effort` are read | `reasoning_budget_tokens` / `thinking_budget_tokens` | no |
| llama.cpp | any string, passed to the template; `none` turns thinking off; non-strings ignored with a log warning | no | yes, all keys reach the template; `enable_thinking` also parsed | `reasoning_budget_tokens` / `thinking_budget_tokens` | **yes**: `GET /props` → `chat_template_caps.reasoning_effort_levels` |
| Ollama | string; `minimal`→`low`, `xhigh`/`ultra`→`max` when the model has no thinking metadata; 400 on a non-thinking model | no | not read (Go ignores the unknown field) | none | **yes**: `POST /api/show` → `thinking {values, default}` |
| LM Studio | **DOCS**: not listed for `/v1/chat/completions`; `reasoning.effort` on `/v1/responses` for gpt-oss | no (DOCS) | not documented | not documented | **DOCS, yes**: `GET /api/v1/models` → `capabilities.reasoning {allowed_options, default}` |
| TGI | not a field; silently ignored | no | not a field; silently ignored | none | no |
| TensorRT-LLM | `low medium high max none`; default `low`; only the Harmony (gpt-oss) and Kimi K3 paths read it | no | yes; unread keys are a 400 except `enable_thinking`/`thinking` | `thinking_token_budget` | no |
| LMDeploy | `low medium high max`; else 400 | no | yes; plus deprecated top-level `enable_thinking` | none | no |

Key cross-engine facts:
- There is no common vocabulary. The same string differs per engine: Strata sends `high` as `xhigh`, Ollama sends
  `xhigh` as `max`, SGLang Kimi K3 drops `medium`, LMDeploy's DeepSeek-V4 template drops `low`/`medium`, and
  TensorRT-LLM maps `max` to Harmony `high`. The legal set depends on engine **and** model template.
- Several engines silently ignore or remap a level they cannot use (TGI, llama.cpp non-string, SGLang Kimi K3 and
  single-level templates, LMDeploy DeepSeek-V4, TensorRT-LLM generic path, Ollama named levels). An engine-side
  refusal can't be relied on, so the supported set has to be declared to fspec (MODEL-009 research §5.3).
- Only three engines advertise per-model levels at runtime: llama.cpp `/props`, Ollama `/api/show`, and (DOCS)
  LM Studio `/api/v1/models`. vLLM, SGLang, Strata, TGI, TensorRT-LLM and LMDeploy do not.
- Numeric efforts are **not token budgets**. SGLang floats 0.0..0.99 and Strata integers 1..100 go in the
  `reasoning_effort` field. Token budgets are separate fields with different names per engine
  (`thinking_token_budget`, `reasoning_budget_tokens`, `custom_params.thinking_budget`).
- Template toggles differ by model family: `enable_thinking` (Qwen3, GLM-4.5, Nemotron 3, InternS1, Gemma 4,
  Granite 4.2) vs `thinking` (DeepSeek-V3.x, Kimi K2, Granite 3.2).

---

## 1. vLLM

**Chat Completions request fields**
- `reasoning_effort: ReasoningEffort | None` — `vllm/vllm/entrypoints/openai/chat_completion/protocol.py:269-279`.
  The description says "none, minimal, low, medium, high, xhigh, and max… 'max' is specific to the DeepSeek V4
  series". The type is `Literal["none", "minimal", "low", "medium", "high", "xhigh", "max"]`
  (`vllm/vllm/renderers/params.py:27`), so any other string, or a number, fails pydantic validation (400).
  **No numeric efforts.**
- `thinking_token_budget: ThinkingTokenBudget = None` — `.../chat_completion/protocol.py:280`. It is validated by
  `vllm/vllm/sampling_params.py:65-91`: a non-negative integer, or -1 for unlimited.
- `include_reasoning: bool = True` — `.../chat_completion/protocol.py:281`.
- `chat_template_kwargs: dict[str, Any] | None` — `.../chat_completion/protocol.py:381-387`.

**How effort reaches the template** (`build_chat_params`, `.../chat_completion/protocol.py:645-680`)
- `map_reasoning_effort(self.reasoning_effort, reasoning_effort_map)` (`:651-653`) applies the server operator's
  `--reasoning-effort-map` (`vllm/vllm/renderers/params.py:53-61`; CLI field
  `vllm/vllm/entrypoints/common/cli_args.py:110`). Unmapped efforts pass unchanged.
- `reasoning_effort` is put in the template kwargs (`:655-660`).
- `enable_thinking = (reasoning_effort != "none")` is injected only if the caller's `chat_template_kwargs` has no
  `enable_thinking` (`:666-668`). `thinking` (DeepSeek-V3.1 / Granite 3.2) is **never** derived. A template that
  doesn't declare the variable drops it (comment at `:662-665`).
- Merge order: `merge_kwargs(self.chat_template_kwargs, extra_kwargs)` (`:673-676`, `params.py:33-45`), so the
  server-derived `reasoning_effort` overrides a caller `chat_template_kwargs.reasoning_effort` when both are set.
- Per-model toggles (DOCS, `vllm/docs/features/reasoning_outputs.md:36-42`): Granite 3.2 and DeepSeek-V3.1 need
  `thinking=True`; Qwen3 and Granite 4.2 disable with `enable_thinking=False`; Gemma 4 enables with
  `enable_thinking=True` or any `reasoning_effort`.

**Model-specific refusals**
- Harmony (gpt-oss) takes only `low`/`medium`/`high`: `vllm/vllm/entrypoints/openai/parser/harmony_utils.py:68-72`,
  `:123-133` raises `VLLMValidationError`. `none` is refused at `vllm/vllm/renderers/online_renderer.py:580-589`.

**Other endpoints**
- Responses: `reasoning.effort` goes through the same map and `enable_thinking` injection
  (`vllm/vllm/entrypoints/openai/responses/protocol.py:488-514`).
- Anthropic Messages: `output_config.effort` (`low|medium|high|xhigh|max`) becomes `reasoning_effort`, and
  `thinking.enabled.budget_tokens` (>= 1024) becomes `thinking_token_budget`
  (`vllm/vllm/entrypoints/anthropic/protocol.py:209-252`, `vllm/vllm/entrypoints/anthropic/serving.py:719-761`).

**Discovery**: `/v1/models` `ModelCard` has `id, object, created, owned_by, root, parent, max_model_len,
permission` only (`vllm/vllm/entrypoints/serve/engine/protocol.py:105-114`). It does not advertise levels.

## 2. SGLang

**Chat Completions request fields** (`sglang/python/sglang/srt/entrypoints/openai/protocol.py`)
- `reasoning_effort: ReasoningEffortType` (`:989-1003`). Its type (`:880-895`) is
  `Literal["none","minimal","low","medium","high","xhigh","ultra","max"]` **or a float in [0.0, 0.99]**
  (`Field(ge=0.0, le=0.99, allow_inf_nan=False)`). Booleans are refused (`:1109-1114`). The description says `max`
  "models that don't support it treat it the same as 'high'".
- `thinking: Optional[Any]`, e.g. `{"type": "adaptive"}`, read only by templates with a thinking mode (`:1004-1007`).
- `separate_reasoning: bool = True` (`:1033`), `chat_template_kwargs: Optional[Dict]` (`:1035`).
- `custom_params` / `custom_logit_processor` (`:1051-1052`).

**Normalisation** (`normalize_reasoning_inputs`, `:1116-1174`)
- It also accepts an OpenRouter-style `reasoning: {effort | reasoning_effort, enabled | enable}` object. Numeric or
  numeric-string efforts become floats (`:1137-1149`).
- `thinking = (effort != "none")` (`:1159-1161`). Note that a float, including `0.0`, counts as thinking on.
- Then `ctk.setdefault("thinking", thinking)` and `ctk.setdefault("enable_thinking", thinking)` (`:1163-1172`),
  covering both families: "thinking" for deepseek-v3 / kimi_k2, "enable_thinking" for qwen3 / glm45 / nemotron_3 /
  interns1. Because it uses `setdefault`, an explicit caller value wins.

**Serving** (`sglang/python/sglang/srt/entrypoints/openai/serving_chat.py`)
- `chat_template_kwargs["reasoning_effort"]` is promoted to `request.reasoning_effort` (`:1321-1337`, `:1840-1842`).
  Harmony (gpt-oss) refuses `none` (`:1331-1334`).
- Render: `extra_template_kwargs["reasoning_effort"] = request.reasoning_effort`, then the caller's
  `chat_template_kwargs` overlay it (`:2198-2201`).
- Silent degradations, logged only:
  - A template with a single-level `effort_kwarg` treats `medium`/`high`/`max` as default thinking (`:2203-2212`).
  - Kimi K3 maps `low`/`high`/`max` to `thinking_effort`, and any other value falls back to "the encoder default"
    (`:775-791`).
- Float efforts are consumed natively by the Inkling renderer
  (`sglang/python/sglang/srt/entrypoints/openai/serving_inkling.py:304-333`), which maps names to floats
  (none 0.0, minimal 0.1, low 0.2, medium 0.7, high 0.9, xhigh/max 0.99) and refuses values outside [0.0, 0.99].
  **GUESS**: other templates receive the float verbatim as `reasoning_effort`, and what happens next depends on the
  template.

**Budget**
- Chat Completions has no `max_thinking_tokens`. The budget goes in `custom_params: {"thinking_budget": N}`
  (DOCS `sglang/docs/docs/advanced_features/thinking_budget.mdx:87-93`), and needs `--enable-strict-thinking` or
  `--enable-custom-logit-processor`.
- The native `/generate` endpoint's `max_thinking_tokens` without `--enable-strict-thinking` is a 400
  (`sglang/python/sglang/srt/managers/tokenizer_manager.py:957-967`). The runtime reads the `thinking_budget` key at
  `:1641-1644` and `sglang/python/sglang/srt/sampling/custom_logit_processor.py:107-129`.

**Responses**: only `effort == "none"` is translated, to `{thinking: false, enable_thinking: false}`
(`protocol.py:2040-2056`).

**Discovery**: `ModelCard` has `id, object, created, owned_by, root, parent, max_model_len` (`protocol.py:87-96`).
It does not advertise levels.

## 3. Strata (Niko1221/Strata, the engine in use per decision 8.1)

**Effort parser** (`Strata/serve/frontend.py`)
- Default (Qwen) table, `EFFORT` (`:132-135`): `none/off/minimal/disabled/false` → thinking off;
  `low`→low; `medium`→medium; `high`→**xhigh**; `xhigh`→xhigh; `max`/`maximum`→**xhigh**.
- DeepSeek-V4.1 table, `EFFORT_DEEPSEEK` (`:138-141`): `high`→high; `xhigh`/`max`/`maximum`→max.
- `use_family("deepseek41")` switches tables and turns on numeric efforts (`:146-151`). It is called from
  `Service.set_family` (`Strata/serve/server.py:4612-4623`), which is set from the loaded model's architecture
  (`Strata/serve/server.py:9847`).
- `effort_kwargs` (`:154-169`):
  - JSON `false` → `enable_thinking: false`.
  - **An integer 1..100 is accepted only on the deepseek41 family** and goes to the template as
    `reasoning_effort: N`.
  - An unknown value is a `ValueError` → 400: "unknown reasoning effort …: use none, low, medium or high
    [, max or 1..100]".
  - **On the default Qwen family, integers are refused.**

**Chat Completions** (`Strata/serve/frontend.py:606-618`)
- It reads `reasoning_effort`, else `reasoning.effort`.
- From `chat_template_kwargs` it honours only `enable_thinking` when falsy, and `reasoning_effort`. A truthy
  `enable_thinking` and DeepSeek-style `thinking` are ignored.

**Budget**
- `reasoning_budget_tokens`, or the llama.cpp name `thinking_budget_tokens`, a whole number (0 or less = none).
  Otherwise the config default applies (`Strata/serve/server.py:4827-4844`).
- DeepSeek-V4.1 with nothing configured gets `DS41_DEFAULT_BUDGET = 32768` (`Strata/serve/server.py:219`, `:4837`).

**Server-side defaults**
- Shared settings accept `reasoning_effort` only as `none|low|medium|high`, plus `reasoning_budget_tokens`
  (`Strata/serve/server.py:8885-8920`). These apply when a request sends none (Responses: `Strata/serve/responses.py:457-463`).

**Anthropic Messages** (`Strata/serve/frontend.py:726-749`)
- `thinking.disabled` → off.
- `output_config.effort` → the effort table.
- `budget_tokens` → a real budget, plus a level: under 2K low, under 8K medium, else high (`:172-178`).
- With config `anthropic_thinking: on_request`, a request without `thinking` doesn't think (`:741-748`,
  `Strata/serve/server.py:9947-9949`).

**Discovery**: `/v1/models` returns `id, object, status, meta.n_ctx, architecture` modalities, and aliases
(`Strata/serve/server.py:7773-7790`). It does not advertise levels.

## 4. llama.cpp server (llama-server)

**Chat Completions** (`llama.cpp/tools/server/server-common.cpp`)
- All `chat_template_kwargs` keys go to the template, JSON-dumped (`:1481-1485`).
- `chat_template_kwargs.enable_thinking` true/false overrides the server default. A string value is an error
  (`:1487-1495`).
- `reasoning_effort` (`:1497-1506`):
  - `"none"` → `enable_thinking = false`, and the template's `reasoning_effort` is removed.
  - Any other non-empty string goes to the template as `reasoning_effort` **unchecked**.
  - The value is read with `json_value(..., std::string(""))`. A non-string, such as a number, is logged as "Wrong
    type supplied … using default value" and **silently ignored** (`llama.cpp/tools/server/server-common.h:45-57`).
  - **No numeric efforts.**
- Budget: `reasoning_budget_tokens`, else `thinking_budget_tokens`, else the server's `--reasoning-budget`. It
  applies only when the template has thinking end tags (`:1518-1531`). CLI: `-1` unrestricted, `0` immediate end,
  `N>0` budget (`llama.cpp/common/arg.cpp:3714-3721`).
- Server flags: `--reasoning on|off|auto` sets the default `enable_thinking` (`llama.cpp/common/arg.cpp:3684-3700`);
  `--reasoning-effort LEVEL` sets the default template `reasoning_effort` (`:3701-3712`).

**Other endpoints**
- Responses: `reasoning.effort` is copied to `reasoning_effort`, "Only 'effort' is handled so far"
  (`llama.cpp/tools/server/server-chat.cpp:304-311`).
- Anthropic: `thinking.enabled.budget_tokens` (default 10000) → `thinking_budget_tokens`
  (`llama.cpp/tools/server/server-chat.cpp:624-632`).

**Discovery: llama.cpp advertises the template's levels.**
- `GET /props` returns `chat_template_caps` (`llama.cpp/tools/server/server-context.cpp:5697`), built by
  `common_chat_templates_get_caps` (`llama.cpp/common/chat.cpp:1946-1955`). It includes
  `supports_reasoning_effort` and `reasoning_effort_levels`.
- The levels are found by probing (`llama.cpp/common/jinja/caps.cpp:581-642`):
  - Render the template with each of `minimal low medium high xhigh max`.
  - Drop levels whose render equals an unknown level's render.
  - Collapse aliases that render identically, keeping the highest.
- It doesn't probe `none`. That's handled by `enable_thinking`.

## 5. Ollama

**OpenAI-compatible Chat Completions** (`ollama/openai/openai.go`)
- `ReasoningEffort *string` and `Reasoning *Reasoning` (`:125-126`). `reasoning.effort` wins over
  `reasoning_effort` (`:790-796`). The field is a string, so a number fails JSON decoding (400). **No numeric
  efforts.**
- `ThinkingFromReasoningEffort` (`:582-614`):
  - `"none"` → `think: false`.
  - When the model has no thinking metadata: `minimal`→`low`, `xhigh`/`ultra`→`max`, then
    `ValidateLegacyThinking` allows only `low|medium|high|max` (`ollama/api/types.go:1254-1263`). Anything else is a
    400 "invalid reasoning value".
  - When the model has metadata: the requested name is passed through unchanged, or becomes `true` for
    boolean-only models.
- `chat_template_kwargs` is not a field. Go's decoder ignores unknown fields, so it is silently dropped.
- **On a model without the thinking capability, any effort other than `none` is a 400**
  (`"<model>" does not support thinking`). This is in `ollama/server/routes.go:3212-3227`. The Chat Completions
  middleware does not set `relax_thinking` (`ollama/middleware/openai.go:465-470`). Only the Responses
  (`ollama/middleware/openai.go:1250-1253`) and Anthropic (`ollama/middleware/anthropic.go:816`) middlewares relax
  it to nil.

**Native API**: `think` is a bool or a model-defined level string (`ollama/api/types.go:107-112`,
`ollama/types/model/thinking.go:61-148`). `true` means `medium` as a string (`thinking.go:118-131`). For the
llama-server runner, `think` becomes `chat_template_kwargs {enable_thinking, reasoning_effort}`
(`ollama/llm/llama_server.go:2331-2357`).

**Discovery: Ollama advertises per-model controls.**
- `POST /api/show` returns `thinking: {values: [bool|string…], default}` (`ollama/api/types.go:806`,
  `ollama/server/routes.go:1951-1953`).
- It is computed by `Model.Thinking()` (`ollama/server/model_thinking.go:63-88`). A non-thinking model reports
  `{values: [false], default: false}`.
- Per `Thinking.Supports` (`thinking.go:38-50`), "Other names may still be accepted by the endpoint and resolve to
  the default". So unadvertised names can be accepted silently.

No token-budget field was found.

## 6. LM Studio (no public server source)

LM Studio's server (`llmster` / the app) is closed source. The public repo `lmstudio-ai/lmstudio-js` (cloned) is a
client SDK. Its shared types expose only reasoning *parsing* (`lmstudio-js/packages/lms-shared-types/src/llm/LLMPredictionConfig.ts:38-46`)
and a `reasoning?: BooleanOrMixed` model flag (`lmstudio-js/packages/lms-shared-types/src/VirtualModelDefinition.ts:63-66`).
It has no request-side effort or template-kwarg handling. Everything else here is **DOCS**:
- `/v1/chat/completions` (https://lmstudio.ai/docs/developer/openai-compat/chat-completions) lists supported payload
  parameters `model, top_p, top_k, messages, temperature, max_tokens, stream, stop, presence_penalty,
  frequency_penalty, logit_bias, repeat_penalty, seed`. It does not list `reasoning_effort`, `chat_template_kwargs`
  or a budget.
- API changelog (https://www.lmstudio.ai/docs/developer/api-changelog):
  - 0.3.29: "Reasoning support with `reasoning.effort` for `openai/gpt-oss-20b`" (Responses API).
  - 0.3.23: reasoning moved to `choices.message.reasoning` / `choices.delta.reasoning`.
  - 0.3.9: `reasoning_content` for DeepSeek R1.
- Native REST `POST /api/v1/chat` (https://lmstudio.ai/docs/developer/rest/chat): `reasoning` is a string
  `off|low|medium|high|on`. "Will error if the model being used does not support the reasoning setting". The default
  is the model's automatic setting.
- `GET /api/v1/models` (https://lmstudio.ai/docs/developer/rest/list) returns
  `capabilities.reasoning: {allowed_options: ("off"|"on"|"low"|"medium"|"high")[], default}`. Example (Gemma 4):
  `{"allowed_options": ["off", "on"], "default": "on"}`.
- Third-party (RubyLLM provider page, unverified) claims `/v1/chat/completions` honours `reasoning_effort`
  `none|minimal|low|medium|high|xhigh`. **Not confirmed by LM Studio's own docs.**

## 7. Hugging Face TGI

- The repository is in **maintenance mode** (`text-generation-inference/README.md:2`).
- `ChatRequest` (`text-generation-inference/router/src/lib.rs:827-937`) has no `reasoning_effort`, no
  `chat_template_kwargs` and no budget field.
- It derives `Deserialize` without `deny_unknown_fields`, and none is set anywhere under `router/`. So these fields
  are **silently ignored**.
- The template receives only `messages, bos_token, eos_token, add_generation_prompt, tools`
  (`router/src/lib.rs:1205-1211`, `router/src/infer/chat_template.rs:109-117`). A template's thinking toggle can't be
  set per request.
- TGI has no reasoning control at all.

## 8. TensorRT-LLM (trtllm-serve)

**Chat Completions** (`TensorRT-LLM/tensorrt_llm/serve/openai_protocol.py`)
- All request models use `extra="forbid"` (`:152-154`). **An unknown request field is a 400.**
- `reasoning_effort: Optional[ReasoningEffort | Literal["low","medium","high","max","none"]]`, with
  **default `ReasoningEffort.LOW`** (`:1157-1164`). **No numeric efforts.**
- `thinking: ChatCompletionThinkingParam` (Kimi/Moonshot `{type: enabled|disabled, keep, effort: low|high|max}`,
  `:1092-1102`, `:1169`).
- `thinking_token_budget: Optional[int]` (`:1170`). It is validated as non-negative or -1
  (`TensorRT-LLM/tensorrt_llm/sampling_params.py:47-56`). It is refused on the Harmony path
  (`TensorRT-LLM/tensorrt_llm/serve/openai_server.py:3251-3254`).
- `chat_template_kwargs` (`:1242-1246`).

**Who reads `reasoning_effort`**
- Harmony (gpt-oss): `maybe_transform_reasoning_effort` maps low/medium/high, and `max`→**high** (labelled "nearest
  harmony level"); `none`→no effort (`TensorRT-LLM/tensorrt_llm/serve/harmony_adapter.py:1972-1988`; used at
  `TensorRT-LLM/tensorrt_llm/serve/chat_tokenization.py:82`).
- Kimi K3 extension: `none`→`thinking=false`; `low|high|max`→`thinking_effort`; other values (e.g. `medium`) are
  ignored (`TensorRT-LLM/tensorrt_llm/serve/extensions/kimi_k3.py:76-91`).
- **GUESS from a search of `inputs/`, `llmapi/` and `serve/` for consumers**: the generic HF-template Chat
  Completions path never passes `reasoning_effort` to the template. Only the caller's `chat_template_kwargs` reach
  it (`TensorRT-LLM/tensorrt_llm/serve/openai_server.py:2136`). For Qwen3 / DeepSeek-style models, thinking is set
  through `chat_template_kwargs` (`enable_thinking` / `thinking` / `reasoning_effort`).

**Unused-kwargs guard** (`TensorRT-LLM/tensorrt_llm/inputs/chat_template_guard.py`)
- A caller `chat_template_kwargs` key the template never reads raises `UnusedChatTemplateKwargsError` → 400
  (`:19-25`, `:121-145`). It can be turned off with `TRTLLM_ALLOW_UNUSED_CHAT_TEMPLATE_KWARGS` (`:16`).
- `enable_thinking`, `thinking` and `force_nonempty_content` are **always allowed** (`:27-45`).
- **`reasoning_effort` in `chat_template_kwargs` is refused for a template that doesn't read it.**

**Responses**: `reasoning.effort` → `ctk.setdefault("reasoning_effort", effort)` plus `thinking: true`; `none` →
`thinking: false` (`TensorRT-LLM/tensorrt_llm/serve/responses_utils.py:1636-1650`). Injected keys are exempt from the
guard (`:1653-1665`).

**Discovery**: `ModelCard` has `id, object, created, owned_by` (`openai_protocol.py:254-258`). It does not advertise
levels.

## 9. LMDeploy

**Chat Completions** (`lmdeploy/lmdeploy/serve/openai/protocol.py`, `ChatCompletionRequest` at `:171`)
- `reasoning_effort: Literal['low', 'medium', 'high', 'max'] | None` (`:204`). Any other value, including `none`,
  `xhigh` or a number, is a 400. **No numeric efforts, and no `none`.**
- `enable_thinking: bool | None`, top level, "will be deprecated" (`:218-221`). `chat_template_kwargs` (`:231-235`).
  Unknown fields are ignored (pydantic default; no `extra="forbid"` on this model).
- The deprecated `enable_thinking` is copied into `chat_template_kwargs.enable_thinking` unless already set
  (`lmdeploy/lmdeploy/serve/core/chat_runner.py:328-332`, `lmdeploy/lmdeploy/serve/core/async_engine.py:520-527`).
- `reasoning_effort` wins over `chat_template_kwargs.reasoning_effort` and is passed to the template
  (`lmdeploy/lmdeploy/serve/processors/multimodal.py:408-416`). HF templates get it as a kwarg, or have it dropped
  when None (`lmdeploy/lmdeploy/model.py:853-854`).
- The DeepSeek-V4 template keeps only `high`/`max`. `low`/`medium` are silently dropped to None
  (`lmdeploy/lmdeploy/model.py:592`, `:611-613`). It treats `thinking` or `enable_thinking` as the toggle
  (`:615-617`).
- Anthropic: `thinking.type != disabled` → `enable_thinking` (`lmdeploy/lmdeploy/serve/anthropic/endpoints/messages.py:77-78`).
  "separate `budget_tokens` enforcement is unsupported" (`lmdeploy/lmdeploy/serve/anthropic/protocol.py:98-101`).
- No token-budget field.

**Discovery**: `ModelCard` has `id, object, created, owned_by, root, parent, permission`
(`protocol.py:36-44`). It does not advertise levels.

---

## 10. Implications for MODEL-016 declarations

1. A local profile model needs a declared capability with four parts:
   - an effort list (strings)
   - an optional numeric-effort range (float 0.0..0.99 for SGLang, integer 1..100 for Strata deepseek41)
   - an optional token-budget range
   - an optional template toggle

   MODEL-009's `ReasoningLevel` is `Effort(name) | Budget(u64)` (`rust/providers/src/models/reasoning.rs:64-67`), and
   `ReasoningCapability` is `{efforts, budget, toggle}` (`:85-89`). Neither has a numeric-effort kind. A numeric
   effort is sent in `reasoning_effort`, not in a budget field.
2. The wire field for each part differs by engine:
   - effort: `reasoning_effort` on all engines except TGI. On TensorRT-LLM's generic path it has no effect.
   - budget: `thinking_token_budget` (vLLM, TensorRT-LLM), `reasoning_budget_tokens` (llama.cpp, Strata),
     `custom_params.thinking_budget` (SGLang). Ollama and LMDeploy have none.
   - toggle: `chat_template_kwargs.enable_thinking` or `.thinking`, depending on the model family; Ollama has no
     template kwargs.
3. Engines that refuse unknown fields: TensorRT-LLM (`extra="forbid"` plus the unused-kwargs guard). Engines that
   silently ignore them: TGI, Ollama, LMDeploy. So sending a field an engine doesn't take is either a 400 or a
   silent drop. Neither is acceptable without a declaration.
4. Runtime discovery is possible for llama.cpp (`/props`), Ollama (`/api/show`) and, per its docs, LM Studio
   (`/api/v1/models`, native vocabulary `off|on|low|medium|high`). Using it is a scope choice. The other six need a
   config declaration.

## 11. Engine facts bearing on MODEL-010's open questions (not answers)

- **No capability: refuse, or send unchecked?** Engine behaviour when an effort reaches a model that can't reason:
  - Ollama returns **400** "does not support thinking" on `/v1/chat/completions` (`ollama/server/routes.go:3218-3226`).
  - vLLM, SGLang, llama.cpp and LMDeploy pass the effort to the template. A template that doesn't read it ignores it
    silently.
  - TGI ignores the field silently.
  - TensorRT-LLM accepts the field but its generic path doesn't use it.
  - Strata always runs a thinking model (Qwen / DeepSeek family) and accepts `none|low|medium|high|xhigh|max`, so
    today's BUG-440 `reasoning_effort` from Off/Low/Medium/High is legal on Strata's default family. Strata remaps
    `high`→`xhigh` itself.
- **Should local-openai send fixed `chat_template_kwargs`?**
  - vLLM already derives `enable_thinking` from `reasoning_effort`, but never `thinking`.
  - SGLang already derives both `thinking` and `enable_thinking` from `reasoning_effort` (setdefault).
  - Strata reads only falsy `enable_thinking` and `reasoning_effort`, and derives the toggle from `reasoning_effort`.
  - llama.cpp maps `reasoning_effort: none` to `enable_thinking=false`, and forwards other kwargs to the template.
  - TensorRT-LLM accepts `enable_thinking`/`thinking` on every template, but refuses other unread kwargs.
  - Ollama ignores `chat_template_kwargs` silently.
  - TGI ignores them silently.
  - LMDeploy forwards them.

  So for the engines that read `reasoning_effort`, sending only `reasoning_effort` already flips the common toggle
  (vLLM `enable_thinking`; SGLang both keys). The exceptions are DeepSeek-V3.1 / Granite 3.2 on vLLM, which need
  `thinking`, and TensorRT-LLM's generic path, where `reasoning_effort` does nothing and only
  `chat_template_kwargs` work.
