# BUG-547 research: what context window / max-output limits each self-hosted engine reports over HTTP

Date: 2026-11-06. The format follows `spec/attachments/MODEL-016/local-engine-reasoning-research.md`. Each engine
with public source was cloned with `git clone --depth 1` into `/tmp/asst/work/BUG-547/src/<engine>/repo` and read
from source. No code was run. References are `<path>:<line>` relative to that repo, at the commit below. Anything
not read from source is marked **DOCS** (public documentation) or **INFERENCE**.

| Engine | Repository | Commit (date) |
|---|---|---|
| vLLM | vllm-project/vllm | `a7809d0` (2026-11-06) |
| SGLang | sgl-project/sglang | `3173a99` (2026-11-06) |
| Strata | Niko1221/Strata | `9f7d977` (2026-11-05) |
| llama.cpp server | ggml-org/llama.cpp | `e14a779` (2026-11-06) |
| Ollama | ollama/ollama | `6362e0a` (2026-11-05) |
| LM Studio | closed-source server; lmstudio-ai/lmstudio-js (SDK only) | `4b9af90` (2026-10-28) |
| Hugging Face TGI | huggingface/text-generation-inference | `b4adbf2` (2026-03-21) |
| TensorRT-LLM (trtllm-serve) | NVIDIA/TensorRT-LLM | `d7822295` (2026-11-05); only `tensorrt_llm/serve` was checked out (the full checkout failed) |
| LMDeploy | InternLM/lmdeploy | `bc2c8da` (2026-11-05) |

`<root>` = the base URL without `/v1`, which is how fspec's `live_check.rs` builds URLs.

---

## 0. Summary

| Engine | Endpoint | Context field | Max-output field | Matching | Notes |
|---|---|---|---|---|---|
| vLLM | `GET <root>/v1/models` | `data[i].max_model_len` (int) | **none** | `data[i].id` == served model name | Running value (`--max-model-len`, resolved). `null` on LoRA cards. A server-wide `max_new_tokens` cap from generation_config is not reported. |
| SGLang | `GET <root>/v1/models` (or `GET /v1/models/{model}`) | `data[i].max_model_len` (int) | **none** | `data[i].id` == `served_model_name` | `model_config.context_len` (`--context-length` or derived). `null` on LoRA cards. `GET /server_info` also has `max_req_input_len` (KV-capped input limit). |
| Strata | `GET <root>/props` (also `/v1/models`, `/health`, `/slots`) | `default_generation_settings.n_ctx`; `/v1/models` → `data[i].meta.n_ctx`; `/health` → `max_context` | `default_generation_settings.params.n_predict` is a **default**, not a cap (-1 = unset) | single model plus aliases; `/props?model=` must be a name or alias, else 404 | Engine's READY context. Can be 0 while starting. llama.cpp-shaped JSON. |
| llama.cpp | `GET <root>/props` (also `/v1/models`) | `default_generation_settings.n_ctx` (per-slot); `/v1/models` → `data[0].meta.n_ctx` plus `meta.n_ctx_train` | **none usable** (`params.n_predict` in `/props` is always -1) | single model (`model_alias`, `data[0].id`, `aliases`) | Running per-slot context, capped at n_ctx_train. Router mode: `/props` without `?model=` gives `n_ctx: 0`, and `/v1/models` gives `context_length` = **training** ctx. |
| Ollama | `GET <root>/api/ps` | `models[i].context_length` (running num_ctx per slot) | **none** (`num_predict` only appears as text inside `parameters` when a Modelfile sets it) | `models[i].name`/`model` (shortest display name) | Only lists **loaded** models. `POST /api/show` `model_info["<arch>.context_length"]` is the **training** context. `/v1/models` has no limits. |
| LM Studio | `GET <root>/api/v1/models` (**DOCS**) | `models[i].loaded_instances[j].config.context_length` (running); `models[i].max_context_length` (model max, may be null) | **none** (**DOCS**) | `models[i].key` or `loaded_instances[j].id` (the same matching as fspec's `lm_studio_options`) | Legacy `GET /api/v0/models` → `data[i].max_context_length` (**DOCS**). |
| TGI | `GET <root>/info` | `max_total_tokens` (input + output); also `max_input_tokens` | no separate field; output ≤ `max_total_tokens - input_len` | single model (`model_id`) | Running router config. `/v1/models` has no limits. |
| TensorRT-LLM | — | **none** | **none** | `/v1/models` gives `id` only | `/server_info` has `max_batch_size` and `tokens_per_block`, but no `max_seq_len`. |
| LMDeploy | — | **none** | **none** | `/v1/models` gives `id`/`root` only | `session_len` is internal only (it sets Prometheus bucket sizes only). |

Key cross-engine facts:
- **No engine reports a hard max-output cap.** Output is always bounded by context minus prompt. The only
  server-side output knobs (vLLM generation_config `max_new_tokens`, llama.cpp `-n`, Strata shared `max_tokens`,
  Ollama `num_predict`) are either unreported or reported only as a request **default**.
- Three engines report a **running** context in llama.cpp-style JSON: llama.cpp, Strata (`n_ctx`) and,
  via OpenAI `/v1/models`, vLLM/SGLang (`max_model_len`). Ollama's running value is only in `/api/ps`, and only
  while the model is loaded. `/api/show` gives the training value, which can be far larger than the running num_ctx.
- TensorRT-LLM and LMDeploy report nothing usable. fspec must use a declared value for them.
- 0 / null mean "unknown", not "unlimited": llama.cpp router `n_ctx: 0`, Strata `n_ctx: 0` while starting, Ollama
  `context_length: 0` for speech-only models, LM Studio `max_context_length: null`, vLLM/SGLang LoRA
  `max_model_len: null`.

---

## 1. vLLM

- `ModelCard.max_model_len: int | None = None` — `vllm/entrypoints/serve/engine/protocol.py:105-113` (field at `:112`).
- `GET /v1/models` — `vllm/entrypoints/openai/models/api_router.py:20-24` → `OpenAIServingModels.show_available_models`
  (`vllm/entrypoints/openai/models/serving.py:149-165`) → `OpenAIModelRegistry.show_available_models` (`:64-77`).
  This emits one card per served name with `id=base_model.name` and `max_model_len=self.model_config.max_model_len`
  (`:66-71`). So every `--served-model-name` alias gets the same value.
- LoRA adapter cards are built without `max_model_len`, so it is `null`; `parent` names the base model
  (`serving.py:153-162`).
- Units: tokens, covering prompt plus output (`vllm/config/model.py:256-268`: "Model context length (prompt and output)").
  `--max-model-len -1`/`auto` picks the largest length that fits in GPU memory. It is resolved at startup
  (`model.py:837-838`, `get_and_verify_max_len` `:2177-2201`) and validated as a positive int (`:1084-1088`). So for a
  base model the reported value is the **running** limit, never -1.
- Max output: not reported. Per request it is `min(max_model_len - input_len, request max_tokens or
  default_sampling_params["max_tokens"], override, platform cap)` (`vllm/entrypoints/serve/utils/api_utils.py:234-270`).
  generation_config `max_new_tokens` "sets a server-wide limit on the number of output tokens"
  (`model.py:365-371`), but no public endpoint exposes it.
- `GET /server_info` (it dumps the full `vllm_config`) exists only when `VLLM_SERVER_DEV_MODE` is set
  (`vllm/entrypoints/launchers/api_server/routers.py:46-49`, `vllm/entrypoints/serve/dev/server_info/api_router.py:43-53`).
  It can't be relied on.

## 2. SGLang

- `ModelCard.max_model_len: Optional[int] = None` — `python/sglang/srt/entrypoints/openai/protocol.py:87-96`.
- `GET /v1/models` — `python/sglang/srt/entrypoints/http_server.py:1991-2020`. The base card has
  `id=served_model_name`, `max_model_len=tokenizer_manager.model_config.context_len` (`:2003`). LoRA cards have
  `max_model_len=None` (`:2016`).
- `GET /v1/models/{model}` — `:2023-2045`. It returns 404 unless `model` == `served_model_name`, else the same card (`:2044`).
- The Rust frontend serializes the same field (`rust/sglang-server/src/api_server/http/openai/models.rs:50-58`).
- How `context_len` is set: it is `--context-length` if given, else derived from the HF config
  (`python/sglang/srt/configs/model_config.py:1393-1435`). The flag is documented as "The model's maximum context length.
  Defaults to None (will use the value from the model's config.json instead)"
  (`python/sglang/srt/arg_groups/fields/model.py:110-117`). A larger-than-derived value raises unless an env override
  is set (`model_config.py:1405-1425`). It is the configured value and is **not** capped by KV pool size.
- `GET /server_info` (`http_server.py:859-893`) merges `server_args.resolved_dict()` and `scheduler_info`
  (`python/sglang/srt/utils/common.py:3600-3610`). It includes `max_req_input_len`
  = `min(context_len - 1, kv_capacity - 1) - 5` (`python/sglang/srt/managers/tp_worker.py:569-573`), and also
  `max_total_num_tokens` and `context_length` (the server arg; **INFERENCE**: `null` when it was not passed).
- Max output: none. `GET /model_info` returns `preferred_sampling_params` (`http_server.py:827`), an operator
  default merged under request params (`python/sglang/srt/managers/tokenizer_manager.py:1607-1608`). It may contain
  `max_new_tokens` if the operator set it. It is a default, not a cap.

## 3. Strata

The server is a single Python file, `serve/server.py`. Its header lists `GET /v1/models, GET /models, GET /props,
GET /slots, GET /health` (`:7`).
- Every endpoint reports `svc.reported_ctx()` (`:5092-5096`): `engine.max_context`, or `known_ctx` while the engine
  restarts, or `0`. "max_context is 0 until READY". `max_context` is parsed from the engine's `READY <n>` line
  (`:1240-1242`). The engine reads `--max-context` (`src/dsv41/ds4_main.cpp:2523`). This is the **running** value.
- `GET /props` → `_props` (`:8378-8403`). It takes an optional `?model=` that must be in `model_names()` (= model + aliases,
  `:5083-5084`), else 404, and returns 503 if the engine is not running. Response:
  `default_generation_settings.n_ctx` = reported ctx, and `default_generation_settings.params.n_predict` =
  `svc.shared.get("max_tokens", -1)` (`:8391-8392`). `svc.shared` is "the web app's Chat settings for every client" (`:4777`),
  applied only when a request sends no `max_tokens`/`max_completion_tokens` (`:5545-5546`). So it is a **default**, and
  -1 means none.
- `GET /v1/models` (`:8111-8130`): `data[i].meta.n_ctx`. Each alias is listed with `alias_of`. `data` is `[]` when the
  model is not loaded and has no idle-unload; an image-gen model entry has no `meta`.
- `GET /health` (`:8079-8083`): `max_context` (no auth). `GET /slots`: `[i].n_ctx` (`:8132-8150`).
- Units: tokens. Requests with `max_tokens` 0 or -1 get "the rest" of the context (`:8596`, `:5962-5972`).

## 4. llama.cpp server

- Routes: `GET /props`, `GET /models`, `GET /v1/models` (`tools/server/server.cpp:174-177`).
- `GET /props` → `get_res_props` (`tools/server/server-context.cpp:5681-5730`):
  `default_generation_settings.n_ctx = meta.slot_n_ctx` (`:5689`), plus `total_slots = params.n_parallel` (`:5697`)
  and `model_alias` (`:5698`).
- `slot_n_ctx = n_ctx_slot()` (`:5267`). `n_ctx_slot()` = `llama_n_ctx_seq`, capped by `--kv-unified-per-slot` and
  then by `n_ctx_train` (`:5081-5090`). So it is the **running per-slot** context, never above training context.
  (**INFERENCE**: without `--kv-unified`, `n_ctx_seq` = `-c` / `-np`.)
- `GET /v1/models` → `get_res_models` / `get_res_model_info` (`:5621-5679`): `data[0].meta.n_ctx` (slot ctx, `:5639`) and
  `data[0].meta.n_ctx_train` (`:5640`). `data[0].id` is the model name, with `aliases`.
- Max output: `/props` `default_generation_settings.params.n_predict` is always -1. `get_res_props` copies only
  `params.sampling` into a fresh `task_params` (`:5685-5686`), whose `n_predict` defaults to -1
  (`tools/server/server-task.h:86`). The server's `-n` is a per-request **default** (`tools/server/server-schema.cpp:497`;
  the request field `n_predict` has aliases `max_tokens`/`max_completion_tokens`, limits -1..INT32_MAX, `:46-50`). It is
  not a cap, and it is not reported.
- **Router mode** (`llama-server` with multiple models): `GET /props` without `?model=` returns a stub with
  `default_generation_settings.n_ctx: 0` (`tools/server/server-models.cpp:2021-2040`). With `?model=<name>` it proxies to
  the child instance (`:2047-2058`). `GET /v1/models` in router mode sets `context_length = n_ctx_train` only when > 0
  (`:2182-2184`). That is the **training** context, not the running one.

## 5. Ollama

- `GET /api/ps` (`server/routes.go:2426`, handler `:2813-2870`): `models[i].context_length`
  (`api/types.go:945-955`), filled from the runner's `ContextLength()` (`server/sched.go:1826-1827`). For the llama.cpp runner
  that is `options.NumCtx` (`llm/llama_server.go:360-362`), the **per-slot** value: llama-server is launched with
  `-c NumCtx*numParallel -np numParallel` (`:402`, `:410`). MLX runner: `mlxrunner/client.go:285-294`.
  `NumCtx` is capped to the GGUF training context (`llm/server.go:110-113`, `server/sched.go:204-206`).
  Speech-only models report `0` (`routes.go:2843-2847`). Only **loaded** models are listed. Matching is by
  `name`/`model` = `model.ParseName(m.ShortName).DisplayShortest()` (`:2818`).
- `POST /api/show {model}` (`routes.go:2405`): `model_info` is the GGUF KV map (`:2098-2101`), so
  `model_info["<general.architecture>.context_length"]` is the **training** context. For safetensors/remote models it
  comes from the manifest/config (`:1979-1981`, `:2036-2040`). `parameters` is a text blob of Modelfile options (`:1989-2001`)
  and may contain `num_ctx`/`num_predict` lines only if the Modelfile sets them.
- Defaults: `NumPredict: -1` (unlimited), `NumCtx: envconfig.ContextLength()` (`api/types.go:1223`, `:1239`).
  `OLLAMA_CONTEXT_LENGTH` defaults to 0 = "4k/32k/256k based on VRAM" (`envconfig/config.go:230`, `:338`).
- OpenAI `GET /v1/models`: `{id, object, created, owned_by}` only (`openai/openai.go:219-224`).

## 6. LM Studio (closed-source server)

The SDK types (`packages/lms-shared-types/src/llm/LLMModelInfo.ts:15-44`) define `maxContextLength` (model) and
`contextLength` (loaded instance). The SDK uses LM Studio's own websocket protocol, not the REST API. The REST shape is
**DOCS**:
- `GET /api/v1/models` (https://lmstudio.ai/docs/developer/rest/list): `models[i].key`;
  `models[i].max_context_length: number | null` ("Maximum context length supported by the model… May be null for
  decision models when unavailable"); `models[i].loaded_instances[j].id`;
  `models[i].loaded_instances[j].config.context_length` ("The maximum context length for the model in number of tokens"),
  which is the **running** value. The example shows Gemma `context_length` 4096 vs `max_context_length` 262144. There is
  no max-output field. Unloaded models have no `loaded_instances` entry.
- Legacy `GET /api/v0/models` and `/api/v0/models/{model}` (https://lmstudio.ai/docs/developer/rest/endpoints):
  `data[i].{id, state, max_context_length}`. `loaded_context_length` does not appear in the current docs.
- Matching: fspec's `lm_studio_options` already matches `key` or a `loaded_instances[].id`
  (`rust/providers/src/local_reasoning/live_check.rs:236-262`). The same lookup works for context.

## 7. Hugging Face TGI

- `GET /info` (`router/src/server.rs:155-163`, route `:2240`) returns `Info` (`router/src/lib.rs:261-298`):
  `model_id`, `max_input_tokens` (`:282`) and `max_total_tokens` (`:284`), all non-optional `usize`. This is the running
  router config.
- `max_total_tokens` is the context window, covering prompt and output: "users can send either a prompt of `1000` and
  ask for `512` new tokens…" (`launcher/src/main.rs:686-695`). The default is `min(max_allocatable, max_position_embeddings)`.
  `max_input_tokens` defaults to that minus 1 (`:676-679`).
- Output: when a request omits `max_new_tokens`, it gets `max_total_tokens - input_length`. Requests over the total are
  rejected (`router/src/validation.rs:152-179`), and inputs over `max_input_tokens` are rejected (`:182`). There is no
  separate output cap.
- `GET /v1/models` (`server.rs:166-185`): `{id: model_id, object, created: 0, owned_by}` only. TGI serves a single
  model, so the request `model` is not used for matching.

## 8. TensorRT-LLM (trtllm-serve)

- `ModelCard` = `{id, object, created, owned_by}` (`tensorrt_llm/serve/openai_protocol.py:254-258`). `GET /v1/models`
  → `ModelList(data=[ModelCard(id=self.model)])` (`tensorrt_llm/serve/openai_server.py:1231`, `:1788-1790`).
- `GET /server_info` (`:1343-1345`, `:3844-3886`): `disaggregated_params`, `render_fingerprint`, `max_batch_size`,
  `kv_cache_hash_algo`, `tokens_per_block` and `startup_metrics`. There is **no** `max_seq_len`/`max_input_len`.
- The other routes (`/health`, `/version`, `/metrics`, `/memory_status`, …) report no limits either. Nothing usable.

## 9. LMDeploy

- `ModelCard` = `{id, object, created, owned_by, root, parent, permission}` (`lmdeploy/serve/openai/protocol.py:36-44`).
  `GET /v1/models` lists `async_engine.model_name` plus adapters (`lmdeploy/serve/openai/endpoints/models.py:16-25`,
  `lmdeploy/serve/openai/utils.py:16-21`).
- The context is `session_len` (`--session-len`, or derived from the HF config; `lmdeploy/serve/core/async_engine.py:146-148`).
  When a request omits `max_new_tokens`, it defaults to `session_len - len(input_ids)` (`:423`). Inputs at or over
  `session_len` are rejected (`:550-553`). Neither value is reported. `/distserve/engine_info` returns only
  parallelism and block counts (`lmdeploy/serve/openai/endpoints/distserve.py:19-33`). `session_len` reaches
  `/metrics` only as histogram bucket sizes (`lmdeploy/metrics/loggers.py:136`, `:286-300`). Nothing usable.
