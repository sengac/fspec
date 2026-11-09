# MODEL-014: the Codex /models endpoint as the source of Codex thinking levels

Date: 2026-11-06. Sources:
- fspec code, `rust/<path>:<line>` relative to the repo root.
- The openai/codex checkout at `/tmp/codex` (`main` @ `0ed9d6fa4`, 2026-11-03). Paths are `codex-rs/<path>:<line>`.
- A live response the Codex CLI cached on this machine, redacted to the reasoning fields: `codex-models-response-capture.json` (this folder).

## 1. The request

- `GET {base}/models?client_version=<x.y.z>`, with base `https://chatgpt.com/backend-api/codex` (`codex-rs/codex-api/src/endpoint/models.rs:33-45`). fspec already uses the same base for responses (`rust/providers/src/catalog.rs:162`, `rust/providers/src/codex/codex_oauth.rs:29`).
- ChatGPT auth means a Bearer access token plus the `ChatGPT-Account-Id` header. fspec's `RefreshingCodexClient` adds both, plus `originator: rust`, to every OAuth request, and refreshes an expired token first (`rust/providers/src/codex/refreshing_client.rs:164-190`, `client_http.rs:36-45`).
- Codex in API-key mode (`CodexProvider::from_api_key`, the legacy token exchange) has no ChatGPT token, so it cannot call this endpoint (`rust/providers/src/codex/constructors.rs:21-70`).
- `client_version` is the Codex CLI's own crate version, `MAJOR.MINOR.PATCH` (`codex-rs/models-manager/src/lib.rs:24-31`). The capture was fetched with `0.165.0`.
- Models carry `minimal_client_version` in Codex's bundled catalog. Examples: gpt-6-astra 0.153.0, gpt-6-sol 0.155.0, gpt-5.5 0.124.0, codex-auto-review 0.98.0 (`codex-rs/models-manager/models.json`). The live response has no such field, which suggests the server filters by `client_version` itself (**inference**). fspec's own version is 0.10.9 (`rust/Cargo.toml:48`).
- The catalog can differ by `originator`: Codex keeps separate catalogs for `codex_work_cca`, `chatgpt_cca` and others (`codex-rs/models-manager/src/originator.rs:1-30`). fspec sends `originator: rust`.

## 2. The response (`{"models": [ModelInfo]}`)

`ModelInfo` is at `codex-rs/protocol/src/openai_models.rs:426-520`. These are its reasoning fields:
- `slug`: the model id, e.g. `gpt-6.1-sol`.
- `supported_reasoning_levels: [{effort, description}]`: an open string per effort (`openai_models.rs:71-170`). Unknown names are `Custom(String)`.
- `default_reasoning_level: effort | null`.
- `multi_agent_reasoning_effort: effort | null`.
- `supports_reasoning_summary_parameter` (defaults to true) and `default_reasoning_summary` (`none` in the capture).
- `support_verbosity`, `default_verbosity`, `service_tiers [{id, name, description}]` and `supports_reasoning_effort_updates`.

The capture (2026-11-05, client_version 0.165.0) lists 10 models:

| slug | default | supported_reasoning_levels |
|---|---|---|
| gpt-6.1-sol, gpt-6.1-terra, gpt-5.6-sol | low | low, medium, high, xhigh, max, ultra |
| gpt-6-astra, gpt-6-sol, gpt-5.6-terra | medium | low, medium, high, xhigh, max, ultra |
| gpt-6-luna, gpt-5.6-luna | medium | low, medium, high, xhigh, max |
| gpt-reserve, codex-auto-review (visibility hide) | medium | low, medium, high, xhigh, max |

- No model lists `none` or `minimal`.
- The `ultra` description is "Maximum reasoning with automatic task delegation".
- gpt-5.5 and most models.dev OpenAI ids (gpt-4o, gpt-5-mini, ...) are not in the response. fspec's Codex section today lists the whole models.dev OpenAI catalog (`rust/sessions/src/cloud_models.rs:179-181`).

## 3. How Codex itself uses it

- **Sending.** `build_reasoning` sends `effort.or_else(default_reasoning_level)` through `resolve_reasoning_effort` (`codex-rs/core/src/client.rs:937-957`). It sends `summary` only when the model supports the summary parameter and the summary is not `none`.
- **Ultra.** `ultra` is never sent. It resolves to `multi_agent_reasoning_effort` when that level is supported, else `max`, else the highest non-ultra level (`codex-rs/protocol/src/openai_models/reasoning_effort.rs:10-35`). It also turns on proactive multi-agent delegation (`codex-rs/core/src/session/multi_agents.rs:96`).
- **Persistent.** `persistent` goes over the wire as `disabled` (research §3).
- **Cache.** Codex caches the response in `~/.codex/models_cache.json` with `fetched_at`, `etag` and `client_version`. The TTL is 300 s (`codex-rs/models-manager/src/manager.rs:42`, `cache.rs:98-107`). Codex refreshes when a response carries a new models etag (`manager.rs:309`, `661-704`).
- **Seed.** Before any fetch, the manager starts from the bundled `models.json` (`manager.rs:449-460`) and the disk cache. fspec's no-fallback rule forbids anything equivalent.

## 4. fspec today

- **Dialog and `/thinking`.** For a Codex model, `ThinkingTarget::for_provider_manager` resolves the capability from the models.dev **OpenAI** entry's `reasoning_options`. `selection_metadata` maps `codex` to `openai` (`rust/providers/src/thinking_wire.rs:150-179`, `rust/providers/src/selection_metadata.rs:19`). So gpt-5.5 via Codex offers models.dev's `none`, although gpt-5.5 is not in the captured `/models` response at all. gpt-6.1-sol offers models.dev's `[low..max]` and misses `ultra`.
- **Request-option snapshot.** `CodexProvider::set_capabilities` has no production caller. `ProviderManager::get_codex` builds a new `CodexProvider` for each turn, compaction and deep search (`rust/providers/src/manager/impl_3.rs:201-224`), and that is the one place to set the snapshot.
- **Snapshot semantics.** `CodexCapabilities` (PROV-152, `rust/providers/src/codex/request_options/types.rs:127-137`) treats an empty list as "nothing supported". The MODEL-010 Codex form always sends `reasoning.summary: "auto"` (`rust/providers/src/thinking_wire/forms.rs:110`). `/models` lists efforts and service tiers but no summary, context or verbosity values, so a snapshot built from it with today's semantics would refuse `summary: "auto"` on every turn. Controls `/models` does not list need to be "not advertised" (unvalidated, BUG-441) rather than "none supported".
- **Advertised defaults.** `resolve` applies the snapshot's advertised defaults when the request names no value (`rust/providers/src/codex/request_options.rs:16-30`). A snapshot whose `defaults.reasoning.effort` is `default_reasoning_level` sends that effort when the session's level is not set. This is what Codex itself does.
- **Async homes.** Model resolution is async in `resolve_provider_manager_and_limits` (session create, resume, set_model through block_in_place; `rust/sessions/src/model_resolution.rs:269-277`, `rust/sessions/src/handle_impl/handle_impl_6.rs:44-48`). `ThinkingTarget::for_provider_manager` is synchronous and reads only ProviderManager state. MODEL-016 stores the profile declaration the same way (`rust/providers/src/manager/profile_reasoning.rs`).
