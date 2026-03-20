//! Agent Lifecycle Hooks — Execution Engine
//!
//! High-level async hook runner functions for session lifecycle events.
//! Builds JSON payloads, delegates to the executor for process management,
//! and interprets results via the response module.
//!
//! Tool-specific hooks (pre_tool_use, post_tool_use) are in tool_engine.rs.

use super::compiled::CompiledLifecycleHooks;
use super::executor::execute_command;
use super::helpers::{collect_exit_code_messages, collect_non_tool_context};
use super::outcome::{
    HookMessage, HookMessageLevel, NotificationOutcome, SessionEndOutcome, SessionStartOutcome,
    UserPromptOutcome,
};
use super::payloads::{
    NotificationPayload, SessionEndPayload, SessionStartPayload, UserPromptSubmitPayload,
};
use super::response::try_parse_json_response;

/// Context passed to every hook execution.
#[derive(Debug, Clone)]
pub struct HookContext {
    pub session_id: String,
    pub cwd: String,
    pub transcript_path: String,
}

// ===== Public API =====

/// Run all session_start hooks and collect outcomes.
pub async fn run_session_start(
    hooks: &CompiledLifecycleHooks,
    ctx: &HookContext,
    source: &str,
) -> SessionStartOutcome {
    let payload = SessionStartPayload {
        hook_event_name: "SessionStart".to_string(),
        session_id: ctx.session_id.clone(),
        cwd: ctx.cwd.clone(),
        source: source.to_string(),
        transcript_path: ctx.transcript_path.clone(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();

    let mut messages = Vec::new();
    let mut additional_context = Vec::new();

    for hook_def in &hooks.session_start {
        let result = execute_command(
            &hook_def.command,
            &payload_json,
            hook_def.timeout,
            ctx,
            hooks.global_shell.as_deref(),
        )
        .await;

        if result.timed_out {
            messages.push(HookMessage {
                level: HookMessageLevel::Warning,
                content: format!(
                    "Hook '{}' timed out after {}s",
                    hook_def.name, hook_def.timeout
                ),
            });
            continue;
        }

        collect_non_tool_context(&result, hook_def, &mut messages, &mut additional_context);
    }

    SessionStartOutcome {
        messages,
        additional_context,
    }
}

/// Run all session_end hooks.
pub async fn run_session_end(
    hooks: &CompiledLifecycleHooks,
    ctx: &HookContext,
    reason: &str,
) -> SessionEndOutcome {
    let payload = SessionEndPayload {
        hook_event_name: "SessionEnd".to_string(),
        session_id: ctx.session_id.clone(),
        cwd: ctx.cwd.clone(),
        reason: reason.to_string(),
        transcript_path: ctx.transcript_path.clone(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();

    let mut messages = Vec::new();

    for hook_def in &hooks.session_end {
        let result = execute_command(
            &hook_def.command,
            &payload_json,
            hook_def.timeout,
            ctx,
            hooks.global_shell.as_deref(),
        )
        .await;

        if result.timed_out {
            messages.push(HookMessage {
                level: HookMessageLevel::Warning,
                content: format!(
                    "Hook '{}' timed out after {}s",
                    hook_def.name, hook_def.timeout
                ),
            });
            continue;
        }

        collect_exit_code_messages(&result, Some(&hook_def.name), &mut messages);
    }

    SessionEndOutcome { messages }
}

/// Run all user_prompt_submit hooks.
pub async fn run_user_prompt(
    hooks: &CompiledLifecycleHooks,
    ctx: &HookContext,
    prompt: &str,
) -> UserPromptOutcome {
    let payload = UserPromptSubmitPayload {
        hook_event_name: "UserPromptSubmit".to_string(),
        session_id: ctx.session_id.clone(),
        cwd: ctx.cwd.clone(),
        prompt: prompt.to_string(),
        transcript_path: ctx.transcript_path.clone(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();

    let mut messages = Vec::new();
    let mut additional_context = Vec::new();
    let mut allow_prompt = true;
    let mut block_reason: Option<String> = None;

    for hook_def in &hooks.user_prompt_submit {
        let result = execute_command(
            &hook_def.command,
            &payload_json,
            hook_def.timeout,
            ctx,
            hooks.global_shell.as_deref(),
        )
        .await;

        if result.timed_out {
            messages.push(HookMessage {
                level: HookMessageLevel::Warning,
                content: format!(
                    "Hook '{}' timed out after {}s",
                    hook_def.name, hook_def.timeout
                ),
            });
            continue;
        }

        // Check for blocking via JSON response
        if let Some(json_resp) = try_parse_json_response(&result.stdout) {
            if json_resp.continue_field == Some(false) {
                allow_prompt = false;
                block_reason = json_resp
                    .reason
                    .clone()
                    .or_else(|| Some("Blocked by hook".to_string()));
            }
            if let Some(ref hso) = json_resp.hook_specific_output {
                if let Some(ref ctx_str) = hso.additional_context {
                    additional_context.push(ctx_str.clone());
                }
            }
        } else {
            // Plain text stdout → additional context
            let stdout_trimmed = result.stdout.trim();
            if !stdout_trimmed.is_empty() {
                additional_context.push(stdout_trimmed.to_string());
            }
        }

        // Check for blocking via exit code 2 + stderr
        if !allow_prompt {
            // Already blocked by JSON
        } else if result.exit_code == Some(2) && !result.stderr.trim().is_empty() {
            allow_prompt = false;
            block_reason = Some(result.stderr.trim().to_string());
        }

        collect_exit_code_messages(&result, Some(&hook_def.name), &mut messages);
    }

    UserPromptOutcome {
        allow_prompt,
        block_reason,
        additional_context,
        messages,
    }
}

/// Run all notification hooks.
pub async fn run_notification(
    hooks: &CompiledLifecycleHooks,
    ctx: &HookContext,
    notification_type: &str,
    title: &str,
    message: &str,
) -> NotificationOutcome {
    let payload = NotificationPayload {
        hook_event_name: "Notification".to_string(),
        session_id: ctx.session_id.clone(),
        cwd: ctx.cwd.clone(),
        notification_type: notification_type.to_string(),
        title: title.to_string(),
        message: message.to_string(),
        transcript_path: ctx.transcript_path.clone(),
    };
    let payload_json = serde_json::to_string(&payload).unwrap_or_default();

    let mut messages = Vec::new();

    for hook_def in &hooks.notification {
        let result = execute_command(
            &hook_def.command,
            &payload_json,
            hook_def.timeout,
            ctx,
            hooks.global_shell.as_deref(),
        )
        .await;

        if result.timed_out {
            messages.push(HookMessage {
                level: HookMessageLevel::Warning,
                content: format!(
                    "Hook '{}' timed out after {}s",
                    hook_def.name, hook_def.timeout
                ),
            });
            continue;
        }

        collect_exit_code_messages(&result, Some(&hook_def.name), &mut messages);
    }

    NotificationOutcome { messages }
}
