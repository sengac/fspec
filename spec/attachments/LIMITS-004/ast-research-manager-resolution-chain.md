# AST Research: ProviderManager Resolution Chain (LIMITS-004)

## Key methods to refactor in manager.rs

### context_window() — Line 684
Returns raw `model_context_window` bypassing provider's ModelLimitsResolver.
Must be changed to resolve through `resolve_context_window()`.

### max_output_tokens() — Line 715
Same pattern as context_window() — returns raw value, must use resolver.

### override_model_limits() — Line 745
Currently stores directly into `model_context_window`/`model_max_output_tokens`.
Must split into user_context_window/user_max_output_tokens fields.

### raw_model_context_window() — Line 762
Used by sub-agent propagation (DeepSearch, AgentManager). Currently returns raw
unclamped value. Must return clamped value by calling context_window().

### raw_model_max_output_tokens() — Line 770
Same — must return clamped value.

### provider_constant_context_window() — Line 693
Will be removed — replaced by ModelLimitsResolver.

### provider_constant_max_output_tokens() — Line 724
Will be removed — replaced by ModelLimitsResolver.

## Provider ModelLimitsResolver implementations found

- ClaudeProvider: max_context_window=Some(200_000), max_output_tokens_limit=Some(8_192)
- OpenAIProvider: trusts registry (None/None), defaults from env vars or 128k/4096
- GeminiProvider: trusts registry (None/None), defaults 1M/8192
- CodexProvider: trusts registry (None/None), defaults 272k/4096, suppresses max_output_tokens
- ZAIProvider: trusts registry (None/None), defaults 128k/8192
- CopilotProvider: trusts registry (None/None), defaults 200k/4096

## Callers of raw_model_context_window / raw_model_max_output_tokens

- session_manager.rs:4955-4956 (DeepSearch handler)
- session_manager.rs:4988-4989 (AgentManager handler)

## Callers of override_model_limits

- session_manager.rs:6696 (NAPI model selection)
- agent_manager_handler.rs:181 (subordinate agent propagation)

## struct fields to split

Current:
- `model_context_window: Option<usize>` — conflates registry + user override
- `model_max_output_tokens: Option<usize>` — conflates registry + user override

New:
- `registry_context_window: Option<usize>` — from models.dev (select_model)
- `user_context_window: Option<usize>` — from NAPI override
- `registry_max_output_tokens: Option<usize>` — from models.dev
- `user_max_output_tokens: Option<usize>` — from NAPI override
