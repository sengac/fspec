# AST Research: Session Message Persistence Migration

## Summary
This document captures AST-based code analysis for migrating message persistence from TypeScript to Rust.

## TypeScript Persistence Calls (TO BE REMOVED)

### persistenceStoreMessageEnvelope in AgentView.tsx
```
src/tui/components/AgentView.tsx:55:3:persistenceStoreMessageEnvelope    (import)
src/tui/components/AgentView.tsx:2638:9:persistenceStoreMessageEnvelope  (call - user message)
src/tui/components/AgentView.tsx:2917:17:persistenceStoreMessageEnvelope (call - assistant message)
src/tui/components/AgentView.tsx:2949:17:persistenceStoreMessageEnvelope (call - tool result)
src/tui/components/AgentView.tsx:3391:13:persistenceStoreMessageEnvelope (call - final assistant)
```

### persistTokenState in AgentView.tsx
```
src/tui/components/AgentView.tsx:41:3:persistTokenState  (import)
src/tui/components/AgentView.tsx:3121:13:persistTokenState (call - on Done chunk)
src/tui/components/AgentView.tsx:4337:7:persistTokenState  (call - other)
```

### Migration Action
- Remove all 4 `persistenceStoreMessageEnvelope` calls (lines 2638, 2917, 2949, 3391)
- Remove both `persistTokenState` calls (lines 3121, 4337)
- Keep imports until all calls removed, then remove imports (lines 55, 41)

## Rust Persistence Functions (NAPI Bindings)

### Available Persistence Functions
```
codelet/napi/src/persistence/napi_bindings.rs:591:1:persistence_store_message_envelope
codelet/napi/src/persistence/napi_bindings.rs:302:1:persistence_update_session_tokens
codelet/napi/src/persistence/napi_bindings.rs:324:1:persistence_set_session_tokens
codelet/napi/src/persistence/napi_bindings.rs:354:1:persistence_set_compaction_state
codelet/napi/src/persistence/napi_bindings.rs:368:1:persistence_clear_compaction_state
```

### Key Observation
- `persistence_set_compaction_state` EXISTS (line 354) but is NEVER CALLED
- Compaction state persistence must be wired up in both hook-triggered and manual compaction pathways

## Rust Session Manager Functions

### Key Entry Points
```
codelet/napi/src/session_manager.rs:4940:1:session_compact (manual compaction entry)
codelet/napi/src/session_manager.rs:3598:26:BackgroundOutput (streaming output handler)
codelet/napi/src/session_manager.rs:4660:1:session_restore_messages (resume entry)
codelet/napi/src/session_manager.rs:4822:1:session_restore_token_state (resume tokens)
```

### agent_loop Location
- The agent_loop function handles message flow but AST search for exact function name returned no matches
- Likely due to function being nested or having complex signature
- Manual inspection needed around line 3538 as noted in work unit architecture notes

## Compaction Pathways

### execute_compaction Usage
```
codelet/cli/src/interactive/repl_loop.rs:2:33:execute_compaction     (import)
codelet/cli/src/interactive/repl_loop.rs:88:19:execute_compaction    (call - CLI REPL)
codelet/cli/src/interactive/stream_loop.rs:18:33:execute_compaction  (import)
codelet/cli/src/interactive/stream_loop.rs:192:15:execute_compaction (call - hook-triggered)
codelet/cli/src/interactive/stream_loop.rs:1172:15:execute_compaction (call - hook-triggered)
codelet/cli/src/interactive_helpers.rs:171:14:execute_compaction     (function definition)
codelet/napi/src/session_manager.rs:10:39:execute_compaction         (import)
codelet/napi/src/session_manager.rs:4978:35:execute_compaction       (call - NAPI manual)
```

### DRY Requirement
Both pathways call `execute_compaction` from `interactive_helpers.rs` - this is where compaction state persistence should be added to ensure both pathways persist correctly.

## Implementation Plan

### Phase 1: Add Rust Persistence (NAPI Path)
1. In `session_manager.rs` agent_loop (~line 3538):
   - Persist user message when input received from channel
   - Persist assistant message when tool_use detected
   - Persist tool_result when tool completes
   - Persist final assistant message on Done chunk
   - Persist accumulated content on error before emitting Error chunk
   - Persist accumulated content on interrupt before emitting Interrupted chunk
   - Persist token state on Done chunk

2. In `interactive_helpers.rs` execute_compaction:
   - Add `persistence_set_compaction_state` call after summary generated

### Phase 2: Add Rust Persistence (CLI Path)
1. In `stream_loop.rs`:
   - Mirror all persistence calls from NAPI path
   - Ensure CLI works fully without TypeScript

### Phase 3: Remove TypeScript Persistence
After Phase 1 & 2 validated with integration tests:
1. Remove `persistenceStoreMessageEnvelope` calls from AgentView.tsx
2. Remove `persistTokenState` calls from AgentView.tsx
3. Remove imports

## Test Evidence Required
- Real session data from ~/.fspec/sessions showing:
  - No sessions ending with tool_result
  - Compaction state preserved in manifests
  - Token state accurate after resume
  - All messages intact after error/interrupt scenarios
