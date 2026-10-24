use super::{
    fixture::{self, Fixture, TestResult},
    lifecycle,
    runtime::{self, Output},
    transport,
};
use codelet_core::persistence::*;
use codelet_rpc_types::StreamChunk;
use codelet_sessions::background_session::BackgroundSession;
use std::{
    path::Path,
    sync::{atomic::AtomicBool, Arc, Mutex},
};
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

pub async fn reconcile(root: &Path, background: &BackgroundSession) -> TestResult {
    let manifest = load_session(background.id)?;
    let identity = manifest
        .compaction_commit
        .as_ref()
        .ok_or("pending boundary")?
        .identity
        .clone();
    assert!(
        manifest
            .compaction_commit
            .as_ref()
            .ok_or("pending boundary")?
            .publication_pending
    );
    let server = transport::server("openai").await;
    let output = Output::default();
    let mut inner = background.inner.lock().await;
    let result =
        runtime::send_inner("openai", root, &server, &mut inner, background.id, &output).await;
    assert!(
        result.is_err(),
        "pending publication must block actual runtime inference"
    );
    assert!(server
        .received_requests()
        .await
        .ok_or("capture")?
        .is_empty());
    let mut wrong = identity.clone();
    wrong.operation_id = uuid::Uuid::new_v4();
    drop(inner);
    assert!(background
        .reconcile_durable_compaction(&wrong)
        .await
        .is_err());
    assert!(background
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()
        .is_err());
    background.reconcile_durable_compaction(&identity).await?;
    let reconciled = fixture::manifest_bytes(root, background.id)?;
    background.reconcile_durable_compaction(&identity).await?;
    assert_eq!(
        reconciled,
        fixture::manifest_bytes(root, background.id)?,
        "duplicate reconciliation changed durable state"
    );
    background
        .inner
        .lock()
        .await
        .ensure_compaction_inference_ready()?;
    output.assert_no_completion();
    Ok(())
}

pub async fn cancel(test: &str) -> TestResult {
    if lifecycle::child().await? {
        return Ok(());
    }
    let fixture = Fixture::new("openai").await?;
    runtime::send("openai", fixture.directory.path(), &fixture.background).await?;
    super::application::cycle(&fixture, "openai", 1, true).await?;
    let before = fixture.bytes()?;
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_secs(60))
                .set_body_json(serde_json::json!({})),
        )
        .mount(&server)
        .await;
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    std::env::set_var("OPENAI_MODEL", "o3");
    {
        let inner = fixture.background.inner.lock().await;
        codelet_agent_loop::generate_compaction_handler::register_compactor_sub_agent_handler(
            fixture.background.id,
            &inner,
            fixture.directory.path().into(),
            fixture.background.compaction_in_progress.clone(),
        );
    }
    let background = fixture.background.clone();
    let output = Arc::new(Output::default());
    let recorded = output.clone();
    let operation = tokio::spawn(async move {
        let mut inner = background.inner.lock().await;
        codelet_cli::compactor_sub_agent::run_compactor_sub_agent_round(
            &mut inner,
            background.id,
            &Arc::new(Mutex::new(codelet_core::TokenState::default())),
            &Arc::new(AtomicBool::new(false)),
            recorded.as_ref(),
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(15), async {
        loop {
            if !server
                .received_requests()
                .await
                .ok_or("builder request capture")?
                .is_empty()
            {
                break;
            }
            if operation.is_finished() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    })
    .await??;
    assert_eq!(
        server.received_requests().await.ok_or("capture")?.len(),
        1,
        "cancellation must occur inside the real provider-backed working-view preparation"
    );
    operation.abort();
    assert!(operation
        .await
        .err()
        .ok_or("actual builder operation cancelled")?
        .is_cancelled());
    output.assert_no_completion();
    assert_eq!(
        before,
        fixture.bytes()?,
        "cancelled builder changed durable state"
    );
    let cold = lifecycle::cold(
        fixture.directory.path(),
        fixture.background.id,
        "openai",
        test,
        false,
    )
    .await?;
    lifecycle::assert_request(&cold, 1)?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&before)?,
        cold["manifest"],
        "cold resume changed durable state before its next physical request"
    );
    Ok(())
}

pub async fn notification(test: &str) -> TestResult {
    if lifecycle::child().await? {
        return Ok(());
    }
    let fixture = Fixture::new("openai").await?;
    runtime::send("openai", fixture.directory.path(), &fixture.background).await?;
    let background = &fixture.background;
    let promoted_identity;
    {
        let mut inner = background.inner.lock().await;
        let source = inner.capture_durable_compaction(background.id)?;
        let content = fixture::dag(source.prefix.len(), 1)?;
        fixture::append_suffix(background.id)?;
        let failure = inner
            .commit_selected_compaction(source, uuid::Uuid::new_v4(), &content, |_| {
                Err("completion receiver disconnected".into())
            })
            .err()
            .ok_or("publication must fail")?;
        let codelet_core::persistence::compaction_commit::CompactionCommitError::Publication {
            identity,
            ..
        } = failure
        else {
            return Err("runtime must report authoritative post-promotion identity".into());
        };
        promoted_identity = identity;
        assert!(inner.ensure_compaction_inference_ready().is_err());
    }
    let promoted = load_session(background.id)?;
    let originals = get_session_messages_full(&promoted)?;
    let cold = lifecycle::cold(
        fixture.directory.path(),
        background.id,
        "openai",
        test,
        true,
    )
    .await?;
    lifecycle::assert_request(&cold, 1)?;
    let restored: SessionManifest = serde_json::from_value(cold["manifest"].clone())?;
    let boundary = restored.compaction_commit.as_ref().ok_or("cold boundary")?;
    assert_eq!(boundary.identity, promoted_identity);
    assert!(!boundary.publication_pending);
    assert_eq!(
        serde_json::to_value(get_session_messages_full(&restored)?)?,
        serde_json::to_value(originals)?
    );
    let events: Vec<StreamChunk> = serde_json::from_value(cold["events"].clone())?;
    assert!(!events
        .iter()
        .any(|event| matches!(event, StreamChunk::CompactionComplete { .. })));
    let refreshes = events
        .iter()
        .filter_map(|event| match event {
            StreamChunk::CommittedCompaction { snapshot } => Some(snapshot),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        refreshes.len(),
        1,
        "exact reconciliation must emit one header refresh, not a duplicate completion"
    );
    let snapshot = refreshes[0];
    assert!(snapshot.reconciled);
    assert_eq!(
        u64::try_from(snapshot.generation)?,
        boundary.identity.generation
    );
    assert_eq!(
        u64::from(snapshot.tokens.input_tokens),
        restored.token_usage.current_context_tokens
    );
    assert_eq!(serde_json::to_value(snapshot)?, cold["contextSnapshot"]);
    Ok(())
}
