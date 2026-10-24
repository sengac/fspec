#[path = "model.rs"]
mod model;
#[path = "retry.rs"]
mod retry;

use codelet_cli::compactor_sub_agent::{
    set_compactor_sub_agent_handler, set_terminal_overflow_owner,
};
use codelet_cli::interactive::{run_agent_stream, StreamEvent, StreamOutput};
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_core::RigAgent;
use codelet_providers::{ProviderManager, ProviderType};
use codelet_rpc_types::{CommittedContextSnapshot, RequestContextSnapshot};
use rig::{client::CompletionClient, message::Message};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, PoisonError,
};
use std::time::Duration;
use tokio::sync::Notify;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;
pub const DAG: &str = "<dag-node depth=\"D1\" turns=\"0-1\" label=\"Work\">Recovered task context.</dag-node>\n<dag-files>\n</dag-files>";

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    PrePrompt,
    Hook,
    Late,
    Admission,
    Published,
}

pub struct Output {
    pub events: Mutex<Vec<StreamEvent>>,
    pub interrupted: Arc<AtomicBool>,
    pub mode: Mode,
}

impl StreamOutput for Output {
    fn emit(&self, event: StreamEvent) {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }

    fn publish_committed_context(
        &self,
        identity: &codelet_core::persistence::compaction_boundary::CompactionIdentity,
        _snapshot: CommittedContextSnapshot,
        _context: RequestContextSnapshot,
    ) -> Result<(), String> {
        assert!(load_session(identity.session_id)?
            .compaction_commit
            .is_some());
        if self.mode == Mode::Published {
            self.interrupted.store(true, Ordering::Release);
        }
        Ok(())
    }
}

pub struct Fixture {
    pub session: Session,
    pub manifest: SessionManifest,
    pub original: serde_json::Value,
    pub interrupted: Arc<AtomicBool>,
    pub notify: Arc<Notify>,
    pub started: Arc<Notify>,
    pub dropped: Arc<AtomicBool>,
    pub busy: Arc<AtomicBool>,
    pub output: Output,
    pub server: MockServer,
    agent: Option<RigAgent<model::InterruptingModel>>,
    _directory: tempfile::TempDir,
}

struct DropSignal(Arc<AtomicBool>);
impl Drop for DropSignal {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl Fixture {
    pub async fn new(mode: Mode) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_common::set_data_directory(directory.path().to_owned())?;
        reset_stores_for_tests();
        let mut manifest =
            create_session_with_provider("cancel compaction", directory.path(), "openai")?;
        let history = if mode == Mode::Hook {
            "oversized conversation history ".repeat(6_000)
        } else {
            "original question".into()
        };
        append_message(&mut manifest, "user", &history)?;
        append_message(&mut manifest, "assistant", "work in progress")?;
        let original = serde_json::to_value(load_session(manifest.id)?)?;
        let mut manager =
            ProviderManager::for_testing(ProviderType::OpenAI, Some(16_000), Some(4096));
        manager.set_model_direct("openai", "selected-model", Some(16_000), Some(4096), None)?;
        let mut session = Session::from_provider_manager(manager);
        session.messages = vec![
            Message::user(history),
            Message::assistant("work in progress"),
        ];
        if mode != Mode::Hook {
            session.token_tracker.input_tokens = 15_000;
        }
        let interrupted = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(Notify::new());
        let started = Arc::new(Notify::new());
        let dropped = Arc::new(AtomicBool::new(false));
        let signal = started.clone();
        let cancellation = interrupted.clone();
        let wakeup = notify.clone();
        let drop_signal = dropped.clone();
        set_compactor_sub_agent_handler(
            manifest.id,
            Some(Arc::new(move |_, _| {
                let entered = signal.clone();
                let cancelled = cancellation.clone();
                let notify = wakeup.clone();
                let dropped = drop_signal.clone();
                Box::pin(async move {
                    let _guard = DropSignal(dropped);
                    entered.notify_one();
                    match mode {
                        Mode::PrePrompt | Mode::Hook => std::future::pending().await,
                        Mode::Late => {
                            cancelled.store(true, Ordering::Release);
                            notify.notify_one();
                            Ok(DAG.into())
                        }
                        _ => Ok(DAG.into()),
                    }
                })
            })),
        );
        set_terminal_overflow_owner(manifest.id, true);
        let server = MockServer::start().await;
        let chunk = serde_json::json!({"id":"unexpected-continuation","object":"chat.completion.chunk","created":1,"model":"selected-model","choices":[{"index":0,"delta":{"role":"assistant","content":"resumed"},"finish_reason":"stop"}],"usage":{"prompt_tokens":100,"completion_tokens":1,"total_tokens":101}});
        Mock::given(method("POST"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n")),
            )
            .mount(&server)
            .await;
        let client: rig::providers::openai::CompletionsClient =
            rig::providers::openai::CompletionsClient::builder()
                .api_key("offline")
                .base_url(server.uri())
                .build()?;
        let agent = RigAgent::new(
            rig::agent::AgentBuilder::new(model::InterruptingModel {
                inner: client.completion_model("selected-model"),
                interrupted: interrupted.clone(),
                during_admission: mode == Mode::Admission,
            })
            .max_tokens(4096)
            .build(),
            1,
        );
        Ok(Self {
            session,
            manifest,
            original,
            interrupted: interrupted.clone(),
            notify,
            started,
            dropped,
            busy: Arc::new(AtomicBool::new(false)),
            output: Output {
                events: Mutex::new(Vec::new()),
                interrupted,
                mode,
            },
            server,
            agent: Some(agent),
            _directory: directory,
        })
    }

    pub async fn run(
        &mut self,
        cancel_pending: bool,
    ) -> Result<anyhow::Result<()>, Box<dyn std::error::Error>> {
        let started = self.started.clone();
        let flag = self.interrupted.clone();
        let wakeup = self.notify.clone();
        let agent = self.agent.take().ok_or("fixture already executed")?;
        let work = run_agent_stream(
            agent,
            "resume original task",
            &mut self.session,
            self.interrupted.clone(),
            self.busy.clone(),
            self.notify.clone(),
            &self.output,
            self.manifest.id,
        );
        let cancel = async move {
            if cancel_pending {
                tokio::time::timeout(Duration::from_secs(3), started.notified()).await?;
                flag.store(true, Ordering::Release);
                wakeup.notify_one();
            }
            Ok::<(), tokio::time::error::Elapsed>(())
        };
        let (result, cancellation) =
            tokio::join!(tokio::time::timeout(Duration::from_secs(2), work), cancel);
        cancellation?;
        assert!(
            result.is_ok(),
            "user cancellation must not wait for the builder deadline"
        );
        Ok(result?)
    }

    pub async fn assert_no_success_or_continuation(&self) -> TestResult {
        assert!(self
            .server
            .received_requests()
            .await
            .ok_or("request log")?
            .is_empty());
        assert!(!self.busy.load(Ordering::Acquire));
        assert!(!self
            .output
            .events
            .lock()
            .map_err(|error| error.to_string())?
            .iter()
            .any(|event| matches!(event, StreamEvent::CompactionComplete(_))));
        Ok(())
    }

    pub fn assert_cancelled(&self, result: anyhow::Result<()>) -> TestResult {
        let error = result.err().ok_or("cancelled operation cannot succeed")?;
        assert!(
            matches!(
                error.downcast_ref::<codelet_cli::compaction_operation::CompactionFailure>(),
                Some(codelet_cli::compaction_operation::CompactionFailure::Cancelled)
            ),
            "{error:?}"
        );
        assert!(self.dropped.load(Ordering::Acquire));
        assert_eq!(
            serde_json::to_value(load_session(self.manifest.id)?)?,
            self.original
        );
        Ok(())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        set_compactor_sub_agent_handler(self.manifest.id, None);
        set_terminal_overflow_owner(self.manifest.id, false);
    }
}
