use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use codelet_cli::compaction_operation::CompactionFailure;
use codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler;
use codelet_cli::interactive::run_agent_stream;
use codelet_cli::session::Session;
use codelet_core::persistence::{get_message, load_session};
use codelet_core::RigAgent;
use codelet_fspec_tui::ViewMode;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::{SessionStatus, StreamChunk};
use rig::client::CompletionClient;
use tokio::io::AsyncWriteExt;
use tokio::sync::oneshot;

use crate::progress_support::{Fixture, TestResult};

struct PendingBuilder(Arc<AtomicBool>);

impl Drop for PendingBuilder {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

pub struct EscapeFixture {
    pub shared: Fixture,
    task: Option<tokio::task::JoinHandle<(bool, String)>>,
    durable: serde_json::Value,
    originals: serde_json::Value,
    messages: serde_json::Value,
    requests: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
    _server_shutdown: oneshot::Sender<()>,
}

impl EscapeFixture {
    pub async fn new(websocket: bool) -> TestResult<Self> {
        let shared = if websocket {
            Fixture::websocket().await?
        } else {
            Fixture::new().await?
        };
        let manifest = load_session(shared.sessions[0].id)?;
        let originals = manifest
            .messages
            .iter()
            .map(|reference| get_message(reference.message_id))
            .collect::<Result<Vec<_>, _>>()?;
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(16_000), Some(4096));
        manager.set_model_direct("openai", "selected-model", Some(16_000), Some(4096), None)?;
        let messages = {
            let mut session = shared.sessions[0].inner.lock().await;
            let messages = std::mem::take(&mut session.messages);
            *session = Session::from_provider_manager(manager);
            session.messages = messages;
            session.token_tracker.input_tokens = 15_000;
            serde_json::to_value(&session.messages)?
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let requests = Arc::new(AtomicUsize::new(0));
        let received = requests.clone();
        let (shutdown, mut stopped) = oneshot::channel::<()>();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stopped => break,
                    accepted = listener.accept() => {
                        let Ok((mut socket, _)) = accepted else { break };
                        received.fetch_add(1, Ordering::AcqRel);
                        let _ = socket.write_all(
                            b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                        ).await;
                    }
                }
            }
        });
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline")
                .base_url(format!("http://{address}"))
                .build()?;
        let agent = RigAgent::new(client.agent("selected-model").max_tokens(4096).build(), 1);
        let mut fixture = Self {
            shared,
            task: None,
            durable: serde_json::to_value(&manifest)?,
            originals: serde_json::to_value(originals)?,
            messages,
            requests,
            calls: Arc::new(AtomicUsize::new(0)),
            dropped: Arc::new(AtomicBool::new(false)),
            _server_shutdown: shutdown,
        };
        let (entered, ready) = oneshot::channel();
        let entered = Arc::new(Mutex::new(Some(entered)));
        let calls = fixture.calls.clone();
        let dropped = fixture.dropped.clone();
        set_compactor_sub_agent_handler(
            fixture.shared.sessions[0].id,
            Some(Arc::new(move |_, _| {
                calls.fetch_add(1, Ordering::AcqRel);
                let entered = entered.clone();
                let dropped = dropped.clone();
                Box::pin(async move {
                    let _pending = PendingBuilder(dropped);
                    if let Some(sender) = entered
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .take()
                    {
                        let _ = sender.send(());
                    }
                    std::future::pending().await
                })
            })),
        );
        fixture.shared.seed_visible_progress();
        let session = fixture.shared.sessions[0].clone();
        let output = fixture.shared.output(0);
        fixture.task = Some(tokio::spawn(async move {
            let mut inner = session.inner.lock().await;
            let result = run_agent_stream(
                agent,
                "resume original task",
                &mut inner,
                session.is_interrupted.clone(),
                session.compaction_in_progress.clone(),
                session.interrupt_notify.clone(),
                &output,
                session.id,
            )
            .await;
            match result {
                Err(error) => (
                    matches!(
                        error.downcast_ref::<CompactionFailure>(),
                        Some(CompactionFailure::Cancelled)
                    ),
                    error.to_string(),
                ),
                Ok(()) => (
                    false,
                    "stream completed successfully instead of cancelling".into(),
                ),
            }
        }));
        tokio::time::timeout(Duration::from_secs(3), ready).await??;
        fixture.shared.flush().await?;
        fixture.shared.app.navigator_mut().active_view = ViewMode::Agent;
        assert!(fixture.shared.sessions[0]
            .compaction_in_progress
            .load(Ordering::Acquire));
        assert_eq!(
            fixture
                .shared
                .app
                .agent_view_store()
                .session_status_for(&fixture.shared.ids[0]),
            Some(&SessionStatus::Compacting)
        );
        Ok(fixture)
    }

    pub fn assert_no_transient_ui(&self) {
        let app = &self.shared.app;
        assert!(app.compositor().is_empty());
        let agent = &app.navigator().agent;
        assert!(agent.slash_popup.is_none());
        assert!(agent.file_popup.is_none());
        assert!(agent.resume_view.is_none());
        assert!(agent.search_view.is_none());
        assert!(!agent.turn_select_mode);
        assert!(agent.input.value().is_empty());
    }

    pub async fn drain_keypress(&mut self) -> TestResult {
        while let Some(action) = self.shared.app.try_recv_action() {
            self.shared.app.dispatch(action);
        }
        while let Some(task) = self.shared.app.next_pending_task() {
            tokio::time::timeout(Duration::from_secs(3), task).await??;
        }
        Ok(())
    }

    pub async fn assert_stopped(&mut self) -> TestResult {
        assert!(
            self.shared.sessions[0]
                .is_interrupted
                .load(Ordering::Acquire),
            "real transport must deliver Escape interrupt"
        );
        let task = self.task.take().ok_or("pending pre-prompt stream task")?;
        let (cancelled, outcome) = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect(
            "Escape reached the session but pending pre-prompt compaction did not stop promptly",
        )?;
        assert!(cancelled, "expected explicit cancellation, got {outcome}");
        assert!(
            self.dropped.load(Ordering::Acquire),
            "pending builder future must be dropped by cancellation"
        );
        self.shared.flush().await?;
        Ok(())
    }

    pub fn assert_no_success_or_continuation(&self) {
        assert_eq!(self.calls.load(Ordering::Acquire), 1);
        assert_eq!(self.requests.load(Ordering::Acquire), 0);
        assert!(!self.shared.sessions[0]
            .get_buffered_output(1000)
            .iter()
            .any(|chunk| matches!(chunk, StreamChunk::CompactionComplete { .. })));
    }

    pub async fn assert_conversation_preserved(&self) -> TestResult {
        let manifest = load_session(self.shared.sessions[0].id)?;
        assert_eq!(serde_json::to_value(&manifest)?, self.durable);
        let originals = manifest
            .messages
            .iter()
            .map(|reference| get_message(reference.message_id))
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(serde_json::to_value(originals)?, self.originals);
        let session = self.shared.sessions[0].inner.lock().await;
        assert_eq!(serde_json::to_value(&session.messages)?, self.messages);
        assert_eq!(session.token_tracker.input_tokens, 15_000);
        assert_eq!(
            self.shared.app.agent_view_store().current_session(),
            Some(&self.shared.ids[0])
        );
        assert_eq!(self.shared.app.navigator().active_view, ViewMode::Agent);
        Ok(())
    }

    pub fn assert_activity_cleared(&self) {
        assert!(!self.shared.sessions[0]
            .compaction_in_progress
            .load(Ordering::Acquire));
        assert!(self.shared.sessions[0].get_compaction_progress().is_none());
        assert_ne!(
            self.shared.sessions[0].get_status(),
            SessionStatus::Compacting
        );
        assert_ne!(
            self.shared
                .app
                .agent_view_store()
                .session_status_for(&self.shared.ids[0]),
            Some(&SessionStatus::Compacting)
        );
        assert!(!self.shared.footer(0).contains("[compacting:"));
        let other = self.shared.sessions[1]
            .get_compaction_progress()
            .expect("other session progress remains visible");
        assert_eq!(other.phase, "Summarising");
        assert!(!self.shared.sessions[1]
            .is_interrupted
            .load(Ordering::Acquire));
        assert!(self.shared.footer(1).contains("[compacting:"));
    }
}
