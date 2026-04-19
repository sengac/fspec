# AST Research: PROV-080 Opus 4.7 Thinking Display

## Key Finding

The adaptive thinking JSON in `thinking_config.rs:251` currently returns:
```json
{"thinking": {"type": "adaptive"}}
```

This needs `"display": "summarized"` added. The change is in:
- `codelet/tools/src/facade/thinking_config.rs` line 251 — `ClaudeThinkingFacade::request_config_for_model()`

## Call Sites (from PROV-079 research)
- `codelet/napi/src/thinking_config.rs:135` — NAPI `get_thinking_config()` calls `request_config_for_model()`
- `codelet/napi/src/session_manager.rs:4897` — agent loop calls `get_thinking_config()` for adaptive models
- The JSON is serialized and passed through to rig-core's `additional_params`

## Downstream Impact
- rig-core `codelet/patches/rig-core/src/providers/anthropic/completion.rs:825` flattens `additional_params` into the request body
- The `thinking` field from thinking_config is merged via `obj.insert(key, value)` in `claude.rs:582-587`
- Adding `display` to the thinking JSON object propagates automatically — no other changes needed
