# BUG-113: Codex agent missing update_plan tool

## Problem

The Codex CLI native tool set includes an `update_plan` tool for structured task planning. This tool is missing from the Codex facade and agent registration.

## Codex CLI Native Spec

From `codex-rs/core/src/tools/spec.rs`:

```
name: "update_plan"
params:
  - explanation: String - reasoning about the plan update
  - plan: Array<{step: String, status: String}> (required)
    - step: String - description of the plan step
    - status: String - one of "pending", "in_progress", "completed"
```

This is a `ToolSpec::Function` type tool. The Codex CLI uses this to let the model maintain a visible plan that updates as work progresses.

## Current State

No `update_plan` tool exists anywhere in the codebase. There is no equivalent internal mechanism.

## Impact

When the model attempts to call `update_plan` (a tool it was trained on), it gets an unknown tool error. This prevents the model from using its trained planning workflow, which may degrade the quality of multi-step task execution.

## Recommended Fix

Create an `UpdatePlanTool` as a standalone `rig::tool::Tool` that:

1. Accepts `explanation` (optional String) and `plan` (required Array of step objects)
2. Stores the plan in session-scoped state (or emits it as structured output)
3. Returns the formatted plan as a confirmation message

Options for plan storage:
- **Session state**: Store in a session-scoped HashMap so the plan persists across turns
- **Output-only**: Simply format and return the plan as text (the model maintains state in context)
- **TUI integration**: Emit the plan as a structured chunk that the TUI can render as a progress panel

The simplest approach is output-only: accept the plan, format it nicely, and return it. The model tracks state in its context window.

Register it in `CodexProvider::create_rig_agent()`.

## References

- Codex CLI tool spec: `codex-rs/core/src/tools/spec.rs`
- Codex CLI plan rendering: `codex-rs/core/src/plan.rs` (if available)
