use codelet_core::persistence::{
    get_message, get_session_message_envelopes, load_session, reset_stores_for_tests,
    transcript::{JournalEvent, JournalRecord, JournalStore, TerminalOutcome},
    update_message_metadata,
};
use codelet_core::SessionManagerHandle;
use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::SessionManager;
use serde_json::json;
use std::{path::Path, process::Command};
use uuid::Uuid;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Environment {
    _directory: tempfile::TempDir,
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

pub async fn repeat_applied_compaction(fixture: &super::automatic::Harness) {
    let target = format!("Target session: {}", fixture.session.id);
    let infer = fixture.infer.clone();
    wiremock::Mock::given(move |request: &wiremock::Request| {
        let text = String::from_utf8_lossy(&request.body);
        !text.contains(&target) && !text.contains("Extract learnings from this session summary")
    })
    .respond_with(move |_: &wiremock::Request| {
        let number = infer.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let chunk = json!({
            "id":format!("applied-{number}"),"object":"chat.completion.chunk","created":1,"model":"o3",
            "choices":[{"index":0,"delta":{"role":"assistant","tool_calls":[{"index":0,"id":format!("summary-{number}"),"type":"function","function":{"name":"GenerateCompaction","arguments":json!({"work_unit_id":"NONE"}).to_string()}}]},"finish_reason":"tool_calls"}],
            "usage":{"prompt_tokens":100,"completion_tokens":10,"total_tokens":110}
        });
        wiremock::ResponseTemplate::new(200)
            .insert_header("content-type", "text/event-stream")
            .set_body_string(format!("data: {chunk}\n\ndata: [DONE]\n\n"))
    })
    .with_priority(1)
    .mount(&fixture.server)
    .await;
}

impl Environment {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        let previous = [
            "ANTHROPIC_API_KEY",
            "OPENAI_API_KEY",
            "OPENAI_BASE_URL",
            "FSPEC_USER_DIR",
        ]
        .into_iter()
        .map(|name| (name, std::env::var_os(name)))
        .collect();
        std::env::set_var("FSPEC_USER_DIR", directory.path());
        std::env::set_var("ANTHROPIC_API_KEY", "test-outer-failure-offline");
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
        reset_stores_for_tests();
    }
}

pub fn prepare(root: &Path, session: Uuid) -> TestResult {
    std::fs::create_dir_all(root.join("cache"))?;
    std::fs::write(
        root.join("cache/models.json"),
        include_str!("../fixtures/prov101_models.json"),
    )?;
    for reference in load_session(session)?.messages {
        let original = get_message(reference.message_id)?.ok_or("missing fixture original")?;
        update_message_metadata(
            original.id,
            serde_json::from_value(json!({
                "uuid":original.id,"provider":"openai",
                "message":{"role":original.role,"content":[{"type":"text","text":original.content}]}
            }))?,
        )?;
    }
    Ok(())
}

pub fn failures(session: Uuid) -> Result<Vec<JournalRecord>, Box<dyn std::error::Error>> {
    Ok(JournalStore::configured(session)?
        .read()?
        .into_iter()
        .filter(|record| {
            matches!(
                record.event,
                JournalEvent::Terminal {
                    outcome: TerminalOutcome::ProviderError | TerminalOutcome::CompactionFailed
                }
            )
        })
        .collect())
}

pub fn cold(root: &Path, session: Uuid, expected: usize) -> TestResult {
    let result = Command::new(std::env::current_exe()?)
        .args(["--ignored", "--exact", "cold_outer_failure_child"])
        .env("BUG258_ROOT", root)
        .env("BUG258_SESSION", session.to_string())
        .env("BUG258_EXPECTED", expected.to_string())
        .status()?;
    assert!(
        result.success(),
        "fresh-process cold replay lost or duplicated the terminal outcome"
    );
    Ok(())
}

pub async fn cold_child() -> TestResult {
    let root = std::env::var_os("BUG258_ROOT").ok_or("missing cold root")?;
    codelet_common::set_data_directory(root.into())?;
    reset_stores_for_tests();
    let session = Uuid::parse_str(&std::env::var("BUG258_SESSION")?)?;
    let expected: usize = std::env::var("BUG258_EXPECTED")?.parse()?;
    let records = JournalStore::configured(session)?.read()?;
    let manifest = serde_json::to_value(load_session(session)?)?;
    let mut previous = None;
    for _ in 0..2 {
        reset_stores_for_tests();
        let manager = SessionManager::new();
        let id = SessionId::new(session.to_string());
        manager
            .create_session_from_manifest(&load_session(session)?, "openai/o3")
            .await?;
        manager.resume_session(&id)?;
        let background = manager.get_session(&id.value)?;
        let chunks = background.get_buffered_output(usize::MAX);
        let safe_failures = chunks
            .iter()
            .filter(|chunk| match chunk {
                StreamChunk::Error { error } => error == TerminalOutcome::ProviderError.message(),
                StreamChunk::UserNotification { message, .. } => {
                    message == TerminalOutcome::CompactionFailed.message()
                }
                _ => false,
            })
            .count();
        assert_eq!(safe_failures, expected, "cold terminal failure count");
        let rendered = serde_json::to_value(&chunks)?;
        if let Some(previous) = &previous {
            assert_eq!(
                &rendered, previous,
                "cold replay identity or ordering changed"
            );
        }
        previous = Some(rendered);
        let model = get_session_message_envelopes(session)?.join("");
        let inner = serde_json::to_string(&background.inner.lock().await.messages)?;
        for forbidden in [
            "_transcript",
            "Provider request failed.",
            TerminalOutcome::CompactionFailed.message(),
            "context_length_exceeded",
            "summary service unavailable",
        ] {
            assert!(
                !model.contains(forbidden),
                "terminal metadata polluted model originals"
            );
            assert!(
                !inner.contains(forbidden),
                "terminal metadata polluted model history"
            );
        }
        assert_eq!(JournalStore::configured(session)?.read()?, records);
        assert_eq!(serde_json::to_value(load_session(session)?)?, manifest);
    }
    Ok(())
}
