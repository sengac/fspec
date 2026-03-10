//! inject_summary handler — bridges codelet-tools InjectSummaryTool
//! to the session manipulation logic in codelet-napi.
//!
//! Feature: spec/features/inject-summary-handler.feature
//!
//! The handler does NOT lock session.inner. The agent_loop
//! holds that lock during streaming, so locking it here would deadlock.
//! Instead, the handler stores the DAG content in `pending_dag_content`
//! (an Arc<std::sync::Mutex<Option<String>>> on BackgroundSession) and
//! returns immediately. After the stream completes, the agent_loop checks
//! for pending DAG content and applies the session state changes.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use codelet_common::token_estimator::count_tokens;
use codelet_tools::inject_summary::{InjectSummaryHandler, InjectSummaryResult};
use uuid::Uuid;

/// Callback invoked after inject_summary stores the DAG and clears the
/// compaction flag. Used by the NAPI session manager to emit
/// `CompactionComplete` immediately so the TUI drops the compaction
/// indicator without waiting for the stream to finish.
///
/// Arguments: (injected_tokens: u32)
pub type OnInjectedCallback = Arc<dyn Fn(u32) + Send + Sync>;

/// Create an inject_summary handler for a specific session.
///
/// The handler captures:
/// - `pending_dag`: shared storage for the DAG content
/// - `context_window`: for estimating remaining budget
/// - `compaction_in_progress`: cleared after DAG is applied
/// - `on_injected`: optional callback fired immediately after DAG is stored
///
/// When invoked by the LLM tool call, the handler:
/// 1. Stores DAG content in `pending_dag` (does NOT touch session state)
/// 2. Clears the `compaction_in_progress` flag
/// 3. Fires `on_injected` callback to emit CompactionComplete immediately
/// 4. Returns estimated token counts
///
/// The actual "partition → clear → restore → inject" happens AFTER the
/// stream completes, in the agent_loop, which has the session lock.
pub fn create_handler(
    pending_dag: Arc<std::sync::Mutex<Option<String>>>,
    context_window: u64,
    compaction_in_progress: Arc<AtomicBool>,
    on_injected: Option<OnInjectedCallback>,
) -> InjectSummaryHandler {
    Arc::new(move |_session_id: Uuid, content: String| {
        let compaction_flag = compaction_in_progress.clone();

        // Step 1: Wrap DAG content
        let wrapped = wrap_dag_content(&content);

        // Step 2: Estimate token counts (no session lock needed)
        let injected_tokens = count_tokens(&wrapped) as u64;
        // Rough estimate — actual budget recalculated when agent_loop applies the DAG
        let remaining_budget = context_window.saturating_sub(injected_tokens);

        // Step 3: Store DAG content for deferred application by agent_loop
        if let Ok(mut guard) = pending_dag.lock() {
            *guard = Some(wrapped);
        } else {
            return Err("Failed to acquire pending_dag lock".to_string());
        }

        // Step 4: Clear compaction_in_progress flag
        // SessionSearch will stop trimming from this point
        compaction_flag.store(false, Ordering::SeqCst);

        // Step 5: Fire on_injected callback to emit CompactionComplete immediately.
        // This lets the TUI drop the compaction indicator as soon as inject_summary
        // runs, instead of waiting for the stream to end and apply_pending_dag.
        if let Some(ref cb) = on_injected {
            cb(injected_tokens as u32);
        }

        Ok(InjectSummaryResult {
            injected_tokens,
            remaining_budget,
        })
    })
}

/// Wrap DAG content in system-reminder compaction-dag markers.
///
/// The resulting message content will be:
/// ```text
/// <system-reminder>
/// <!-- type:compaction-dag -->
/// {dag_content}
/// </system-reminder>
/// ```
pub fn wrap_dag_content(content: &str) -> String {
    format!(
        "<system-reminder>\n<!-- type:compaction-dag -->\n{}\n</system-reminder>",
        content
    )
}

/// Determine if the Done handler should set session status to Idle.
///
/// The Done handler must NOT set Idle when either:
/// - `compaction_in_progress` is still true (agent is building DAG)
/// - `pending_dag_content` has content (inject_summary stored DAG but apply_pending_dag hasn't run)
///
/// Returns true only when BOTH conditions are false — meaning it's safe to go Idle.
///
/// Truth table:
/// | compaction_in_progress | has_pending_dag | should_idle |
/// |------------------------|-----------------|-------------|
/// | true                   | true            | NO          |
/// | true                   | false           | NO          |
/// | false                  | true            | NO          |
/// | false                  | false           | YES         |
pub fn should_idle_on_done(
    compaction_in_progress: &AtomicBool,
    pending_dag_content: &std::sync::Mutex<Option<String>>,
) -> bool {
    let has_pending_dag = pending_dag_content
        .lock()
        .map(|guard| guard.is_some())
        .unwrap_or(false);
    let compaction_active = compaction_in_progress.load(Ordering::Acquire);
    !compaction_active && !has_pending_dag
}

/// Apply pending DAG content to a session.
///
/// Called by the agent_loop AFTER the stream completes, while it still holds
/// the session lock. This is where the actual "clear → restore → inject" happens.
///
/// Returns true if a DAG was applied, false if nothing was pending.
pub fn apply_pending_dag(
    session: &mut codelet_cli::session::Session,
    pending_dag: &Arc<std::sync::Mutex<Option<String>>>,
) -> bool {
    use codelet_cli::interactive_helpers::{recalculate_token_tracker, reset_session_to_reminders};
    use rig::message::{Message, UserContent};
    use rig::OneOrMany;

    // Take the pending DAG content (if any)
    let dag_content = match pending_dag.lock() {
        Ok(mut guard) => guard.take(),
        Err(_) => None,
    };

    let Some(wrapped) = dag_content else {
        return false;
    };

    // Partition, clear, restore system reminders, clear turns
    let _counts = reset_session_to_reminders(session);

    // Append the wrapped DAG content as a system-reminder-style user message
    session.messages.push(Message::User {
        content: OneOrMany::one(UserContent::text(&wrapped)),
    });

    // Recalculate token tracker from actual post-injection messages
    recalculate_token_tracker(session);

    true
}

#[cfg(test)]
mod tests {
    //! Feature: spec/features/in-view-dag-compaction.feature
    //!
    //! Tests for inject_summary handler flag clearing.

    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    // Scenario: inject_summary handler clears compaction_in_progress flag
    #[test]
    fn test_inject_summary_clears_compaction_flag() {
        // @step Given a session with compaction_in_progress flag set to true
        let compaction_flag = Arc::new(AtomicBool::new(true));
        assert!(compaction_flag.load(Ordering::Relaxed), "Flag should start as true");

        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let context_window: u64 = 200_000;

        let handler = create_handler(
            pending_dag.clone(),
            context_window,
            compaction_flag.clone(),
            None,
        );

        let session_id = Uuid::new_v4();
        let dag_content = "# D2: Architecture\n- JWT auth\n# D1: Current Arc\n- Implementing login".to_string();

        // @step When the agent calls inject_summary with DAG content
        let result = handler(session_id, dag_content);
        assert!(result.is_ok(), "inject_summary should succeed");

        // @step Then the inject_summary handler should clear the compaction_in_progress flag
        assert!(
            !compaction_flag.load(Ordering::Relaxed),
            "compaction_in_progress flag should be cleared after inject_summary"
        );

        // @step And the DAG content should be stored in pending_dag
        let stored = pending_dag.lock().unwrap();
        assert!(stored.is_some(), "pending_dag should have content");
        assert!(stored.as_ref().unwrap().contains("compaction-dag"));
    }

    // Scenario: inject_summary stores wrapped DAG content
    #[test]
    fn test_inject_summary_stores_wrapped_content() {
        let compaction_flag = Arc::new(AtomicBool::new(true));
        let pending_dag = Arc::new(std::sync::Mutex::new(None));

        let handler = create_handler(
            pending_dag.clone(),
            200_000,
            compaction_flag.clone(),
            None,
        );

        let dag = "# D2: Durable\n- Using bcrypt\n# D1: Arc\n- Building login".to_string();
        let result = handler(Uuid::new_v4(), dag);
        assert!(result.is_ok());

        let stored = pending_dag.lock().unwrap();
        let content = stored.as_ref().unwrap();
        assert!(content.contains("<system-reminder>"));
        assert!(content.contains("<!-- type:compaction-dag -->"));
        assert!(content.contains("Using bcrypt"));
    }

    // Scenario: inject_summary returns token counts
    #[test]
    fn test_inject_summary_returns_token_counts() {
        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let handler = create_handler(
            pending_dag,
            200_000,
            Arc::new(AtomicBool::new(true)),
            None,
        );

        let result = handler(Uuid::new_v4(), "# Summary".to_string());
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.injected_tokens > 0, "Should report injected tokens");
        assert!(result.remaining_budget > 0, "Should report remaining budget");
    }

    // Scenario: apply_pending_dag with no pending content is a no-op
    #[test]
    fn test_apply_pending_dag_no_content() {
        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let provider_manager = codelet_providers::ProviderManager::new()
            .expect("Need at least one API key for tests");
        let mut session = codelet_cli::session::Session::from_provider_manager(provider_manager);

        let applied = apply_pending_dag(&mut session, &pending_dag);
        assert!(!applied, "Should return false when no pending DAG");
    }

    // Scenario: Compaction system instruction guides agent through DAG construction
    #[test]
    fn test_compaction_instruction_content() {
        use codelet_cli::interactive_helpers::COMPACTION_SYSTEM_INSTRUCTION;

        let instruction = COMPACTION_SYSTEM_INSTRUCTION;
        assert!(!instruction.is_empty(), "Instruction should not be empty");
        assert!(instruction.contains("SessionSearch"), "Must mention SessionSearch");
        assert!(instruction.contains("D0"), "Must mention D0");
        assert!(instruction.contains("D1"), "Must mention D1");
        assert!(instruction.contains("D2"), "Must mention D2");
        assert!(instruction.contains("inject_summary"), "Must tell agent to call inject_summary");
    }

    // Scenario: on_injected callback fires immediately after DAG is stored
    #[test]
    fn test_on_injected_callback_fires() {
        let compaction_flag = Arc::new(AtomicBool::new(true));
        let pending_dag = Arc::new(std::sync::Mutex::new(None));
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_tokens = Arc::new(AtomicU32::new(0));

        let called_clone = callback_called.clone();
        let tokens_clone = callback_tokens.clone();
        let on_injected: OnInjectedCallback = Arc::new(move |tokens| {
            called_clone.store(true, Ordering::SeqCst);
            tokens_clone.store(tokens, Ordering::SeqCst);
        });

        let handler = create_handler(
            pending_dag.clone(),
            200_000,
            compaction_flag.clone(),
            Some(on_injected),
        );

        // @step When the agent calls inject_summary with DAG content
        let result = handler(Uuid::new_v4(), "# D2: Architecture\n- JWT auth".to_string());
        assert!(result.is_ok());

        // @step Then the on_injected callback should have been called
        assert!(callback_called.load(Ordering::SeqCst), "on_injected callback should fire");

        // @step And the injected token count should be greater than 0
        assert!(callback_tokens.load(Ordering::SeqCst) > 0, "injected tokens should be > 0");

        // @step And compaction_in_progress should already be false
        assert!(!compaction_flag.load(Ordering::Relaxed), "flag should be cleared before callback");
    }
}
