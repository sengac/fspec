use std::sync::Arc;
use std::time::Duration;

use codelet_agent_loop::BackgroundOutput;
use codelet_cli::compactor_sub_agent::set_compactor_sub_agent_handler;
use codelet_core::session_manager_handle::SessionManagerHandle;
use codelet_core::work_units::WorkUnitsWatcher;
use codelet_fspec_tui::views::agent::chrome_paint::{paint_footer, ChromeAreas};
use codelet_fspec_tui::{Action, App, EmbeddedFspecBackend, FspecBackend, WebSocketFspecBackend};
use codelet_rpc::SharedFspecService;
use codelet_rpc_types::{CompactionProgress, NotificationSeverity, SessionId, StreamChunk};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use ratatui::{buffer::Buffer, layout::Rect};
use tokio::sync::oneshot;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct Fixture {
    pub app: App,
    pub sessions: Vec<Arc<BackgroundSession>>,
    pub ids: Vec<SessionId>,
    _directory: tempfile::TempDir,
    _manager: Arc<SessionManager>,
    server: Option<tokio::task::JoinHandle<()>>,
}

impl Fixture {
    pub async fn new() -> TestResult<Self> {
        Self::with_transport(false).await
    }

    pub async fn websocket() -> TestResult<Self> {
        Self::with_transport(true).await
    }

    async fn with_transport(websocket: bool) -> TestResult<Self> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        std::fs::create_dir_all(directory.path().join("cache"))?;
        std::fs::write(
            directory.path().join("cache/models.json"),
            include_str!("../fixtures/prov101_models.json"),
        )?;
        std::fs::create_dir_all(directory.path().join("spec"))?;
        std::fs::write(
            directory.path().join("spec/work-units.json"),
            r#"{"workUnits":{}}"#,
        )?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().into())?;
        std::env::set_var("ANTHROPIC_API_KEY", "bug210-no-network");
        let manager = Arc::new(SessionManager::new());
        manager.set_default_model("anthropic/claude-opus-4-5");
        let handle: Arc<dyn SessionManagerHandle> = manager.clone();
        let ids = vec![handle.create_session(None), handle.create_session(None)];
        let mut sessions = Vec::new();
        for id in &ids {
            let session = manager.get_session(&id.value)?;
            let mut manifest = codelet_core::persistence::load_session(session.id)?;
            codelet_core::persistence::append_message(&mut manifest, "user", "unfinished task")?;
            codelet_core::persistence::append_message(&mut manifest, "assistant", "working")?;
            session.inner.lock().await.messages = vec![
                rig::message::Message::user("unfinished task"),
                rig::message::Message::assistant("working"),
            ];
            sessions.push(session);
        }
        let watcher = Arc::new(WorkUnitsWatcher::new(directory.path())?);
        let service = Arc::new(
            SharedFspecService::with_session_manager(watcher, handle)
                .with_cwd(directory.path().into()),
        );
        let mut server = None;
        let backend: Arc<dyn FspecBackend> = if websocket {
            let (address, _, task) =
                codelet_rpc_server::bind_and_serve("127.0.0.1:0", service).await?;
            server = Some(task);
            let url = url::Url::parse(&format!("ws://{address}/"))?;
            Arc::new(WebSocketFspecBackend::connect(url).await?)
        } else {
            Arc::new(EmbeddedFspecBackend::new(
                tokio::runtime::Handle::current(),
                service,
            ))
        };
        let mut app = App::new(backend);
        app.dispatch(Action::Reconnected);
        while let Some(task) = app.next_pending_task() {
            task.await?;
        }
        while let Some(action) = app.try_recv_action() {
            app.dispatch(action);
        }
        app.agent_view_store_mut().open_sessions_mut().clear();
        for id in &ids {
            app.agent_view_store_mut()
                .append_session(codelet_fspec_tui::SessionContext::new(id.clone()));
        }
        app.agent_view_store_mut().focus_session_index(0);
        Ok(Self {
            app,
            sessions,
            ids,
            _directory: directory,
            _manager: manager,
            server,
        })
    }

    pub fn output(&self, index: usize) -> BackgroundOutput {
        BackgroundOutput::with_provider(self.sessions[index].clone(), "claude".into(), None)
    }

    pub async fn flush(&mut self) -> TestResult {
        let marker = "bug210-progress-stream-barrier";
        for session in &self.sessions {
            session.handle_output(StreamChunk::user_notification(
                marker.into(),
                NotificationSeverity::Info,
            ));
        }
        let mut pending_sessions = self.ids.clone();
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let Some(action) = self.app.try_recv_action() {
                    if let Action::ChunkReceived(
                        _,
                        chunk @ StreamChunk::CompactionProgressUpdate { progress },
                    ) = &action
                    {
                        let json =
                            codelet_agent_loop::stream_chunk_json::stream_chunk_to_json_value(
                                chunk,
                            );
                        assert_eq!(json["type"], "compactionProgressUpdate");
                        assert_eq!(json["progress"], serde_json::to_value(progress)?);
                    }
                    if let Action::ChunkReceived(
                        session,
                        StreamChunk::UserNotification { message, .. },
                    ) = &action
                    {
                        if message == marker {
                            pending_sessions.retain(|pending| pending != session);
                        }
                    }
                    self.app.dispatch(action);
                    if pending_sessions.is_empty() {
                        break Ok::<(), Box<dyn std::error::Error + Send + Sync>>(());
                    }
                } else {
                    tokio::task::yield_now().await;
                }
            }
        })
        .await??;
        Ok(())
    }

    pub fn footer(&self, index: usize) -> String {
        let areas = ChromeAreas {
            header: Rect::default(),
            menu: Rect::default(),
            role: Rect::default(),
            scrollback: Rect::default(),
            footer: Rect::new(0, 0, 120, 1),
            input: Rect::default(),
        };
        let mut buffer = Buffer::empty(areas.footer);
        paint_footer(
            &areas,
            &mut buffer,
            self.app.agent_view_store(),
            Some(&self.ids[index]),
        );
        (0..120)
            .map(|column| buffer[(column, 0)].symbol())
            .collect()
    }

    pub fn seed_visible_progress(&mut self) {
        for index in 0..2 {
            self.sessions[index].update_compaction_progress("Summarising".into(), 1, 4);
            self.app.agent_view_store_mut().set_compaction_progress(
                self.ids[index].clone(),
                CompactionProgress {
                    phase: "Summarising".into(),
                    current: 1,
                    total: 4,
                },
            );
        }
    }

    pub async fn start_recovery(&self, index: usize) -> TestResult<tokio::task::JoinHandle<()>> {
        let (entered_tx, entered_rx) = oneshot::channel();
        let entered = Arc::new(std::sync::Mutex::new(Some(entered_tx)));
        set_compactor_sub_agent_handler(
            self.sessions[index].id,
            Some(Arc::new(move |_, _| {
                if let Some(sender) = entered
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .take()
                {
                    let _ = sender.send(());
                }
                Box::pin(std::future::pending())
            })),
        );
        let session = self.sessions[index].clone();
        let task = tokio::spawn(async move {
            let error =
                codelet_providers::request_admission::RequestAdmissionError::BudgetExceeded {
                    input_tokens: 201_000,
                    output_reservation: 4096,
                    context_capacity: 200_000,
                }
                .into();
            let _ = codelet_agent_loop::terminal_overflow_recovery::try_terminal_overflow_recovery(
                session, &error,
            )
            .await;
        });
        tokio::time::timeout(Duration::from_secs(3), entered_rx).await??;
        Ok(task)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
        }
        for session in &self.sessions {
            set_compactor_sub_agent_handler(session.id, None);
        }
    }
}
