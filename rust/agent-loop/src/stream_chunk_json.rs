//! StreamChunk → JSON conversion (RPC-072 lift from
//! `rust/napi/src/types.rs:148-359`).
//!
//! Manual serialization helper used by the bridge relay (BRIDGE-001)
//! and the AgentManager handler (AMGR-002). Lifted verbatim from the
//! NAPI source so consumers in this crate can call it without going
//! through `codelet_napi::types::*`.
//!
//! Free function (NOT inherent method) because StreamChunk is now
//! defined in [`codelet_rpc_types`] and Rust's orphan rule forbids
//! inherent impls on types from another crate.

use codelet_rpc_types::StreamChunk;

/// Convert StreamChunk to serde_json::Value for bridge relay (BRIDGE-001).
pub fn stream_chunk_to_json_value(chunk: &StreamChunk) -> serde_json::Value {
    use serde_json::json;

    match chunk {
        StreamChunk::Text {
            text,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "text",
            "text": text,
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::Thinking {
            thinking,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "thinking",
            "thinking": thinking,
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::ToolCall {
            tool_call,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "toolCall",
            "toolCall": {
                "id": tool_call.id,
                "name": tool_call.name,
                "input": tool_call.input,
            },
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::ToolResult {
            tool_result,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "toolResult",
            "toolResult": {
                "toolCallId": tool_result.tool_call_id,
                "content": tool_result.content,
                "isError": tool_result.is_error,
            },
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::ToolProgress {
            tool_progress,
            correlation_id,
            observed_correlation_ids,
        } => json!({
            "type": "toolProgress",
            "toolProgress": {
                "toolCallId": tool_progress.tool_call_id,
                "toolName": tool_progress.tool_name,
                "outputChunk": tool_progress.output_chunk,
                "isStderr": tool_progress.is_stderr,
            },
            "correlationId": correlation_id,
            "observedCorrelationIds": observed_correlation_ids,
        }),
        StreamChunk::SessionStateChange { state } => json!({
            "type": "sessionStateChange",
            "state": format!("{:?}", state),
        }),
        StreamChunk::UserNotification { message, severity } => json!({
            "type": "userNotification",
            "message": message,
            "severity": format!("{:?}", severity),
        }),
        StreamChunk::Interrupted { queued_inputs } => json!({
            "type": "interrupted",
            "queuedInputs": queued_inputs,
        }),
        StreamChunk::TokenUpdate { tokens } => json!({
            "type": "tokenUpdate",
            "tokens": {
                "inputTokens": tokens.input_tokens,
                "outputTokens": tokens.output_tokens,
                "cacheCreationInputTokens": tokens.cache_creation_input_tokens,
                "cacheReadInputTokens": tokens.cache_read_input_tokens,
                "tokensPerSecond": tokens.tokens_per_second,
            },
        }),
        StreamChunk::ContextFillUpdate { context_fill } => json!({
            "type": "contextFillUpdate",
            "contextFill": {
                "fillPercentage": context_fill.fill_percentage,
                "effectiveTokens": context_fill.effective_tokens,
                "threshold": context_fill.threshold,
                "contextWindow": context_fill.context_window,
            },
        }),
        StreamChunk::Done => json!({
            "type": "done",
        }),
        StreamChunk::Error { error } => json!({
            "type": "error",
            "error": error,
        }),
        StreamChunk::UserInput { text } => json!({
            "type": "userInput",
            "text": text,
        }),
        StreamChunk::IncomingMessage { text, images } => {
            let mut obj = json!({
                "type": "supervisorInput",
                "text": text,
            });
            if let Some(imgs) = images {
                obj["images"] = json!(imgs
                    .iter()
                    .map(|i| json!({
                        "data": i.data,
                        "mediaType": i.media_type,
                    }))
                    .collect::<Vec<_>>());
            }
            obj
        }
        StreamChunk::SupervisorPendingInjection {
            supervisor_pending_injection,
        } => json!({
            "type": "supervisorPendingInjection",
            "supervisorPendingInjection": {
                "urgent": supervisor_pending_injection.urgent,
                "content": supervisor_pending_injection.content,
            },
        }),
        StreamChunk::CompactionComplete { compaction_result } => json!({
            "type": "compactionComplete",
            "compactionResult": {
                "originalTokens": compaction_result.original_tokens,
                "compactedTokens": compaction_result.compacted_tokens,
                "compressionRatio": compaction_result.compression_ratio,
                "turnsSummarized": compaction_result.turns_summarized,
                "turnsKept": compaction_result.turns_kept,
            },
        }),
        StreamChunk::FspecCommandRequest { fspec_request } => json!({
            "type": "fspecCommandRequest",
            "fspecRequest": {
                "command": fspec_request.command,
                "argsJson": fspec_request.args_json,
                "projectRoot": fspec_request.project_root,
                "toolCallId": fspec_request.tool_call_id,
            },
        }),
        StreamChunk::FspecCommandResult { fspec_result } => json!({
            "type": "fspecCommandResult",
            "fspecResult": {
                "success": fspec_result.success,
                "data": fspec_result.data,
                "error": fspec_result.error,
                "systemReminder": fspec_result.system_reminder,
                "toolCallId": fspec_result.tool_call_id,
            },
        }),
        StreamChunk::WorkUnitsUpdate { work_units } => json!({
            "type": "workUnitsUpdate",
            "workUnits": work_units.iter().map(|wu| json!({
                "id": wu.id,
                "title": wu.title,
                "status": wu.status,
                "workType": wu.work_type,
            })).collect::<Vec<_>>(),
        }),
        StreamChunk::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
        } => json!({
            "type": "isolationStateChange",
            "isIsolated": is_isolated,
            "worktreePath": worktree_path,
            "baseCommit": base_commit,
        }),
        StreamChunk::FooterStateUpdate {
            cwd,
            display_path,
            is_git_repo,
            branch,
        } => json!({
            "type": "footerStateUpdate",
            "cwd": cwd,
            "displayPath": display_path,
            "isGitRepo": is_git_repo,
            "branch": branch,
        }),
        StreamChunk::DebugStateChange { enabled } => json!({
            "type": "debugStateChange",
            "enabled": enabled,
        }),
        StreamChunk::ContinueStateUpdate { continue_state } => json!({
            "type": "continueStateUpdate",
            "continueState": {
                "enabled": continue_state.enabled,
                "budget": continue_state.budget,
                "nudgesUsed": continue_state.nudges_used,
                "goalActive": continue_state.goal_active,
                "effectiveBudget": continue_state.effective_budget,
                "goalCleared": continue_state.goal_cleared,
                "doneRejections": continue_state.done_rejections,
            },
        }),
    }
}
