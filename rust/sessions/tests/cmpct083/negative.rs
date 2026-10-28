use super::{
    fixture::{self, Fixture, TestResult},
    runtime::{self, Output},
    transport,
};
use codelet_cli::{
    compaction_operation::CompactionFailure,
    compactor_sub_agent::{run_compactor_sub_agent_round, set_compactor_sub_agent_handler},
};
use codelet_core::{persistence::*, TokenState};
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

async fn blocked(fixture: &Fixture, provider: &str, output: &Output) -> TestResult {
    let server = transport::server(provider).await;
    let mut inner = fixture.background.inner.lock().await;
    let result = runtime::send_inner(
        provider,
        fixture.directory.path(),
        &server,
        &mut inner,
        fixture.background.id,
        output,
    )
    .await;
    assert!(
        result.is_err(),
        "failed operation admitted ordinary inference"
    );
    assert!(inner.ensure_compaction_inference_ready().is_err());
    assert!(
        server
            .received_requests()
            .await
            .ok_or("request capture")?
            .is_empty(),
        "failure sent an alternate request"
    );
    output.assert_no_completion();
    Ok(())
}

pub async fn run(row: &str) -> TestResult {
    let provider = if row.contains("websocket") || row.contains("checkpoint") {
        "codex"
    } else {
        "openai"
    };
    let fixture = Fixture::new(provider).await?;
    runtime::send(provider, fixture.directory.path(), &fixture.background).await?;
    let stale_source = fixture
        .background
        .inner
        .lock()
        .await
        .capture_durable_compaction(fixture.background.id)?;
    super::application::cycle(&fixture, provider, 1, false).await?;
    let before = fixture.bytes()?;
    let originals = serde_json::to_value(get_session_messages_full(&load_session(
        fixture.background.id,
    )?)?)?;
    let output = Output::default();
    match row {
        "missing handler" | "bad candidate" | "timeout" => {
            builder_failure(&fixture, row, &output).await?
        }
        "stale generation" => stale(&fixture, stale_source).await?,
        "durable write failure" => write_failure(&fixture).await?,
        "structured replay gap" => replay_gap(&fixture).await?,
        "websocket upgrade rejection" | "incompatible checkpoint lineage" => {
            super::negative_transport::run(&fixture, row, &output).await?
        }
        _ => return Err(format!("unrecognized negative row {row}").into()),
    }
    blocked(&fixture, provider, &output).await?;
    assert_eq!(
        before,
        fixture.bytes()?,
        "{row} changed the committed manifest"
    );
    assert_eq!(
        originals,
        serde_json::to_value(get_session_messages_full(&load_session(
            fixture.background.id
        )?)?)?,
        "{row} changed immutable original payloads"
    );
    Ok(())
}

async fn builder_failure(fixture: &Fixture, row: &str, output: &Output) -> TestResult {
    let id = fixture.background.id;
    set_compactor_sub_agent_handler(id, None);
    codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(None);
    let server = transport::builder_response(
        "not a DAG",
        if row == "timeout" {
            Duration::from_secs(60)
        } else {
            Duration::ZERO
        },
    )
    .await;
    std::env::set_var("OPENAI_BASE_URL", server.uri());
    std::env::set_var("OPENAI_MODEL", "o3");
    let mut inner = fixture.background.inner.lock().await;
    if row != "missing handler" {
        codelet_agent_loop::generate_compaction_handler::register_compactor_sub_agent_handler(
            id,
            &inner,
            fixture.directory.path().into(),
            fixture.background.compaction_in_progress.clone(),
        );
    }
    codelet_cli::interactive::set_deep_search_wall_clock_timeout_override(Some(
        // The budget includes actual request preparation and tool-definition
        // capture. 500 ms expired before this fixture reached either fault.
        Duration::from_secs(5),
    ));
    let result = tokio::time::timeout(
        Duration::from_secs(8),
        run_compactor_sub_agent_round(
            &mut inner,
            id,
            &Arc::new(Mutex::new(TokenState::default())),
            &Arc::new(AtomicBool::new(false)),
            output,
        ),
    )
    .await?;
    set_compactor_sub_agent_handler(id, None);
    let failure = result
        .err()
        .ok_or("negative builder row must return typed failure")?;
    let requests = server
        .received_requests()
        .await
        .ok_or("builder request capture")?;
    if row == "missing handler" {
        assert!(requests.is_empty(), "unavailable builder made a request");
    } else {
        assert!(
            !requests.is_empty(),
            "fault fixture must reach its real local provider request: {failure:?}"
        );
    }
    assert!(
        match (row, &failure) {
            ("missing handler", CompactionFailure::Unavailable) => true,
            // The real registered builder returns Result<String, String>;
            // run_round deliberately wraps its inner failure as Builder.
            ("bad candidate", CompactionFailure::Builder(reason)) => {
                reason.contains("candidate rejected after 3 attempts: invalid DAG candidate")
                    && reason.contains("missing or unclosed dag-node structure")
            }
            ("timeout", CompactionFailure::Builder(reason)) => {
                reason == "compaction synthesis deadline exceeded; submit fresh input to retry"
            }
            ("timeout", CompactionFailure::Deadline) => true,
            _ => false,
        },
        "wrong typed outcome {row}: {failure:?}"
    );
    Ok(())
}

async fn stale(fixture: &Fixture, source: compaction_boundary::CompactionSource) -> TestResult {
    let mut inner = fixture.background.inner.lock().await;
    let content = fixture::dag(source.prefix.len(), 2)?;
    let failure = inner
        .commit_selected_compaction(source, uuid::Uuid::new_v4(), &content, |_| Ok(()))
        .err()
        .ok_or("stale runtime source must be rejected")?;
    assert!(
        matches!(
            failure,
            compaction_commit::CompactionCommitError::Rejected(ref reason)
                if reason.contains("stale source generation")
        ),
        "{failure:?}"
    );
    Ok(())
}

async fn write_failure(fixture: &Fixture) -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let directory = fixture.directory.path().join("sessions");
    let previous = std::fs::metadata(&directory)?.permissions();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o500))?;
    let result = {
        let mut inner = fixture.background.inner.lock().await;
        let source = inner.capture_durable_compaction(fixture.background.id)?;
        let content = fixture::dag(source.prefix.len(), 2)?;
        inner.commit_selected_compaction(source, uuid::Uuid::new_v4(), &content, |_| Ok(()))
    };
    std::fs::set_permissions(directory, previous)?;
    let failure = result.err().ok_or("real manifest staging must fail")?;
    assert!(
        matches!(
            failure,
            compaction_commit::CompactionCommitError::Persistence(_)
        ),
        "{failure:?}"
    );
    Ok(())
}

async fn replay_gap(fixture: &Fixture) -> TestResult {
    let mut inner = fixture.background.inner.lock().await;
    let source = inner.capture_durable_compaction(fixture.background.id)?;
    let mut broken = load_session(fixture.background.id)?;
    broken
        .messages
        .first_mut()
        .ok_or("source reference")?
        .message_id = uuid::Uuid::new_v4();
    let content = fixture::dag(source.prefix.len(), 2)?;
    let original = fixture.bytes()?;
    let original_manifest = load_session(fixture.background.id)?;
    let originals = get_session_messages_full(&original_manifest)?;
    let messages = inner.messages.clone();
    let tokens = serde_json::to_value(&inner.token_tracker)?;
    let path = fixture
        .directory
        .path()
        .join(format!("sessions/{}.json", fixture.background.id));
    std::fs::write(&path, serde_json::to_vec(&broken)?)?;
    reset_stores_for_tests();
    let broken_bytes = fixture.bytes()?;
    let result =
        inner.commit_selected_compaction(source, uuid::Uuid::new_v4(), &content, |_| Ok(()));
    let failure = result
        .err()
        .ok_or("structured replay with a missing original must fail")?;
    assert!(
        matches!(
            failure,
            compaction_commit::CompactionCommitError::Rejected(_)
        ),
        "{failure:?}"
    );
    assert_eq!(
        fixture.bytes()?,
        broken_bytes,
        "replay rejection mutated the manifest before fixture cleanup"
    );
    let rejected = load_session(fixture.background.id)?;
    assert_eq!(
        serde_json::to_value(&rejected)?,
        serde_json::to_value(&broken)?
    );
    assert_eq!(
        rejected
            .compaction_commit
            .as_ref()
            .ok_or("rejected boundary")?
            .identity,
        original_manifest
            .compaction_commit
            .as_ref()
            .ok_or("original boundary")?
            .identity
    );
    assert_eq!(
        inner.messages, messages,
        "replay rejection changed live history"
    );
    assert_eq!(serde_json::to_value(&inner.token_tracker)?, tokens);
    for original_message in originals {
        assert_eq!(
            serde_json::to_value(
                get_message(original_message.id)?
                    .ok_or("immutable original after replay rejection")?
            )?,
            serde_json::to_value(original_message)?
        );
    }
    std::fs::write(&path, original)?;
    reset_stores_for_tests();
    Ok(())
}
