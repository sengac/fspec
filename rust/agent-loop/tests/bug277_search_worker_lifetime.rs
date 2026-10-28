//! Feature: spec/features/cooperative-session-search.feature

#[path = "support/search_worker.rs"]
mod support;

use codelet_agent_loop::session_search_handler::create_handler;
use codelet_core::persistence::{self, append_message, create_session, get_message};
use codelet_tools::session_search::{
    set_session_search_handler,
    types::{SessionSearchAction, SessionSearchArgs, SessionSearchResult},
    SessionSearchTool,
};
use rig::tool::Tool;
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

use support::{Fixture, TestResult};

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_abandoned_search_releases_a_contended_storage_lease() -> TestResult {
    for via_server in [false, true] {
        // @step Given an active history search waiting for storage ownership
        let directory = codelet_test_helpers::setup_temp_directory()?;
        persistence::reset_stores_for_tests();
        codelet_common::set_data_directory(directory.path().into())?;
        let mut manifest = create_session("search worker", directory.path())?;
        append_message(&mut manifest, "user", "synthetic searchable history")?;
        get_message(uuid::Uuid::nil())?;
        let lease = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(directory.path().join("messages.lock"))?;
        lease.lock()?;
        let handler = create_handler(directory.path().into(), Arc::new(AtomicBool::new(false)));
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (finished_tx, mut finished_rx) = tokio::sync::oneshot::channel();
        let signals = Mutex::new(Some((started_tx, finished_tx)));
        set_session_search_handler(
            manifest.id,
            Some(Arc::new(move |action, id| {
                let (started, finished) = signals
                    .lock()
                    .expect("fixture signals poisoned")
                    .take()
                    .expect("single invocation");
                let _ = started.send(());
                let result = handler(action, id);
                let cancelled = matches!(&result, SessionSearchResult::Error { message } if message.contains("cancel"));
                let _ = finished.send(cancelled);
                result
            })),
        );
        let id = manifest.id;
        let search = tokio::spawn(async move {
            let tool = SessionSearchTool::new(id);
            let args = SessionSearchArgs {
                action: SessionSearchAction::Show {
                    session_id: Some(id.to_string()),
                    user_only: None,
                    max_turns: None,
                    start_turn: None,
                    end_turn: None,
                },
            };
            if via_server {
                let server = rig::tool::server::ToolServer::new().tool(tool).run();
                server
                    .call_tool(
                        "SessionSearch",
                        &serde_json::to_string(&args).map_err(|error| error.to_string())?,
                    )
                    .await
                    .map_err(|error| error.to_string())
            } else {
                tool.call(args).await.map_err(|error| error.to_string())
            }
        });
        tokio::time::timeout(Duration::from_secs(5), started_rx).await??;
        // @step When its tool future is dropped while storage remains unavailable
        search.abort();
        let aborted = search.await;
        assert!(aborted.is_err());
        let stopped = tokio::time::timeout(Duration::from_millis(350), &mut finished_rx).await;
        let stopped_before_release = stopped.is_ok();
        lease.unlock()?;
        let cancelled = match stopped {
            Ok(result) => result?,
            Err(_) => tokio::time::timeout(Duration::from_secs(5), finished_rx).await??,
        };
        set_session_search_handler(id, None);
        // @step Then the actual worker stops without waiting for storage release
        assert!(
            stopped_before_release,
            "abandoned worker waited for the storage lease"
        );
        // @step And it publishes no successful result into another turn
        assert!(
            cancelled,
            "abandoned invocation returned successful history"
        );
    }
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_cancellation_stops_reads_within_a_large_session() -> TestResult {
    use codelet_common::read_work::{observe, ReadCancellation};
    use std::sync::atomic::{AtomicUsize, Ordering};
    // @step Given a history search paused between bounded message batches
    let fixture = Fixture::new(96)?;
    let cancellation = ReadCancellation::default();
    let reads = Arc::new(AtomicUsize::new(0));
    let observed_reads = reads.clone();
    let token = cancellation.clone();
    let writer = Arc::new(support::CompetingWriter::new(fixture.other.clone()));
    let probe = writer.clone();
    let (result, _) = cancellation.run(|| {
        observe(
            Arc::new(move |boundary| {
                if boundary == "history.message" {
                    observed_reads.fetch_add(1, Ordering::AcqRel);
                }
                if boundary == "history.batch" && observed_reads.load(Ordering::Acquire) > 0 {
                    // @step And an unrelated session can persist while the search is paused
                    probe.run();
                    // @step When its invocation is cancelled
                    token.cancel();
                }
            }),
            || (fixture.handler)(fixture.show(), fixture.manifest.id),
        )
    });
    writer.finish()?;
    // @step Then no additional history batch is read
    assert!(matches!(result, SessionSearchResult::Error { message } if message.contains("cancel")));
    assert!(reads.load(Ordering::Acquire) > 0);
    assert!(reads.load(Ordering::Acquire) <= 32);
    let other = persistence::load_session(fixture.other.id)?;
    assert_eq!(other.messages.len(), 2);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn scenario_cancellation_covers_startup_and_session_boundaries() -> TestResult {
    use codelet_common::read_work::{observe, ReadCancellation};
    use std::sync::atomic::{AtomicUsize, Ordering};
    // @step Given history searches queued for execution or moving between sessions
    let fixture = Fixture::new(2)?;
    for before_start in [true, false] {
        let cancellation = ReadCancellation::default();
        let visits = Arc::new(AtomicUsize::new(0));
        let seen = visits.clone();
        let token = cancellation.clone();
        // @step When their invocation is cancelled at either boundary
        if before_start {
            cancellation.cancel();
        }
        let (result, _) = cancellation.run(|| {
            observe(
                Arc::new(move |boundary| {
                    if boundary == "search.session" && seen.fetch_add(1, Ordering::AcqRel) == 1 {
                        token.cancel();
                    }
                }),
                || (fixture.handler)(fixture.search(100), fixture.manifest.id),
            )
        });
        // @step Then expensive history work is not started or resumed
        assert_eq!(
            visits.load(Ordering::Acquire),
            if before_start { 0 } else { 2 }
        );
        // @step And cancellation is explicit rather than an empty successful search
        assert!(
            matches!(result, SessionSearchResult::Error { message } if message.contains("cancel"))
        );
    }
    support::queued_abandonment_and_default_policy().await?;
    Ok(())
}

#[test]
#[serial_test::serial]
fn scenario_cancellation_interrupts_large_original_and_blob_reads() -> TestResult {
    use codelet_common::read_work::{observe, ReadCancellation};
    use std::sync::atomic::{AtomicUsize, Ordering};
    // @step Given a search resolving a large stored original or blob
    let fixture = Fixture::new(1)?;
    let content = "synthetic large content ".repeat(16_000);
    let hash = persistence::store_blob(content.as_bytes())?;
    let mut store = persistence::MessageStore::new()?;
    let id = store.store("user", &content)?;
    for boundary in ["blob.bytes", "message.bytes"] {
        let cancellation = ReadCancellation::default();
        let token = cancellation.clone();
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = reads.clone();
        let (result, _) = cancellation.run(|| {
            observe(
                Arc::new(move |current| {
                    if current == boundary && observed.fetch_add(1, Ordering::AcqRel) == 1 {
                        // @step When its invocation is cancelled between bounded byte reads
                        token.cancel();
                    }
                }),
                || {
                    if boundary == "blob.bytes" {
                        persistence::get_blob(&hash).map(|_| ())
                    } else {
                        get_message(id).map(|_| ())
                    }
                },
            )
        });
        // @step Then further reads stop and all storage ownership is released
        assert_eq!(reads.load(Ordering::Acquire), 2);
        // @step And no partial original or blob is published as successful evidence
        assert!(result.is_err());
        assert!(get_message(fixture.manifest.messages[0].message_id)?.is_some());
    }
    assert_eq!(persistence::get_blob(&hash)?, content.as_bytes());
    support::cancelled_index_publication(fixture.directory.path(), id)?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
#[serial_test::serial]
async fn scenario_uncancelled_history_access_preserves_its_contracts() -> TestResult {
    // @step Given real history with blobs and multiple sessions
    let fixture = Fixture::new(5)?;
    let blob = persistence::store_blob(b"blob needle")?;
    let mut other = fixture.other.clone();
    append_message(&mut other, "user", &persistence::make_blob_reference(&blob))?;
    // @step When uncancelled search show recent and originals actions run
    let search = (fixture.handler)(fixture.search(2), fixture.manifest.id);
    let show = (fixture.handler)(fixture.show(), fixture.manifest.id);
    let recent = (fixture.handler)(
        SessionSearchAction::Recent { count: Some(1) },
        fixture.manifest.id,
    );
    let originals = persistence::originals::retrieve_originals(serde_json::from_value(
        serde_json::json!({"session_id": fixture.manifest.id.to_string(), "limit": 2}),
    )?)?;
    // @step Then filtering trimming limits routing and integrity checks remain intact
    assert!(matches!(
        search,
        SessionSearchResult::Search {
            total_matches: 2,
            ..
        }
    ));
    assert!(
        matches!(show, SessionSearchResult::Session { messages, .. } if messages.len() == 2 && messages[0].turn_index == 2)
    );
    assert!(matches!(recent, SessionSearchResult::Recent { sessions } if sessions.len() == 1));
    assert_eq!(originals.records.len(), 2);
    assert_eq!(originals.next_turn, Some(2));
    // @step And hook denial prevents dispatch while unrelated async work remains responsive
    let id = fixture.manifest.id;
    set_session_search_handler(id, Some(fixture.handler.clone()));
    codelet_tools::pre_tool_hook::register_pre_tool_hook(
        id,
        Arc::new(|_, _, _| {
            codelet_tools::pre_tool_hook::PreToolHookDecision::Deny("synthetic denial".into())
        }),
    );
    let result = SessionSearchTool::new(id)
        .call(SessionSearchArgs {
            action: fixture.search(1),
        })
        .await;
    codelet_tools::pre_tool_hook::unregister_pre_tool_hook(id);
    set_session_search_handler(id, None);
    assert!(result.is_err());
    assert_eq!(tokio::spawn(async { 7 }).await?, 7);
    assert_eq!(
        std::fs::read(
            fixture
                .directory
                .path()
                .join("blobs")
                .join(&blob[..2])
                .join(&blob)
        )?,
        b"blob needle"
    );
    Ok(())
}
