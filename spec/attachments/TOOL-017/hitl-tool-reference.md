# TOOL-017: Request User Input HITL Tool — Reference Architecture

## Overview

A provider-agnostic human-in-the-loop tool that pauses the agent loop to request structured user input. Follows VTCode's architecture where the core tool declaration returns an error by design, and the actual execution is intercepted by the TUI layer.

## Schema

```json
{
  "type": "object",
  "required": ["questions"],
  "properties": {
    "questions": {
      "type": "array",
      "minItems": 1,
      "maxItems": 3,
      "items": {
        "type": "object",
        "required": ["id", "header", "question"],
        "properties": {
          "id": {
            "type": "string",
            "description": "Stable snake_case identifier for mapping answers."
          },
          "header": {
            "type": "string",
            "description": "Short label shown in UI (≤12 chars)."
          },
          "question": {
            "type": "string",
            "description": "Single-sentence prompt shown to user."
          },
          "options": {
            "type": "array",
            "minItems": 2,
            "maxItems": 3,
            "items": {
              "type": "object",
              "required": ["label", "description"],
              "properties": {
                "label": { "type": "string", "description": "1-5 word label. Suffix recommended option with '(Recommended)'." },
                "description": { "type": "string", "description": "One sentence explaining impact." }
              }
            }
          }
        }
      }
    }
  }
}
```

## Response Schema

```json
{
  "answers": {
    "<question_id>": {
      "selected": ["Option A (Recommended)"],
      "other": "optional freeform text"
    }
  }
}
```

On cancellation:
```json
{
  "cancelled": true
}
```

## Block-on-Oneshot Pattern (from Codex reference)

### How it works

1. **LLM calls the tool** with questions array
2. **Handler validates**:
   - Mode allows user input (TUI must be present)
   - Every question has non-empty options
   - Question IDs are snake_case, headers ≤12 chars
3. **Handler creates oneshot channel**: `(tx, rx) = oneshot::channel()`
4. **Stores tx** in session state (keyed by call_id or sub_id)
5. **Emits event** to TUI: `RequestUserInput { call_id, questions }`
6. **Awaits rx** — the entire tool call is now **suspended**
7. **TUI renders modal** with options + "Other" freeform
8. **User responds** → TUI calls response handler → resolves oneshot
9. **Handler resumes** → serializes answers → returns to LLM

### Integration with PauseHandler

The existing `codelet/tools/src/tool_pause.rs` has `PauseHandler` with `PauseRequest` and `PauseResponse`. The HITL tool should use this:

```rust
// In the tool handler:
let response = pause_handler.pause(PauseRequest {
    kind: PauseKind::UserInput,
    call_id: call_id.clone(),
    questions: validated_questions,
}).await?;

// PauseResponse contains the user's answers
```

The TUI side already handles pause events and can be extended to render the question modal.

### Mode Gating

- **Interactive TUI**: Tool available, renders modal
- **Headless/non-interactive**: Returns error immediately
- Error message: `"request_user_input is unavailable in the current session mode"`

### Always Add "Other" Option

Both Codex and VTCode automatically add a freeform "Other" option to every question:
- Codex: `question.is_other = true` on every question
- VTCode: Appends "Custom note (inline)" as last option
- Our implementation should do the same

## VTCode Reference Files

### Core tool declaration
- `vtcode-core/src/tools/request_user_input.rs` — stub that returns error ("requires interactive UI")
- `vtcode-core/src/tools/registry/builtins.rs` — registered as `CapabilityLevel::Basic`, `ToolPolicy::Allow`

### TUI execution
- `src/agent/runloop/unified/tool_pipeline/hitl.rs` — intercepts tool call in pipeline
- `src/agent/runloop/unified/request_user_input/modal.rs` — `execute_request_user_input_tool()`, wizard modal
- `src/agent/runloop/unified/request_user_input/schema.rs` — `normalize_request_user_input_args()`, validation
- `src/agent/runloop/unified/request_user_input/options.rs` — `resolve_question_options()`, dedup
- `src/agent/runloop/unified/request_user_input/suggestions.rs` — auto-generated options from hints

### VTCode extensions beyond Codex
- `focus_area`: Optional topic hint for auto-suggestions
- `analysis_hints`: Array of weakness/risk hints
- Auto-generates options when LLM omits them
- Detects duplicate options across questions and regenerates

## Codex Reference Files

- `codex-rs/core/src/tools/handlers/request_user_input.rs` — handler with mode gating and validation
- `codex-rs/core/src/codex.rs` — `request_user_input()` (oneshot creation), `request_user_input_response()` (oneshot resolution)
- `codex-rs/tui/src/bottom_pane/request_user_input/mod.rs` — TUI overlay with option navigation, freeform input, queue management
- `codex-rs/tui/src/bottom_pane/request_user_input/render.rs` — rendering logic

## Existing Infrastructure in Codelet

- `codelet/tools/src/tool_pause.rs` — `PauseHandler`, `PauseRequest`, `PauseResponse`, `PauseKind`
- `codelet-napi/src/` — NAPI bindings for TUI communication
- TUI React components for modal rendering
