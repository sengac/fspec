//! BRIDGE-008: Telegram stream control channel.
//!
//! Feature: spec/features/telegram-stream-control-channel.feature
//!
//! Each scenario runs a real session whose agent loop registers the real
//! bridge handlers (`FspecAgentHooks`) while a turn runs. Like the relay,
//! which takes the session's handlers when the Bridge tool connects during
//! a turn and keeps them for the connection, the fixture captures them
//! during a setup turn. Inbound `session` envelopes then go through
//! `codelet_tools::handle_multiplexed_inbound`, the relay's entry point.

#[path = "cmpct060/support.rs"]
pub mod support;

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use codelet_rpc_types::{InterruptionReason, SessionState, SessionStatus, StreamChunk};
use codelet_sessions::background_session::BackgroundSession;
use codelet_sessions::SessionManager;
use rig::message::Message;
use serde_json::json;
use wiremock::{matchers::method, Mock, MockServer, Request, ResponseTemplate};

type TestResult = support::TestResult;

/// Captures formatted tracing output for the unknown-action warning.
#[derive(Clone, Default)]
struct CapturedLog(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for CapturedLog {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for CapturedLog {
    type Writer = CapturedLog;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Restores the OpenAI env vars the provider mock redirects.
struct OpenAiEnv(Vec<(&'static str, Option<std::ffi::OsString>)>);

impl OpenAiEnv {
    fn point_at(server: &MockServer) -> Self {
        let previous = ["OPENAI_API_KEY", "OPENAI_BASE_URL"]
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        std::env::set_var("OPENAI_API_KEY", "offline");
        std::env::set_var("OPENAI_BASE_URL", server.uri());
        Self(previous)
    }
}

impl Drop for OpenAiEnv {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

struct Connected {
    _directory: support::TestDirectory,
    _env: OpenAiEnv,
    manager: Arc<SessionManager>,
    session: Arc<BackgroundSession>,
    input_injector: codelet_tools::InputInjector,
    control_handler: codelet_tools::ControlHandler,
    bodies: Arc<Mutex<Vec<String>>>,
}

impl Drop for Connected {
    fn drop(&mut self) {
        self.session.interrupt();
        codelet_tools::remove_bridge_session_context(self.session.id);
    }
}

/// Scripted OpenAI provider: a request whose body contains a marker gets
/// that marker's reply after its delay; any other request (the setup turn)
/// gets "bridge008 setup reply" after one second, long enough for the
/// fixture to capture the turn's bridge handlers. Every body is recorded.
async fn provider(
    scripted: Vec<(&'static str, Duration, &'static str)>,
) -> (MockServer, Arc<Mutex<Vec<String>>>) {
    let server = MockServer::start().await;
    let bodies: Arc<Mutex<Vec<String>>> = Arc::default();
    let seen = bodies.clone();
    Mock::given(method("POST"))
        .respond_with(move |request: &Request| {
            let body = String::from_utf8_lossy(&request.body).into_owned();
            let (delay, text) = scripted
                .iter()
                .find(|(marker, _, _)| body.contains(marker))
                .map_or(
                    (Duration::from_secs(1), "bridge008 setup reply"),
                    |(_, delay, text)| (*delay, *text),
                );
            seen.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(body);
            text_reply(text).set_delay(delay)
        })
        .mount(&server)
        .await;
    (server, bodies)
}

/// A session driven by the real agent loop whose bridge handlers were
/// captured during a finished setup turn (the first scripted reply).
async fn connected_session(
    server: &MockServer,
    bodies: Arc<Mutex<Vec<String>>>,
) -> Result<Connected, Box<dyn std::error::Error>> {
    let (directory, manager, _, _) = support::manager().await?;
    let env = OpenAiEnv::point_at(server);
    manager.set_hooks(Arc::new(codelet_agent_loop::FspecAgentHooks::new()));
    let id = manager
        .create_session(
            "anthropic/claude-sonnet-4",
            &directory.path().to_string_lossy(),
        )
        .await?;
    let session = manager.get_session(&id)?;
    session
        .inner
        .lock()
        .await
        .provider_manager_mut()
        .set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    session.send_input("bridge008 setup".into(), None)?;
    let mut found = None;
    wait_until("the bridge session context exists", || {
        found = codelet_tools::get_bridge_session_context(session.id);
        found.is_some()
    })
    .await?;
    let context = found.ok_or("the bridge session context exists")?;
    let control_handler = context
        .control_handler
        .clone()
        .ok_or("the agent loop registers a bridge control handler")?;
    let input_injector = context.input_injector.clone();
    wait_until("the setup turn finishes", || {
        session.get_status() == SessionStatus::Idle
            && codelet_tools::get_bridge_session_context(session.id).is_none()
    })
    .await?;
    Ok(Connected {
        _directory: directory,
        _env: env,
        manager,
        session,
        input_injector,
        control_handler,
        bodies,
    })
}

/// Sends one inbound `session` envelope through the relay's handler, with
/// the connection's captured handlers as its fallbacks.
async fn bridge_receives(
    connected: &Connected,
    msg_type: &str,
    data: serde_json::Value,
) -> Result<Option<codelet_tools::bridge_multiplexed::Envelope>, String> {
    let envelope = json!({
        "service": "session",
        "type": msg_type,
        "session_id": connected.session.id.to_string(),
        "data": data,
    });
    codelet_tools::handle_multiplexed_inbound(
        &envelope.to_string(),
        connected.session.id,
        connected.input_injector.clone(),
        Some(connected.control_handler.clone()),
        None,
        None,
    )
    .await
}

fn text_reply(text: &str) -> ResponseTemplate {
    let chunk = json!({"id": "bridge008", "object": "chat.completion.chunk", "created": 1,
        "model": "o3", "choices": [{"index": 0, "delta": {"role": "assistant", "content": text},
        "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 50, "completion_tokens": 5, "total_tokens": 55}});
    ResponseTemplate::new(200)
        .insert_header("content-type", "text/event-stream")
        .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
}

async fn wait_until(
    what: &str,
    mut done: impl FnMut() -> bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // TEST-042: the state is polled between yields, under the deadlock guard.
    codelet_test_helpers::wait_until(what, || std::future::ready(done()))
        .await
        .map_err(Into::into)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_handle_interrupt_control_message() -> TestResult {
    // @step Given the bridge is connected to a session
    let (server, bodies) = provider(vec![(
        "work on something long",
        Duration::from_secs(60),
        "never delivered",
    )])
    .await;
    let connected = connected_session(&server, bodies).await?;
    let session = &connected.session;

    // @step And the agent is processing a request
    session.send_input("work on something long".into(), None)?;
    wait_until("the long request reaches the provider", || {
        connected
            .bodies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .any(|body| body.contains("work on something long"))
    })
    .await?;
    assert_eq!(session.get_status(), SessionStatus::Running);

    // @step When the bridge receives a message with type "control" and action "interrupt"
    let reply = bridge_receives(&connected, "control", json!({"action": "interrupt"})).await?;
    assert!(reply.is_none());

    // @step Then the agent should stop processing
    wait_until("the interrupted turn stops", || {
        session.get_status() != SessionStatus::Running
    })
    .await?;

    // @step And the is_interrupted flag should be set to true
    assert!(session.is_interrupted.load(Ordering::Acquire));
    assert_eq!(
        session.interruption_reason(),
        Some(InterruptionReason::User)
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_handle_clear_control_message() -> TestResult {
    // @step Given the bridge is connected to a session
    let (server, bodies) = provider(Vec::new()).await;
    let connected = connected_session(&server, bodies).await?;
    let session = &connected.session;

    // @step And the session has conversation history
    {
        let mut inner = session.inner.lock().await;
        inner.messages.extend([
            Message::user("bridge008 earlier question"),
            Message::assistant("bridge008 earlier answer"),
        ]);
        inner.token_tracker.input_tokens = 4_321;
        let history = serde_json::to_string(&inner.messages)?;
        assert!(history.contains("bridge008 setup reply"));
    }
    session.cached_input_tokens.store(4_321, Ordering::Release);
    let mut events = session.subscribe_to_stream();

    // @step When the bridge receives a message with type "control" and action "clear"
    let reply = bridge_receives(&connected, "control", json!({"action": "clear"})).await?;
    assert!(reply.is_none());

    // @step Then the session should be reset
    let mut cleared = false;
    while let Ok(event) = events.try_recv() {
        cleared |= matches!(
            event,
            StreamChunk::SessionStateChange {
                state: SessionState::Cleared
            }
        );
    }
    assert!(
        cleared,
        "a Cleared session state change should be published"
    );
    assert_eq!(session.cached_input_tokens.load(Ordering::Acquire), 0);

    // @step And the conversation history should be cleared
    let inner = session.inner.lock().await;
    let history = serde_json::to_string(&inner.messages)?;
    assert!(
        !history.contains("bridge008"),
        "earlier turns must be gone: {history}"
    );
    assert_eq!(inner.token_tracker.input_tokens, 0);
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_handle_unknown_control_action_gracefully() -> TestResult {
    // @step Given the bridge is connected to a session
    let (server, bodies) = provider(Vec::new()).await;
    let connected = connected_session(&server, bodies).await?;
    let session = &connected.session;
    let log = CapturedLog::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(log.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::WARN)
        .finish();
    let guard = tracing::subscriber::set_default(subscriber);

    // @step When the bridge receives a message with type "control" and action "unknown"
    let reply = bridge_receives(&connected, "control", json!({"action": "unknown"})).await;
    drop(guard);

    // @step Then a warning naming the unknown action should be logged
    let text = String::from_utf8(log.0.lock().unwrap_or_else(PoisonError::into_inner).clone())?;
    assert!(
        text.lines()
            .any(|line| line.contains("WARN") && line.contains("unknown action 'unknown'")),
        "expected an unknown-action warning, got: {text}"
    );

    // @step And the bridge should not crash
    assert!(matches!(reply, Ok(None)), "got {reply:?}");

    // @step And the session should remain active
    assert!(!session.is_interrupted.load(Ordering::Acquire));
    assert_eq!(session.get_status(), SessionStatus::Idle);
    assert!(connected
        .manager
        .get_session(&session.id.to_string())
        .is_ok());
    let history = serde_json::to_string(&session.inner.lock().await.messages)?;
    assert!(history.contains("bridge008 setup reply"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn scenario_forward_input_messages_to_agent() -> TestResult {
    // @step Given the bridge is connected to a session
    let (server, bodies) = provider(vec![(
        "bridge008 telegram question",
        Duration::ZERO,
        "bridge008 reply",
    )])
    .await;
    let connected = connected_session(&server, bodies).await?;
    let session = &connected.session;

    // @step When the bridge receives a message with type "input"
    let reply = bridge_receives(
        &connected,
        "input",
        json!({"message": "bridge008 telegram question"}),
    )
    .await?;
    assert!(reply.is_none());

    // @step Then the message should be forwarded to the agent
    wait_until("the provider sees the forwarded message", || {
        connected
            .bodies
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .any(|body| body.contains("bridge008 telegram question"))
    })
    .await?;

    // @step And the agent should process the input
    let session = &session;
    codelet_test_helpers::wait_until("the agent's reply is part of the history", || async move {
        serde_json::to_string(&session.inner.lock().await.messages)
            .is_ok_and(|history| history.contains("bridge008 reply"))
    })
    .await?;
    Ok(())
}
