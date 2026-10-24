use codelet_providers::{codex::session::CodexSession, session::ProviderSession};
use codelet_rpc_types::{SessionId, StreamChunk};
use codelet_sessions::{background_session::BackgroundSession, SessionManager};
use serde_json::{json, Value};
use std::sync::Arc;

pub type TestResult = Result<(), Box<dyn std::error::Error>>;

pub struct Fixture {
    pub _directory: tempfile::TempDir,
    pub session: Arc<BackgroundSession>,
    pub extensions: ProviderSession,
    pub chunks: tokio::sync::broadcast::Receiver<(SessionId, StreamChunk)>,
}

pub async fn verify_retry_waiting(fixture: &mut Fixture) -> TestResult {
    use super::transport;

    fixture.extensions.begin_turn();
    fixture.drain();
    let server = transport::Server::start(
        vec![
            [transport::event(buffering()), reasoning("abandoned")].concat(),
            [
                transport::event(buffering()),
                transport::text(),
                transport::terminal("completed", None, None),
            ]
            .concat(),
        ],
        false,
    )
    .await?;
    let model = server
        .models()?
        .0
        .with_metadata_handle(fixture.codex()?.metadata.clone())
        .with_recovery_handle(fixture.codex()?.recovery.clone());
    let observed =
        tokio::time::timeout(std::time::Duration::from_secs(5), transport::collect(model))
            .await??;
    assert!(observed.errors.is_empty());
    assert_eq!(observed.text, "Hello");
    assert_eq!(observed.usage.len(), 1);
    assert_eq!(observed.finals.len(), 1);
    let chunks = fixture.drain();
    let active = Some("Giving this request a little extra thought".into());
    assert_eq!(waiting(&chunks), vec![active.clone(), None, active, None]);
    assert!(notices(&chunks)
        .iter()
        .any(|message| message.contains("Reconnected")));
    assert_eq!(
        server
            .requests
            .lock()
            .map_err(|_| "retry request capture")?
            .len(),
        2
    );
    Ok(())
}

pub fn reasoning(delta: &str) -> String {
    format!(
        "data: {}\n\n",
        json!({"type":"response.reasoning_summary_text.delta",
        "item_id":"reason-241", "output_index":1, "summary_index":0, "delta":delta})
    )
}

impl Fixture {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let directory = codelet_test_helpers::setup_temp_directory()?;
        codelet_core::persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().to_path_buf())?;
        codelet_providers::stub_provider::register_stub_provider();
        let manager = Arc::new(SessionManager::new());
        let session_id = manager
            .create_session("stub/canned", &directory.path().to_string_lossy())
            .await?;
        let session = manager.get_session(&session_id)?;
        let extensions = ProviderSession::for_provider("codex");
        session.prepare_provider_status(&extensions, "codex");
        extensions.begin_turn();
        let chunks = manager.chunks_tx().subscribe();
        Ok(Self {
            _directory: directory,
            session,
            extensions,
            chunks,
        })
    }

    pub fn codex(&self) -> Result<&CodexSession, Box<dyn std::error::Error>> {
        self.extensions
            .extension("codex")
            .ok_or_else(|| "missing Codex extension".into())
    }

    pub fn drain(&mut self) -> Vec<StreamChunk> {
        let mut chunks = Vec::new();
        while let Ok((_, chunk)) = self.chunks.try_recv() {
            chunks.push(chunk);
        }
        chunks
    }
}

pub fn buffering() -> Value {
    json!({"type":"response.metadata", "safety_buffering":{
        "use_cases":["PRIVATE-case"], "reasons":["PRIVATE-reason"]
    }})
}

pub fn verification() -> Value {
    json!({"type":"response.metadata", "metadata":{
        "openai_verification_recommendation":["trusted_access_for_cyber"]
    }})
}

pub fn waiting(chunks: &[StreamChunk]) -> Vec<Option<String>> {
    chunks
        .iter()
        .filter_map(|chunk| {
            let value = serde_json::to_value(chunk).ok()?;
            let waiting = value.get("ProviderWaiting")?;
            Some(waiting["message"].as_str().map(str::to_owned))
        })
        .collect()
}

pub fn verify_observer_deduplication() -> TestResult {
    use codelet_providers::codex::metadata::{CodexMetadataHandle, CodexPresentationEvent};
    use std::sync::Mutex;

    let handle = CodexMetadataHandle::default();
    let received = Arc::new(Mutex::new(Vec::new()));
    let captured = received.clone();
    let observer: Arc<dyn Fn(CodexPresentationEvent) + Send + Sync> = Arc::new(move |event| {
        if let Ok(mut events) = captured.lock() {
            events.push(event);
        }
    });
    handle.begin_turn();
    handle.set_presentation_observer(observer.clone());
    handle.apply_event(&verification());
    handle.set_presentation_observer(observer.clone());
    handle.apply_event(&verification());
    assert_eq!(
        received
            .lock()
            .map_err(|_| "verification observer capture")?
            .iter()
            .filter(|event| **event == CodexPresentationEvent::Verification)
            .count(),
        1
    );
    handle.begin_turn();
    handle.set_presentation_observer(observer);
    handle.apply_event(&verification());
    assert_eq!(
        received
            .lock()
            .map_err(|_| "next turn verification capture")?
            .iter()
            .filter(|event| **event == CodexPresentationEvent::Verification)
            .count(),
        2
    );
    Ok(())
}

pub fn verify_callback_ordering(fixture: &mut Fixture) -> TestResult {
    use codelet_providers::codex::metadata::CodexPresentationEvent;
    use std::sync::Mutex;

    let handle = fixture.codex()?.metadata.clone();
    fixture.extensions.begin_turn();
    fixture.drain();
    let resetter = handle.clone();
    handle.set_observer(Arc::new(move |status| {
        if status.buffering {
            resetter.reset();
        }
    }));
    handle.apply_event(&buffering());
    assert!(!handle.status().buffering);
    assert!(waiting(&fixture.drain()).iter().all(Option::is_none));

    handle.set_observer(Arc::new(|_| {}));
    handle.begin_turn();
    handle.apply_event(&buffering());
    assert!(waiting(&fixture.drain()).iter().any(Option::is_some));
    let resetter = handle.clone();
    handle.set_observer(Arc::new(move |status| {
        if !status.buffering {
            resetter.begin_turn();
        }
    }));
    handle.reset();
    assert_eq!(waiting(&fixture.drain()), vec![None]);

    let received = Arc::new(Mutex::new(Vec::new()));
    let target = received.clone();
    let replacement: Arc<dyn Fn(CodexPresentationEvent) + Send + Sync> = Arc::new(move |event| {
        if let Ok(mut captured) = target.lock() {
            captured.push(event);
        }
    });
    let rebound = handle.clone();
    handle.set_observer(Arc::new(move |status| {
        if status.buffering {
            rebound.set_presentation_observer(replacement.clone());
        }
    }));
    handle.apply_event(&buffering());
    assert_eq!(
        *received.lock().map_err(|_| "rebound event capture")?,
        vec![CodexPresentationEvent::Waiting(true)]
    );
    handle.set_observer(Arc::new(|_| {}));
    assert!(waiting(&fixture.drain()).is_empty());
    let target = received.clone();
    let replacement: Arc<dyn Fn(CodexPresentationEvent) + Send + Sync> = Arc::new(move |event| {
        if let Ok(mut captured) = target.lock() {
            captured.push(event);
        }
    });
    let rebound = handle.clone();
    handle.set_observer(Arc::new(move |status| {
        if !status.buffering {
            rebound.set_presentation_observer(replacement.clone());
        }
    }));
    handle.reset();
    assert_eq!(
        *received.lock().map_err(|_| "replacement clear capture")?,
        vec![
            CodexPresentationEvent::Waiting(true),
            CodexPresentationEvent::Waiting(false)
        ]
    );
    Ok(())
}

pub fn notices(chunks: &[StreamChunk]) -> Vec<String> {
    chunks
        .iter()
        .filter_map(|chunk| match chunk {
            StreamChunk::UserNotification { message, .. } => Some(message.clone()),
            _ => None,
        })
        .collect()
}
