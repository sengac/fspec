# AST Research: Supervisor Infrastructure Removal Targets

## Rust Code — codelet/napi/src/session_manager.rs

### Structs to Remove
- `SupervisorRole` — line 259 (struct with name, brief, auto_inject, breakpoint_config)
- `SupervisorInput` — line 296 (struct with session_id, role_name, message, images)
- `ObservationBuffer` — line 383 (struct with chunks, pending_eval, last_eval_time, etc.)
- `SupervisorRoleInfo` — line 6868 (NAPI export struct)

### Functions to Remove
- `format_evaluation_prompt` — line 481 (formats observation buffer for LLM evaluation)
- `evaluate_and_maybe_inject` — line 570 (LLM evaluation of observations)
- `format_supervisor_input` — line 360 (formats SupervisorInput for injection)
- `supervisor_agent_loop` — line 5712 (the specialized async loop with observation pipeline)
- `session_create_supervisor` — line 6922 (NAPI: spawns supervisor_agent_loop)
- `supervisor_inject` — line 6997 (NAPI: injects message into subordinate)

### Methods to Remove from BackgroundSession
- `receive_supervisor_input` — line 1512
- `supervisor_input_sender` — line 1521
- Fields: `supervisor_input_tx`, `supervisor_input_rx` — lines 899-900

### Fields to Simplify on BackgroundSession
- `role: RwLock<Option<SupervisorRole>>` — line 895 → change to `role: RwLock<Option<String>>`
- `set_role` / `get_role` methods — lines 1296, 1303 → simplify for String

## Rust Code — codelet/napi/src/types.rs

### Types to Remove
- `SupervisorInputImage` — line 34
- `StreamChunk::SupervisorInput` variant — line 321
- `supervisor_input_with_images` method — line 459
- `supervisor_input` method — line 455

## TypeScript — TUI Files to Delete Entirely
- `src/tui/components/SupervisorTemplateList.tsx`
- `src/tui/components/SupervisorCreateView.tsx`
- `src/tui/components/SupervisorTemplateForm.tsx`
- `src/tui/types/supervisorTemplate.ts`
- `src/tui/utils/supervisorTemplateStorage.ts`

## TypeScript — TUI Files to Modify
- `src/tui/components/AgentView.tsx` — remove supervisor imports, /supervisor command handler
- `src/tui/utils/slashCommands.ts` — remove supervisor entry from command list

## TypeScript — TUI Files to Keep
- `src/tui/components/SplitSessionView.tsx` — stays (split pane display)
- `src/tui/utils/correlationMapping.ts` — stays (cross-pane highlighting)
- `src/tui/utils/chunkProcessor.ts` — parseSupervisorPrefix stays (display concern)
- `src/tui/types/conversation.ts` — 'supervisor-input' type stays (display)

## Test Files to Remove/Update
- `codelet/napi/tests/watcher_interjection_test.rs` — remove entirely
- `codelet/napi/tests/message_duplication_test.rs` — update TestSupervisorInput references
- `codelet/napi/src/session_manager.rs` test module — remove supervisor-specific tests (~L2770+)

## NAPI Type Declarations
- `codelet/napi/index.d.ts` — regenerated after Rust changes
