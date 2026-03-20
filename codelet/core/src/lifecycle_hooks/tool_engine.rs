//! Agent Lifecycle Hooks — Tool Use Engine
//!
//! Async hook runner functions for pre_tool_use and post_tool_use events.
//! These use CompiledHookGroup with regex matchers, unlike the simpler
//! HookDefinition-based events handled by the main engine module.

use super::compiled::CompiledLifecycleHooks;
use super::executor::execute_command;
use super::helpers::collect_exit_code_messages;
use super::outcome::{
    HookMessage, HookMessageLevel, PostToolOutcome, PreToolHookDecision, PreToolOutcome,
};
use super::payloads::{PostToolUsePayload, PreToolUsePayload};
use super::response::{extract_reason, interpret_pre_tool_result, try_parse_json_response};

use super::engine::HookContext;

/// Run pre_tool_use hooks for a specific tool (all matching groups).
pub async fn run_pre_tool(
    hooks: &CompiledLifecycleHooks,
    ctx: &HookContext,
    tool_name: &str,
    tool_input: &serde_json::Value,
) -> PreToolOutcome {
    let payload = PreToolUsePayload {
        hook_event_name: "PreToolUse".to_string(),
        session_id: ctx.session_id.clone(),
        cwd: ctx.cwd.clone(),
        tool_name: tool_name.to_string(),
        tool_input: tool_input.clone(),
        transcript_path: ctx.transcript_path.clone(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();

    let mut messages = Vec::new();

    for group in &hooks.pre_tool_use {
        if !group.matcher.matches(tool_name) {
            continue;
        }

        for cmd in &group.commands {
            let result = execute_command(
                &cmd.command,
                &payload_json,
                cmd.timeout,
                ctx,
                hooks.global_shell.as_deref(),
            )
            .await;

            // Timeout on pre_tool_use → Deny (safety-first)
            if result.timed_out {
                messages.push(HookMessage {
                    level: HookMessageLevel::Warning,
                    content: format!(
                        "pre_tool_use hook timed out after {}s — denying for safety",
                        cmd.timeout
                    ),
                });
                return PreToolOutcome {
                    decision: PreToolHookDecision::Deny,
                    reason: Some(format!("Hook timed out after {}s", cmd.timeout)),
                    messages,
                };
            }

            let decision = interpret_pre_tool_result(&result, &mut messages);
            match decision {
                PreToolHookDecision::Continue => {}
                other => {
                    return PreToolOutcome {
                        decision: other,
                        reason: extract_reason(&result),
                        messages,
                    };
                }
            }
        }
    }

    PreToolOutcome {
        decision: PreToolHookDecision::Continue,
        reason: None,
        messages,
    }
}

/// Run post_tool_use hooks for a specific tool (all matching groups).
pub async fn run_post_tool(
    hooks: &CompiledLifecycleHooks,
    ctx: &HookContext,
    tool_name: &str,
    tool_input: &serde_json::Value,
    tool_response: &str,
) -> PostToolOutcome {
    let payload = PostToolUsePayload {
        hook_event_name: "PostToolUse".to_string(),
        session_id: ctx.session_id.clone(),
        cwd: ctx.cwd.clone(),
        tool_name: tool_name.to_string(),
        tool_input: tool_input.clone(),
        tool_response: tool_response.to_string(),
        transcript_path: ctx.transcript_path.clone(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();

    let mut messages = Vec::new();
    let mut additional_context = Vec::new();

    for group in &hooks.post_tool_use {
        if !group.matcher.matches(tool_name) {
            continue;
        }

        for cmd in &group.commands {
            let result = execute_command(
                &cmd.command,
                &payload_json,
                cmd.timeout,
                ctx,
                hooks.global_shell.as_deref(),
            )
            .await;

            if result.timed_out {
                messages.push(HookMessage {
                    level: HookMessageLevel::Warning,
                    content: format!("post_tool_use hook timed out after {}s", cmd.timeout),
                });
                continue;
            }

            if let Some(json_resp) = try_parse_json_response(&result.stdout) {
                if let Some(ref hso) = json_resp.hook_specific_output {
                    if let Some(ref ctx_str) = hso.additional_context {
                        additional_context.push(ctx_str.clone());
                    }
                }
            }

            collect_exit_code_messages(&result, None, &mut messages);
        }
    }

    PostToolOutcome {
        additional_context,
        messages,
    }
}
