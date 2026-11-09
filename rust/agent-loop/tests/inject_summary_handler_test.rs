#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Feature: spec/features/inject-summary-handler.feature
//!
//! This test file validates the acceptance criteria for the inject_summary
//! handler in `codelet_agent_loop::inject_summary_handler`:
//! - apply_pending_dag with no pending content leaves the session untouched
//! - handler registration with codelet-tools and the handler type contract
//! - the real agent loop registers the session's handler for each turn and
//!   clears it when the turn ends
//!
//! Scenarios that asserted the legacy raw-DAG success path were dropped when
//! the handler became fail-closed (see uncaptured_compaction_contract.rs);
//! the feature marks them @deprecated (product owner, 2026-11-05).

#[path = "support/terminal_recovery.rs"]
pub mod support;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use codelet_agent_loop::agent_loop::agent_loop;
use codelet_agent_loop::inject_summary_handler;
use codelet_cli::compactor_sub_agent::has_compactor_sub_agent_handler;
use codelet_cli::session::system_reminders::{add_system_reminder, SystemReminderType};
use codelet_rpc_types::SessionStatus;
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::inject_summary::{
    set_inject_summary_handler, InjectSummaryHandler, InjectSummaryResult,
};
use codelet_tools::{
    has_generate_compaction_handler, has_inject_summary_handler, has_session_search_handler,
};
use rig::message::{AssistantContent, Message, UserContent};
use rig::OneOrMany;
use tokio::sync::{mpsc, Notify};
use uuid::Uuid;
use wiremock::{matchers::method, Mock, MockServer, Request, ResponseTemplate};

use codelet_cli::session::Session;

/// Helper to create a user message
fn create_user_message(content: &str) -> Message {
    Message::User {
        content: OneOrMany::one(UserContent::text(content)),
    }
}

/// Helper to create an assistant message
fn create_assistant_message(content: &str) -> Message {
    Message::Assistant {
        id: None,
        content: OneOrMany::one(AssistantContent::text(content)),
    }
}

/// Helper to create a system reminder message
fn create_system_reminder(reminder_type: SystemReminderType, content: &str) -> Message {
    let messages: Vec<Message> = vec![];
    let result = add_system_reminder(&messages, reminder_type, content);
    result.into_iter().next().unwrap()
}

/// Helper to create a session with system reminders and conversation messages
fn create_session_with_messages(
    num_system_reminders: usize,
    num_conversation_pairs: usize,
) -> Session {
    // Credential-free provider manager: these scenarios never call a model.
    let provider_manager = codelet_providers::ProviderManager::for_testing(
        codelet_providers::ProviderType::OpenAI,
        Some(200_000),
        Some(4096),
    );
    let mut session = Session::from_provider_manager(provider_manager);

    let reminder_types = [
        SystemReminderType::Environment,
        SystemReminderType::SystemPrompt,
        SystemReminderType::FspecWorkflow,
        SystemReminderType::GitStatus,
        SystemReminderType::TokenStatus,
    ];

    for i in 0..num_system_reminders {
        let reminder_type = reminder_types[i % reminder_types.len()];
        let content = format!("System reminder content #{}", i + 1);
        let reminder = create_system_reminder(reminder_type, &content);
        session.messages.push(reminder);
    }

    for i in 0..num_conversation_pairs {
        session
            .messages
            .push(create_user_message(&format!("User message #{}", i + 1)));
        session.messages.push(create_assistant_message(&format!(
            "Assistant response #{}",
            i + 1
        )));
    }

    session
}

// =============================================================================
// Scenario: apply_pending_dag with no pending content is a no-op
// =============================================================================

#[test]
fn test_apply_pending_dag_no_content() {
    let pending_dag = Arc::new(std::sync::Mutex::new(None));
    let mut session = create_session_with_messages(3, 10);
    let original_count = session.messages.len();

    let applied = inject_summary_handler::apply_pending_dag(&mut session, &pending_dag);

    assert!(applied.is_none(), "Should return None when no pending DAG");
    assert_eq!(
        session.messages.len(),
        original_count,
        "Messages should be unchanged"
    );
}

// =============================================================================
// Scenario: Handler registration and cleanup
// =============================================================================

#[test]
fn test_handler_registration_callable() {
    let session_id = Uuid::new_v4();
    let handler: InjectSummaryHandler = Arc::new(|_sid, _content| {
        Ok(InjectSummaryResult {
            injected_tokens: 100,
            remaining_budget: 199_900,
        })
    });

    set_inject_summary_handler(session_id, Some(handler));
    assert!(codelet_tools::inject_summary::has_inject_summary_handler(
        session_id
    ));

    set_inject_summary_handler(session_id, None);
    assert!(!codelet_tools::inject_summary::has_inject_summary_handler(
        session_id
    ));
}

// =============================================================================
// Scenario: create_handler returns correct InjectSummaryHandler type
// =============================================================================

#[test]
fn test_create_handler_returns_correct_type() {
    // @step Given a pending-DAG slot Arc<Mutex<Option<String>>>, a context_window of 200000 and a compaction flag
    let pending_dag = Arc::new(std::sync::Mutex::new(None));
    let context_window: u64 = 200_000;

    // @step When create_handler is called with those arguments
    // @step Then it should return an InjectSummaryHandler (Arc<dyn Fn(Uuid, String) -> Result<InjectSummaryResult, String> + Send + Sync>)
    let handler: InjectSummaryHandler = inject_summary_handler::create_handler(
        pending_dag,
        context_window,
        Arc::new(AtomicBool::new(false)),
        None,
    );

    // @step And the handler should be callable with a session_id and content string
    let result = handler(Uuid::new_v4(), "test content".to_string());
    let _ = result;
}

// =============================================================================
// Real agent loop: per-turn registration and cleanup
// =============================================================================

/// Mounts an OpenAI-compatible provider that signals when the turn's request
/// arrives and answers with `response`, then selects it on the session.
async fn select_signalling_provider(
    session: &BackgroundSession,
    response: ResponseTemplate,
) -> (MockServer, Arc<Notify>) {
    let server = MockServer::start().await;
    let entered = Arc::new(Notify::new());
    let provider_entered = entered.clone();
    Mock::given(method("POST"))
        .respond_with(move |_: &Request| {
            provider_entered.notify_one();
            response.clone()
        })
        .mount(&server)
        .await;
    std::env::set_var("OPENAI_API_KEY", "test-no-network");
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    let mut inner = session.inner.lock().await;
    inner.provider_manager_mut().set_facade_override(None);
    inner
        .provider_manager_mut()
        .set_model_direct("openai", "o3", Some(100_000), Some(4096), None)
        .expect("select the signalling provider");
    drop(inner);
    (server, entered)
}

// =============================================================================
// Scenario: Handler is registered alongside SessionSearch handler in agent loop setup
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn test_agent_loop_registers_inject_summary_handler_with_turn_handlers() {
    // @step Given a BackgroundSession is being set up for an agent run
    let (_directory, session, input_rx) = support::fixture();
    let (_server, entered) = select_signalling_provider(
        &session,
        ResponseTemplate::new(200).set_delay(Duration::from_secs(60)),
    )
    .await;
    assert!(!has_inject_summary_handler(session.id));

    // @step When the agent loop registers handlers
    let (_injection_tx, injection_rx) = mpsc::channel(4);
    let task = tokio::spawn(agent_loop(session.clone(), input_rx, injection_rx));
    session
        .send_input("register".into(), None)
        .expect("queue turn");
    codelet_test_helpers::deadlock_guard("entered.notified()", entered.notified())
        .await
        .expect("the turn's provider request starts");

    // @step Then set_inject_summary_handler should be called with the session ID and a handler closure
    let registered = has_inject_summary_handler(session.id);

    // @step And it is registered alongside the GenerateCompaction and SessionSearch handlers for the same session
    let generate = has_generate_compaction_handler(session.id);
    let search = has_session_search_handler(session.id);
    task.abort();
    let _ = task.await;
    assert!(
        registered,
        "the turn must register the inject_summary handler"
    );
    assert!(generate, "GenerateCompaction must be registered with it");
    assert!(search, "SessionSearch must be registered with it");
}

// =============================================================================
// Scenario: Handler is cleaned up on session teardown
// =============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn test_agent_loop_clears_inject_summary_handler_when_the_run_ends() {
    // @step Given a BackgroundSession has completed its agent run
    let (_directory, session, input_rx) = support::fixture();
    let (_server, entered) = select_signalling_provider(
        &session,
        ResponseTemplate::new(400)
            .set_delay(Duration::from_millis(500))
            .set_body_string("rejected by the test provider"),
    )
    .await;
    let (_injection_tx, injection_rx) = mpsc::channel(4);
    let task = tokio::spawn(agent_loop(session.clone(), input_rx, injection_rx));
    session
        .send_input("finish".into(), None)
        .expect("queue turn");
    codelet_test_helpers::deadlock_guard("entered.notified()", entered.notified())
        .await
        .expect("the turn's provider request starts");
    let registered_during_run = has_inject_summary_handler(session.id);

    // @step When the session teardown cleanup executes
    let cleaned = codelet_test_helpers::deadlock_guard("the awaited block", async {
        while has_inject_summary_handler(session.id) || session.get_status() != SessionStatus::Idle
        {
            codelet_test_helpers::yield_now().await;
        }
    })
    .await;
    let loop_still_running = !task.is_finished();

    // @step Then set_inject_summary_handler should be called with the session ID and None
    let inject_cleared = !has_inject_summary_handler(session.id);

    // @step And the cleanup occurs alongside the compactor sub-agent and GenerateCompaction handler cleanup
    let compactor_cleared = !has_compactor_sub_agent_handler(session.id);
    let generate_cleared = !has_generate_compaction_handler(session.id);
    task.abort();
    let _ = task.await;
    assert!(registered_during_run, "the run must register the handler");
    assert!(cleaned.is_ok(), "the run must end and clear its handlers");
    assert!(
        loop_still_running,
        "the agent loop itself clears the handlers, not task teardown"
    );
    assert!(inject_cleared);
    assert!(compactor_cleared);
    assert!(generate_cleared);
}
