# REFAC-031 design: split rust/providers/src/openai.rs under 300 lines

## Problem

PROV-171 must change how `OpenAIProvider` reads `OPENAI_CONTEXT_WINDOW` /
`OPENAI_MAX_OUTPUT_TOKENS` and drop its guessed `DEFAULT_*` constants. AGENTS.md caps
files at 300 lines and `rust/providers/src/openai.rs` had 631 (HEAD before this card):

| Item | Lines |
|---|---|
| module docs, imports, `mod discovery` | 1-23 |
| `DEFAULT_CONTEXT_WINDOW`, `DEFAULT_MAX_OUTPUT_TOKENS`, `ANTI_RAMBLE_*`, `anti_ramble_additional_params` | 25-67 |
| `pub const CONTEXT_WINDOW` / `MAX_OUTPUT_TOKENS`, `normalize_base_url` | 69-90 |
| `struct OpenAIProvider`, `Debug`, `ProviderAdapter` | 92-130 |
| `impl ModelLimitsResolver for OpenAIProvider` | 132-158 |
| `impl OpenAIProvider` (constructors :170-311, `base_url`/`is_local_endpoint` :317-326, `list_local_models*` :347-368, `client` :377, `create_rig_agent` :401-474, `rig_response_to_completion` :477-505) | 160-506 |
| `impl LlmProvider for OpenAIProvider` | 508-579 |
| `#[cfg(test)] mod tests` | 581-631 |

## Chosen split

`openai.rs` stays the module root (so `codelet_providers::openai::{OpenAIProvider,
CONTEXT_WINDOW, MAX_OUTPUT_TOKENS}` and `codelet_providers::OpenAIProvider` are
unchanged) and keeps docs, constants, `anti_ramble_additional_params`,
`normalize_base_url`, the struct, `Debug` and `ProviderAdapter`. Child file modules
(children see the parent's private items and the struct's private fields, so no
visibility changes):

- `openai/construction.rs`: `new`, `from_api_key`, `from_api_key_with_session`,
  `with_max_output_tokens`, `from_api_key_with_options`, `base_url`, `is_local_endpoint`.
- `openai/agent.rs`: `list_local_models`, `list_local_models_with_auth`, `client`,
  `create_rig_agent` (uses the parent's private `discovery` module).
- `openai/llm_provider.rs`: `ModelLimitsResolver` impl, the private
  `rig_response_to_completion` (kept in the same module as its only caller
  `complete_with_tools`, so it stays private), and the `LlmProvider` impl.
- `openai/tests.rs`: the unit tests verbatim; it imports `crate::LlmProvider`
  explicitly because the parent no longer imports that trait.

`openai_discovery.rs` keeps its `#[path]` include. Each child imports only what it
uses. Coverage mappings into the moved lines are re-pointed by item.

## Alternatives rejected

- `include!` fragments of one `impl` block: unnecessary — the items split cleanly at
  impl boundaries and real modules are rustfmt/clippy-visible.
- Making `rig_response_to_completion` `pub(super)` and keeping it with the agent code:
  needless visibility change.
- Removing the guessed constants now: behaviour change, belongs to PROV-171.

## API changes

None. No type, field, signature, visibility or behaviour change.
