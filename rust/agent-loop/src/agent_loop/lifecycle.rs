use codelet_core::lifecycle_hooks::{run_session_end, run_session_start, HookMessageLevel};
use codelet_rpc_types::{NotificationSeverity, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;

pub(super) async fn start_session_hooks(session: &BackgroundSession) {
    if let Some(ref hooks) = session.lifecycle_hooks {
        let ctx = session.hook_context();
        let outcome = run_session_start(hooks, &ctx, "startup").await;
        if !outcome.additional_context.is_empty() {
            let mut inner = session.inner.lock().await;
            let combined_context = outcome.additional_context.join("\n");
            inner.add_system_reminder(
                codelet_cli::session::SystemReminderType::FspecWorkflow,
                &combined_context,
            );
            drop(inner);
        }
        for msg in &outcome.messages {
            if msg.level == HookMessageLevel::Warning || msg.level == HookMessageLevel::Error {
                tracing::warn!(content = %msg.content, "[HOOK-013] session_start hook warning or error");
                session.handle_output(StreamChunk::user_notification(
                    format!("Hook: {}", msg.content),
                    NotificationSeverity::Warning,
                ));
            }
        }
    }
}

pub(super) async fn end_session_hooks(session: &BackgroundSession) {
    if let Some(ref hooks) = session.lifecycle_hooks {
        let ctx = session.hook_context();
        let _outcome = run_session_end(hooks, &ctx, "exit").await;
    }
}
