use super::automatic::Harness;
use super::wire::{response, Reply};
use codelet_rpc_types::{SessionStatus, StreamChunk};
use codelet_tools::tool_pause::PauseResponse;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use wiremock::Mock;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct PauseRelease(Arc<codelet_sessions::background_session::BackgroundSession>);

impl Drop for PauseRelease {
    fn drop(&mut self) {
        self.0.interrupt();
        let _ = self
            .0
            .get_pause_response_tx()
            .send(PauseResponse::Interrupted);
    }
}

pub struct Environment {
    _directory: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl Environment {
    pub fn new() -> TestResult<Self> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let previous = [
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "FSPEC_USER_DIR",
            "CODEX_HOME",
        ]
        .into_iter()
        .map(|name| (name, std::env::var_os(name)))
        .collect();
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        std::env::set_var("CODEX_HOME", directory.path().join("codex"));
        Ok(Self {
            _directory: directory,
            previous,
        })
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        for (name, previous) in &self.previous {
            match previous {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
        codelet_core::persistence::reset_stores_for_tests();
    }
}

#[derive(Debug)]
pub struct Observation {
    pub rejections: u32,
    pub armed: bool,
    pub done_registered: bool,
    pub body: String,
}

pub async fn script(
    fixture: &Harness,
    replies: impl Fn(usize) -> Reply + Send + Sync + 'static,
) -> Arc<Mutex<Vec<Observation>>> {
    let observations = Arc::new(Mutex::new(Vec::new()));
    let captured = observations.clone();
    let session_id = fixture.session.id;
    let target = format!("Target session: {session_id}");
    Mock::given(move |request: &wiremock::Request| {
        let text = String::from_utf8_lossy(&request.body);
        !text.contains(&target) && !text.contains("Extract learnings from this session summary")
    })
    .respond_with(move |request: &wiremock::Request| {
        let body: serde_json::Value =
            serde_json::from_slice(&request.body).expect("local inference request JSON");
        let mut observations = captured.lock().expect("observations lock");
        let number = observations.len();
        observations.push(Observation {
            rejections: codelet_tools::done_rejection_count(session_id),
            armed: codelet_tools::is_continue_armed(session_id),
            done_registered: body["tools"]
                .as_array()
                .is_some_and(|tools| tools.iter().any(|tool| tool["function"]["name"] == "Done")),
            body: String::from_utf8_lossy(&request.body).into_owned(),
        });
        response(replies(number), number)
    })
    .with_priority(1)
    .mount(&fixture.server)
    .await;
    observations
}

pub async fn finish_recovery(fixture: &mut Harness, loop_abort: bool) -> Vec<StreamChunk> {
    let mut chunks = fixture.until_done().await;
    if loop_abort {
        chunks.extend(fixture.until_done().await);
    }
    chunks
}

pub async fn assert_budget_recovery(loop_abort: bool) -> TestResult {
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    fixture.session.set_continue_state(true, 1);
    let observations = script(&fixture, move |number| match number {
        1 if loop_abort => Reply::Loop,
        1 => Reply::Overflow,
        _ => Reply::Text,
    })
    .await;
    fixture.send();
    let chunks = finish_recovery(&mut fixture, loop_abort).await;
    let inner = fixture.session.inner.lock().await;
    assert_eq!(inner.continue_nudges_used, 1);
    let observations = observations.lock().map_err(|error| error.to_string())?;
    assert_eq!(observations.len(), 3, "recovery renewed the nudge budget");
    assert!(observations
        .iter()
        .all(|entry| entry.armed && entry.done_registered));
    assert!(chunks.iter().any(|chunk| matches!(chunk,
        StreamChunk::ContinueStateUpdate { continue_state } if continue_state.nudges_used == 1)));
    if loop_abort {
        assert!(observations[2].body.contains("fresh approach"));
        assert_eq!(
            fixture.builders.load(std::sync::atomic::Ordering::SeqCst),
            0
        );
    } else {
        assert!(observations[2].body.contains("Completed evidence"));
        assert_eq!(
            fixture.builders.load(std::sync::atomic::Ordering::SeqCst),
            1
        );
    }
    Ok(())
}

pub async fn assert_goal_recovery(loop_abort: bool) -> TestResult {
    let _environment = Environment::new()?;
    let mut fixture = Harness::new(false, false, false).await;
    let _release = PauseRelease(fixture.session.clone());
    fixture
        .session
        .set_goal_state(Some(("finish the verified task".into(), None)));
    let observations = script(&fixture, move |number| match number {
        0 | 1 | 3 | 4 => Reply::RejectDone,
        2 if loop_abort => Reply::Loop,
        2 => Reply::Overflow,
        _ => Reply::Text,
    })
    .await;
    let session = fixture.session.clone();
    let reviewer = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                if session.get_status() == SessionStatus::Paused {
                    let snapshot: serde_json::Value = serde_json::from_str(
                        &session
                            .interaction_snapshot()
                            .expect("interaction snapshot"),
                    )
                    .expect("snapshot JSON");
                    if let Some(request_id) = snapshot["pause"]["request_id"].as_str() {
                        let message = snapshot["pause"]["state"]["message"].clone();
                        session
                            .send_pause_response_checked(request_id, PauseResponse::Resumed)
                            .expect("resolve this synthetic fixture's current pause");
                        return message;
                    }
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
    });
    fixture.send();
    finish_recovery(&mut fixture, loop_abort).await;
    let message = reviewer.await??;
    assert!(
        message
            .as_str()
            .is_some_and(|text| text.contains("repeatedly claims completion")),
        "wrong escalation reason: {message}"
    );
    assert_eq!(codelet_tools::done_rejection_count(fixture.session.id), 4);
    let inner = fixture.session.inner.lock().await;
    assert_eq!(inner.done_rejections, 4);
    assert!(inner.goal.is_some());
    let observations = observations.lock().map_err(|error| error.to_string())?;
    assert_eq!(observations.len(), 6);
    assert_eq!(observations[2].rejections, 2);
    assert_eq!(
        observations[3].rejections, 2,
        "recovery erased goal rejections"
    );
    assert!(observations
        .iter()
        .all(|entry| entry.armed && entry.done_registered));
    Ok(())
}
