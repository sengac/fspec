#[path = "../../core/tests/support/compaction_boundary.rs"]
mod boundary;
#[path = "support/captured_compaction_builder.rs"]
mod captured_builder;
#[path = "../../sessions/tests/cmpct060/support.rs"]
mod support;

use codelet_agent_loop::compaction_evidence::CompactionEvidence;
use codelet_agent_loop::generate_compaction_handler::register_generate_compaction_handler;
use codelet_core::compaction::{validate_dag_candidate, DagValidationContext};
use codelet_core::persistence::compaction_commit::{commit_compaction, reconcile_compaction};
use codelet_core::persistence::*;
use codelet_tools::inject_summary::{InjectSummaryArgs, InjectSummaryTool};
use codelet_tools::session_search::originals::OriginalsRequest;
use codelet_tools::session_search::types::{SessionSearchAction, SessionSearchResult};
use rig::tool::Tool;

const RAW_D2: &str = "<dag-node depth=\"D2\" turns=\"0-0\" label=\"Permanent\">exact durable memory</dag-node>\n<dag-files>\n</dag-files>";

fn originals_request(session_id: uuid::Uuid) -> OriginalsRequest {
    OriginalsRequest {
        session_id: session_id.to_string(),
        start_turn: None,
        end_turn: None,
        next_turn: None,
        limit: Some(1),
        max_bytes: None,
    }
}

fn append_original(manifest: &mut SessionManifest, content: &str) -> Result<(), String> {
    append_message_with_metadata(
        manifest,
        "user",
        content,
        [
            ("uuid".into(), serde_json::json!(uuid::Uuid::new_v4())),
            (
                "message".into(),
                serde_json::json!({"role":"user", "content":content}),
            ),
        ]
        .into(),
    )?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn empty_capture_cannot_read_later_appends() -> support::TestResult {
    let (_directory, manager, id, _) = support::manager().await?;
    let background = manager.get_session(&id.value)?;
    let source = background
        .inner
        .lock()
        .await
        .capture_durable_compaction(background.id)?;
    let evidence = CompactionEvidence::capture(source)?;
    let mut manifest = load_session(background.id)?;
    append_original(&mut manifest, "live suffix")?;
    let handler = evidence.search_handler(manifest.project);
    let result = handler(
        SessionSearchAction::Originals {
            request: originals_request(background.id),
        },
        background.id,
    );
    assert!(matches!(result, SessionSearchResult::OriginalsError { .. }));
    assert!(!serde_json::to_string(&result)?.contains("live suffix"));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn evidence_pages_stop_at_capture_even_after_same_turn_appends() -> support::TestResult {
    let (_directory, manager, id, _) = support::manager().await?;
    let background = manager.get_session(&id.value)?;
    let mut manifest = load_session(background.id)?;
    append_original(&mut manifest, "captured first")?;
    append_original(&mut manifest, "captured second")?;
    let source = background
        .inner
        .lock()
        .await
        .capture_durable_compaction(background.id)?;
    let evidence = CompactionEvidence::capture(source)?;
    append_original(&mut manifest, "live suffix must not be evidence")?;
    let handler = evidence.search_handler(manifest.project.clone());
    let mut request = originals_request(background.id);
    let mut indices = Vec::new();
    loop {
        let result = handler(
            SessionSearchAction::Originals {
                request: request.clone(),
            },
            background.id,
        );
        let SessionSearchResult::Originals { page } = result else {
            return Err(format!("unexpected evidence result: {result:?}").into());
        };
        indices.extend(page.records.iter().map(|record| record.turn_index));
        assert!(!serde_json::to_string(&page)?.contains("live suffix"));
        match page.next_turn {
            Some(next) => request.next_turn = Some(next),
            None => break,
        }
    }
    assert_eq!(indices, [0, 1]);
    for action in [
        SessionSearchAction::Show {
            session_id: None,
            user_only: None,
            max_turns: Some(1),
            start_turn: None,
            end_turn: None,
        },
        SessionSearchAction::Search {
            query: "captured".into(),
            context_turns: Some(10),
            limit: None,
            all_projects: None,
            last_hours: None,
            last_days: None,
            after: None,
            before: None,
            start_turn: None,
            end_turn: None,
        },
    ] {
        let result = handler(action, uuid::Uuid::new_v4());
        let encoded = serde_json::to_string(&result)?;
        assert!(encoded.contains("captured second"), "{encoded}");
        assert!(!encoded.contains("live suffix"), "{encoded}");
    }
    request.end_turn = Some(100);
    request.next_turn = Some(2);
    let result = handler(SessionSearchAction::Originals { request }, background.id);
    assert!(!serde_json::to_string(&result)?.contains("live suffix"));
    let result = handler(
        SessionSearchAction::Originals {
            request: originals_request(uuid::Uuid::new_v4()),
        },
        background.id,
    );
    assert!(matches!(result, SessionSearchResult::OriginalsError { .. }));
    manifest.messages.swap(0, 1);
    save_session(&manifest)?;
    let result = handler(
        SessionSearchAction::Originals {
            request: originals_request(background.id),
        },
        background.id,
    );
    assert!(matches!(result, SessionSearchResult::Error { .. }));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn durable_raw_d2_is_used_without_active_message_markers() -> support::TestResult {
    let (_directory, manager, id, _) = support::manager().await?;
    let background = manager.get_session(&id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(&mut manifest, "user", "covered")?;
    let mut candidate = boundary::candidate(&manifest)?;
    let inner = background.inner.lock().await;
    let model = inner.current_model_id().ok_or("model")?;
    candidate.source = inner.capture_durable_compaction(background.id)?;
    candidate.checkpoint.model = model.clone();
    candidate.checkpoint.payload =
        serde_json::to_value(inner.provider_session.prepare_generation(1, &model)?)?;
    candidate.dag = validate_dag_candidate(
        RAW_D2,
        &DagValidationContext {
            persisted_turn_count: 1,
            required_coverage_end: Some(0),
            ..Default::default()
        },
    )?;
    let committed = commit_compaction(candidate, &model, |_, _| Ok(()))?;
    reconcile_compaction(
        &committed
            .compaction_commit
            .as_ref()
            .ok_or("commit")?
            .identity,
    )?;
    let evidence = CompactionEvidence::capture(inner.capture_durable_compaction(background.id)?)?;
    assert_eq!(evidence.existing_dag(), Some((RAW_D2.to_string(), 0)));
    let _queue = register_generate_compaction_handler(
        background.id,
        &inner,
        manifest.project,
        background.clone(),
        None,
    );
    drop(inner);
    let replacement = RAW_D2.replace("exact durable memory", "rewritten permanent memory");
    assert!(InjectSummaryTool::new(background.id)
        .call(InjectSummaryArgs {
            content: replacement
        })
        .await
        .is_err());
    InjectSummaryTool::new(background.id)
        .call(InjectSummaryArgs {
            content: RAW_D2.into(),
        })
        .await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn ordinary_user_marker_does_not_become_authoritative_dag() -> support::TestResult {
    let (_directory, manager, id, _) = support::manager().await?;
    let background = manager.get_session(&id.value)?;
    let mut manifest = load_session(background.id)?;
    append_message(
        &mut manifest,
        "user",
        "Explain <!-- type:compaction-dag -->",
    )?;
    let mut inner = background.inner.lock().await;
    inner.messages.push(rig::message::Message::user(
        "Explain <!-- type:compaction-dag -->",
    ));
    let _queue = register_generate_compaction_handler(
        background.id,
        &inner,
        manifest.project,
        background.clone(),
        None,
    );
    drop(inner);
    InjectSummaryTool::new(background.id)
        .call(InjectSummaryArgs {
            content: RAW_D2.into(),
        })
        .await?;
    Ok(())
}
