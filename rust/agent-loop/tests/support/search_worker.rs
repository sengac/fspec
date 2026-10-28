use codelet_core::persistence::{self, SessionManifest};
use codelet_tools::session_search::{types::SessionSearchAction, SessionSearchHandler};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub fn cancelled_index_publication(directory: &std::path::Path, message: uuid::Uuid) -> TestResult {
    use codelet_common::read_work::{observe, ReadCancellation};
    let index = directory.join("messages/messages.idx");
    std::fs::remove_file(&index)?;
    persistence::reset_message_store_for_tests();
    let token = ReadCancellation::default();
    let cancellation = token.clone();
    let checkpoints = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = checkpoints.clone();
    let (result, _) = token.run(|| {
        observe(
            Arc::new(move |boundary| {
                if boundary == "index.publish" && seen.fetch_add(1, Ordering::AcqRel) == 1 {
                    cancellation.cancel();
                }
            }),
            || persistence::get_message(message),
        )
    });
    assert_eq!(checkpoints.load(Ordering::Acquire), 2);
    assert!(
        result.is_err(),
        "cancelled recovery published a successful lookup"
    );
    assert!(!index.exists(), "cancelled derived index was promoted");
    assert!(persistence::get_message(message)?.is_some());
    assert!(index.exists());
    Ok(())
}

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub manifest: SessionManifest,
    pub other: SessionManifest,
    pub handler: SessionSearchHandler,
}

struct LatchedTool {
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
    completed: Arc<std::sync::atomic::AtomicUsize>,
}

impl rig::tool::Tool for LatchedTool {
    const NAME: &'static str = "LatchedControl";
    type Args = serde_json::Value;
    type Output = String;
    type Error = std::io::Error;

    async fn definition(&self, _: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: Self::NAME.into(),
            description: "Synthetic default-policy control".into(),
            parameters: serde_json::json!({"type":"object"}),
        }
    }

    async fn call(&self, _: Self::Args) -> Result<String, Self::Error> {
        self.started.notify_one();
        self.release.notified().await;
        self.completed.fetch_add(1, Ordering::AcqRel);
        Err(std::io::Error::other("genuine synthetic error after latch"))
    }
}

pub async fn queued_abandonment_and_default_policy() -> TestResult {
    use codelet_tools::session_search::{set_session_search_handler, SessionSearchTool};
    use std::{future::Future, task::Poll, time::Duration};
    let session = uuid::Uuid::new_v4();
    let searched = Arc::new(AtomicBool::new(false));
    let seen = searched.clone();
    set_session_search_handler(
        session,
        Some(Arc::new(move |_, _| {
            seen.store(true, Ordering::Release);
            codelet_tools::session_search::types::SessionSearchResult::Recent { sessions: vec![] }
        })),
    );
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let server = rig::tool::server::ToolServer::new()
        .tool(LatchedTool {
            started: started.clone(),
            release: release.clone(),
            completed: completed.clone(),
        })
        .tool(SessionSearchTool::new(session))
        .run();
    let control_server = server.clone();
    let control = tokio::spawn(async move {
        control_server
            .call_tool("LatchedControl", "{\"work_unit_id\":\"NONE\"}")
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), started.notified()).await?;
    let mut search = Box::pin(server.call_tool(
        "SessionSearch",
        "{\"action_type\":\"recent\",\"work_unit_id\":\"NONE\"}",
    ));
    std::future::poll_fn(|context| {
        assert!(search.as_mut().poll(context).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(search);
    control.abort();
    let _ = control.await;
    assert_eq!(completed.load(Ordering::Acquire), 0);
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), server.get_tool_defs(None)).await??;
    set_session_search_handler(session, None);
    assert_eq!(
        completed.load(Ordering::Acquire),
        1,
        "non-opted-in tool was universally aborted"
    );
    assert!(
        !searched.load(Ordering::Acquire),
        "abandoned queued search was dispatched"
    );
    let genuine = server.call_tool("LatchedControl", "{\"work_unit_id\":\"NONE\"}");
    release.notify_one();
    let error = tokio::time::timeout(Duration::from_secs(2), genuine)
        .await?
        .expect_err("genuine synthetic error must be returned");
    assert!(error
        .to_string()
        .contains("genuine synthetic error after latch"));
    assert_eq!(completed.load(Ordering::Acquire), 2);
    Ok(())
}

pub struct CompetingWriter {
    target: SessionManifest,
    progressed: AtomicBool,
    task: Mutex<Option<std::thread::JoinHandle<Result<(), String>>>>,
}

impl CompetingWriter {
    pub fn new(target: SessionManifest) -> Self {
        Self {
            target,
            progressed: AtomicBool::new(false),
            task: Mutex::new(None),
        }
    }

    pub fn run(&self) {
        let mut target = self.target.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let task = std::thread::spawn(move || {
            persistence::append_message(&mut target, "user", "competing append")?;
            let _ = sender.send(());
            Ok(())
        });
        *self.task.lock().expect("writer handle poisoned") = Some(task);
        self.progressed.store(
            receiver
                .recv_timeout(std::time::Duration::from_millis(350))
                .is_ok(),
            Ordering::Release,
        );
    }

    pub fn finish(&self) -> TestResult {
        self.task
            .lock()
            .expect("writer handle poisoned")
            .take()
            .ok_or("batch boundary was never reached")?
            .join()
            .map_err(|_| "competing writer failed")??;
        assert!(
            self.progressed.load(Ordering::Acquire),
            "search retained storage across its batch checkpoint"
        );
        Ok(())
    }
}

impl Fixture {
    pub fn new(count: usize) -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().into())?;
        let mut manifest = persistence::create_session("large history", directory.path())?;
        for index in 0..count {
            let text = format!("needle synthetic original {index}");
            let metadata = serde_json::json!({
                "uuid": uuid::Uuid::new_v4(),
                "message": {"role":"user", "content":[{"type":"text","text":text}]}
            });
            persistence::append_message_with_metadata(
                &mut manifest,
                "user",
                &text,
                serde_json::from_value(metadata)?,
            )?;
        }
        let mut other = persistence::create_session("other history", directory.path())?;
        persistence::append_message(&mut other, "user", "other needle")?;
        let handler = codelet_agent_loop::session_search_handler::create_handler(
            directory.path().into(),
            Arc::new(AtomicBool::new(false)),
        );
        Ok(Self {
            directory,
            manifest,
            other,
            handler,
        })
    }

    pub fn search(&self, limit: usize) -> SessionSearchAction {
        SessionSearchAction::Search {
            query: "needle".into(),
            context_turns: Some(1),
            limit: Some(limit),
            all_projects: Some(false),
            last_hours: None,
            last_days: None,
            after: None,
            before: None,
            start_turn: None,
            end_turn: None,
        }
    }

    pub fn show(&self) -> SessionSearchAction {
        SessionSearchAction::Show {
            session_id: Some(self.manifest.id.to_string()),
            user_only: Some(true),
            max_turns: Some(2),
            start_turn: Some(1),
            end_turn: Some(3),
        }
    }
}
