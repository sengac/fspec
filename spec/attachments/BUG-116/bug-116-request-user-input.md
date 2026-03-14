# BUG-116: Codex facade maps request_user_input to HITL tool

## Architecture

```
Codex LLM → request_user_input (Codex-native schema)
    ↓
CodexRequestUserInputFacade (codelet/tools/src/facade/codex.rs)
    ↓
RequestUserInputTool (codelet/tools/src/ — provider-agnostic, TOOL-017)
    ↓
PauseHandler → TUI modal → oneshot response
```

## Codex-Native Tool Schema to Map

### `request_user_input` → HITL tool

```json
{
  "name": "request_user_input",
  "parameters": {
    "questions": {
      "type": "array",
      "minItems": 1,
      "maxItems": 3,
      "items": {
        "type": "object",
        "required": ["id", "header", "question"],
        "properties": {
          "id": { "type": "string", "description": "Stable snake_case identifier" },
          "header": { "type": "string", "description": "Short UI label (≤12 chars)" },
          "question": { "type": "string", "description": "Single-sentence prompt" },
          "options": {
            "type": "array",
            "minItems": 2,
            "maxItems": 3,
            "items": {
              "type": "object",
              "required": ["label", "description"],
              "properties": {
                "label": { "type": "string", "description": "1-5 word label" },
                "description": { "type": "string", "description": "One sentence impact" }
              }
            }
          }
        }
      }
    }
  },
  "required": ["questions"]
}
```

The Codex schema is already close to the HITL tool schema (TOOL-017). The facade is a thin translation:
- `questions` → pass through directly (same structure)
- Codex requires non-empty `options` on every question — the HITL tool should validate this
- Codex always adds "Other" freeform option (`is_other = true`) — the HITL tool should do the same

### Response Schema

```json
{
  "answers": {
    "<question_id>": {
      "selected": ["Option A"],
      "other": "optional freeform text"
    }
  }
}
```

## How Block-on-Oneshot Works

The HITL tool (TOOL-017) implements this pattern. The facade just translates schemas:

1. LLM calls `request_user_input({ questions: [...] })`
2. Facade maps to HITL tool
3. HITL tool creates oneshot channel, pauses agent via PauseHandler
4. TUI renders question modal with options + freeform
5. User answers → oneshot resolves → HITL tool returns answers
6. Facade formats response in Codex-expected shape

### Cancellation

If the user cancels or the session is interrupted:
- Codex expects: error message "request_user_input was cancelled before receiving a response"
- The facade should catch the HITL tool's cancellation and return this Codex-specific error message

### Mode Gating

Codex gates this by collaboration mode. Our facade should:
- Return error when no TUI is present (headless/non-interactive mode)
- Error message: "request_user_input is unavailable in the current session mode"

## VTCode Reference

VTCode implements this for **all providers**:

- **Core tool** (`vtcode-core/src/tools/request_user_input.rs`): Returns error by design — actual execution intercepted by TUI
- **HITL pipeline** (`src/agent/runloop/unified/tool_pipeline/hitl.rs`): Intercepts the tool call in the TUI runloop
- **Wizard modal** (`src/agent/runloop/unified/request_user_input/modal.rs`): Multi-step wizard with options + freeform
- **Auto-suggestions** (`suggestions.rs`): When LLM omits options, VTCode auto-generates from question text and hints
- **Provider-agnostic**: Registered once in builtin registry, available for Anthropic, OpenAI, Gemini, Ollama, etc.

VTCode also extends the Codex schema with:
- `focus_area`: Optional topic hint for auto-suggested options
- `analysis_hints`: Optional weakness/risk hints for auto-generated choices

## References

- Codex CLI request_user_input handler: `/tmp/codex/codex-rs/core/src/tools/handlers/request_user_input.rs`
- Codex session integration: `/tmp/codex/codex-rs/core/src/codex.rs` (`request_user_input()`)
- Codex TUI overlay: `/tmp/codex/codex-rs/tui/src/bottom_pane/request_user_input/mod.rs`
- VTCode tool declaration: `/tmp/VTCode/vtcode-core/src/tools/request_user_input.rs`
- VTCode HITL pipeline: `/tmp/VTCode/src/agent/runloop/unified/tool_pipeline/hitl.rs`
- VTCode TUI modal: `/tmp/VTCode/src/agent/runloop/unified/request_user_input/modal.rs`
- Existing Codex facade: `codelet/tools/src/facade/codex.rs`
- Tool pause system: `codelet/tools/src/tool_pause.rs`
- HITL tool: TOOL-017
