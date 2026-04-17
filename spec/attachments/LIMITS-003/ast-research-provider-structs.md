# AST Research: ModelLimitsResolver Trait Implementation Targets

## Trait Definition

- `ModelLimitsResolver` trait defined at `codelet/providers/src/model_limits.rs:27`
- Methods: `max_context_window()`, `max_output_tokens_limit()`, `default_context_window()`, `default_max_output_tokens()`, `should_send_max_output_tokens()`
- Existing test doubles: `ClampingResolver`, `MinimalResolver` (in same file's `#[cfg(test)]` module)
- No production `impl ModelLimitsResolver for ...` blocks exist yet — only test doubles

## Provider Structs (Implementation Targets)

| Provider | Struct | Location |
|----------|--------|----------|
| Claude | `ClaudeProvider` | `codelet/providers/src/claude.rs:163` |
| OpenAI | `OpenAIProvider` | `codelet/providers/src/openai.rs:60` |
| Gemini | `GeminiProvider` | `codelet/providers/src/gemini.rs:27` |
| Codex | `CodexProvider` | `codelet/providers/src/codex/mod.rs:62` |
| Z.AI | `ZAIProvider` | `codelet/providers/src/zai.rs:37` |
| Copilot | `CopilotProvider` | `codelet/providers/src/copilot/provider.rs:52` |

## Existing Constants (Reusable)

| Provider | Context Window Const | Max Output Const |
|----------|---------------------|-----------------|
| Claude | `CONTEXT_WINDOW = 200_000` (claude.rs:42) | `MAX_OUTPUT_TOKENS = 8192` (claude.rs:45) |
| OpenAI | `DEFAULT_CONTEXT_WINDOW = 128_000` (openai.rs:24) | `DEFAULT_MAX_OUTPUT_TOKENS = 4096` (openai.rs:28) |
| Gemini | `CONTEXT_WINDOW = 1_000_000` (gemini.rs:20) | `MAX_OUTPUT_TOKENS = 8192` (gemini.rs:23) |
| Codex | `CONTEXT_WINDOW = 272_000` (codex/mod.rs:42) | `MAX_OUTPUT_TOKENS = 4096` (codex/mod.rs:45) |
| Z.AI | `CONTEXT_WINDOW = 128_000` (zai.rs:30) | `MAX_OUTPUT_TOKENS = 8192` (zai.rs:33) |
| Copilot | `CONTEXT_WINDOW = 200_000` (copilot/mod.rs:64) | `MAX_OUTPUT_TOKENS = 4_096` (copilot/mod.rs:72) |

## OpenAI Env Var Reading (Already Exists in Constructor)

OpenAI reads env vars at construction time:
- `OPENAI_CONTEXT_WINDOW` → `openai.rs:167-169`
- `OPENAI_MAX_OUTPUT_TOKENS` → `openai.rs:172-174`

The `default_context_window()` and `default_max_output_tokens()` on the resolver should use the same env vars.
