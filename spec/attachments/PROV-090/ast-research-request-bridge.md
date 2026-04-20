# AST Research — PROV-090

## request_to_rhai (current signature)

- Location: `codelet/providers/src/custom/request_bridge.rs:55`
- Signature:
  ```rust
  pub fn request_to_rhai(
      messages: &[Message],
      tools: &[ToolDefinition],
  ) -> Result<Dynamic, CustomProviderError>
  ```
- Produces Rhai map: `#{ messages: [...], tools: [...] }`.

## invoke_build_request / invoke_build_stream_request (call sites)

- `codelet/providers/src/custom/provider.rs:157` — `invoke_build_request`, calls `request_to_rhai(messages, tools)`.
- `codelet/providers/src/custom/provider_stream.rs:23` — `invoke_build_stream_request`, same shape.
- Only callers of `invoke_build_request`: `CustomProvider::complete_with_tools` (same file, line 266).
- Only callers of `invoke_build_stream_request`: `CustomProvider::open_streaming` (same file, line 65).

## create_rig_agent signatures across providers

| Provider | Path | thinking_config param |
| --- | --- | --- |
| Claude | `codelet/providers/src/claude.rs:507` | `thinking_config: Option<serde_json::Value>` (present) |
| OpenAI | `codelet/providers/src/openai.rs:410` | absent / different reasoning flow |
| Codex | `codelet/providers/src/codex/mod.rs:331` | reasoning via separate channel |
| Copilot | `codelet/providers/src/copilot/rig_agent.rs:56` | absent |
| Gemini | `codelet/providers/src/gemini.rs:130` | absent |
| Zai | `codelet/providers/src/zai.rs:218` | absent |
| Custom | `codelet/providers/src/custom/custom_provider.rs:75` | **target — add parameter here** |

The reference shape we mirror is Claude's:
```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<ClaudeCompletionModel>
```

For `CustomProvider::create_rig_agent` we add `thinking_config: Option<serde_json::Value>` as the final parameter after the existing `(project_root, name, model_alias, session_id, preamble)`.

## Conversion helper

`codelet/providers/src/custom/conversion.rs::json_value_to_dynamic` already maps `serde_json::Value::Null` to Rhai `Dynamic::UNIT`. That means when we receive `Option<&serde_json::Value>`:

- `Some(v)`  → `json_value_to_dynamic(v)` (scalar/map/array/unit as appropriate)
- `None`     → `Dynamic::UNIT` (equivalent to what the script would see for `Value::Null`)

## Test entry points

- Unit tests over `request_to_rhai` live in `codelet/providers/src/custom/request_bridge.rs` (via `#[cfg(test)]`) and cross-module tests use the public function directly.
- Integration tests touching `invoke_build_request`/`invoke_build_stream_request`: `codelet/providers/tests/custom_http_lifecycle_tests.rs`, `codelet/providers/tests/custom_streaming_sse_bridge_tests.rs`.

## Impact summary

- Extend `request_to_rhai` to accept `Option<&serde_json::Value>` as a 3rd parameter.
- Insert `thinking_config` key into the outer map unconditionally (unit when None).
- Update both `invoke_build_request` and `invoke_build_stream_request` to accept & forward the option.
- Update `CustomProvider::create_rig_agent` to accept `Option<serde_json::Value>` matching Claude.
- Update existing tests in `custom_http_lifecycle_tests.rs` and `custom_streaming_sse_bridge_tests.rs` that call the two invoke functions with the old 2-arg shape.
