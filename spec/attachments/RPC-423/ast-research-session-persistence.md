# AST Research for RPC-423: Session Persistence Investigation

## FspecAgentHooks Implementation

### spawn_agent_loop function
- **File**: `codelet/agent-loop/src/hooks.rs`
- **Line 38**: `fn spawn_agent_loop` - the method that creates duplicate manifests

### Key Findings
1. `FspecAgentHooks::spawn_agent_loop` (line 38) creates a session manifest at lines 60-92
2. `SessionManager::create_session_with_id` (line 571) already creates the manifest BEFORE calling spawn_agent_loop (line 770)
3. The duplicate save overwrites the correct manifest with truncated provider data

### Code Locations
- `codelet/agent-loop/src/hooks.rs:38` - FspecAgentHooks::spawn_agent_loop
- `codelet/sessions/src/session_manager.rs:571` - SessionManager::create_session_with_id manifest creation
- `codelet/sessions/src/session_manager.rs:770` - spawn_agent_loop call site

## SessionManager Implementation

### create_session_with_id
- **File**: `codelet/sessions/src/session_manager.rs`
- **Line 571**: `codelet_core::persistence::save_session(&manifest)` - creates manifest with full provider string
- **Line 770**: `self.hooks().spawn_agent_loop(session.clone(), input_rx, mcp_injection_rx)` - calls hooks AFTER manifest creation

### list_sessions
- **File**: `codelet/sessions/src/session_manager.rs`
- **Line 361**: Merges in-memory sessions with persisted sessions from disk

## Persistence Layer

### SessionStore
- **File**: `codelet/core/src/persistence/manifest.rs`
- **Line 293**: `SessionStore::save()` - writes manifest to disk
- **Line 569**: `save_session()` - public API for saving sessions

### Message Persistence
- **File**: `codelet/agent-loop/src/persist.rs`
- `persist_user_message()` - called from agent_loop when user input received
- `persist_assistant_message_internal()` - called from BackgroundOutput on Done
- `persist_tool_result_internal()` - called from BackgroundOutput on ToolResult

## Root Cause Analysis

The duplicate manifest creation in `FspecAgentHooks::spawn_agent_loop` (lines 60-92) overwrites the manifest created by `SessionManager::create_session_with_id` (line 571). The hooks code reads `session.provider_id` which is only the provider part (e.g., "anthropic"), not the full provider/model string (e.g., "anthropic/claude-sonnet-4"). This causes the persisted manifest to have incomplete provider data, breaking session resume.

## Fix

Remove the duplicate manifest creation block (lines 44-92) from `FspecAgentHooks::spawn_agent_loop`. The manifest is already created correctly by `SessionManager::create_session_with_id` before `spawn_agent_loop` is called.
