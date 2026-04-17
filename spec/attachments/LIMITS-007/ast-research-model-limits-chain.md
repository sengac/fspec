# AST Research: Model Limits Resolution Chain

## resolve_model_limits (pure function)
- **File**: `codelet/providers/src/model_limits.rs:77`
- **Signature**: `pub fn resolve_model_limits(registry_value, user_override, provider_max, provider_default) -> usize`
- Priority chain: user_override → clamp by provider_max → registry_value → clamp by provider_max → provider_default

## resolve_context_window / resolve_max_output_tokens (convenience wrappers)
- **File**: `codelet/providers/src/model_limits.rs:106,123`
- Delegate to `resolve_model_limits` using resolver trait methods

## ProviderManager::context_window() / max_output_tokens()
- **File**: `codelet/providers/src/manager.rs:800,813`
- Builds a `ConstantResolver` stub per provider, delegates to resolve_context_window/resolve_max_output_tokens
- Uses `provider_limits_resolver()` which maps each ProviderType to its constants

## Provider Constants
| Provider | CONTEXT_WINDOW | MAX_OUTPUT_TOKENS | max_ctx clamp | max_out clamp |
|----------|---------------|-------------------|---------------|---------------|
| Claude   | 200,000       | 8,192             | Some(200k)    | Some(8192)    |
| OpenAI   | 128,000       | 4,096             | None          | None          |
| Gemini   | 1,000,000     | 8,192             | None          | None          |
| Codex    | 272,000       | 4,096             | None          | None          |
| Z.AI     | 128,000       | 8,192             | None          | None          |
| Copilot  | 200,000       | 4,096             | None          | None          |

## ProviderManager::for_testing()
- **File**: `codelet/providers/src/manager.rs:900`
- Test-only constructor, no credentials needed
- Stores context_window/max_output_tokens as registry values

## ProviderManager::override_model_limits()
- **File**: `codelet/providers/src/manager.rs:827`
- Sets user_context_window / user_max_output_tokens for NAPI overrides

## ProviderManager::raw_model_context_window()
- **File**: `codelet/providers/src/manager.rs:847`
- Returns clamped context_window for sub-agent propagation
