//! Feature: spec/features/responsive-session-search.feature
use codelet_tools::session_search::types::{
    SessionSearchAction, SessionSearchArgs, SessionSearchResult,
};
use codelet_tools::session_search::{set_session_search_handler, SessionSearchTool};
use rig::tool::Tool;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use uuid::Uuid;

#[tokio::test(flavor = "current_thread")]
async fn search_does_not_block_async_workers() -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a SessionSearch handler waiting for storage
    let session = Uuid::new_v4();
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    set_session_search_handler(
        session,
        Some(Arc::new(move |_, _| {
            started_tx.send(()).expect("signal storage wait");
            release_rx
                .lock()
                .expect("release channel")
                .recv()
                .expect("release storage");
            SessionSearchResult::Recent { sessions: vec![] }
        })),
    );
    let (heartbeat_tx, heartbeat_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    let controller = std::thread::spawn(move || {
        started_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("search started");
        ready_tx
            .send(())
            .expect("tell async heartbeat search is blocked");
        let responsive = heartbeat_rx
            .recv_timeout(Duration::from_millis(500))
            .is_ok();
        release_tx.send(()).expect("release blocked search");
        responsive
    });
    // @step When the tool runs on a single async worker
    let search = tokio::spawn(async move {
        SessionSearchTool::new(session)
            .call(SessionSearchArgs {
                action: SessionSearchAction::Recent { count: Some(1) },
            })
            .await
    });
    let heartbeat = tokio::spawn(async move {
        ready_rx.await.expect("search wait barrier");
        let _ = heartbeat_tx.send(()); // Controller records an expired heartbeat as failure.
    });
    let result = search.await?;
    heartbeat.await?;
    let responsive = controller.join().map_err(|_| "controller failed")?;
    set_session_search_handler(session, None);
    // @step Then unrelated async work continues and the search result is preserved
    assert!(responsive, "storage blocked the only async worker");
    assert!(result?.contains("recent"));
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn denied_search_does_not_dispatch_a_blocking_handler(
) -> Result<(), Box<dyn std::error::Error>> {
    use codelet_tools::pre_tool_hook::{
        register_pre_tool_hook, unregister_pre_tool_hook, PreToolHookDecision,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    let session = Uuid::new_v4();
    let called = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&called);
    set_session_search_handler(
        session,
        Some(Arc::new(move |_, _| {
            flag.store(true, Ordering::SeqCst);
            SessionSearchResult::Recent { sessions: vec![] }
        })),
    );
    register_pre_tool_hook(
        session,
        Arc::new(|_, _, _| PreToolHookDecision::Deny("fixture denial".into())),
    );
    let result = SessionSearchTool::new(session)
        .call(SessionSearchArgs {
            action: SessionSearchAction::Recent { count: None },
        })
        .await;
    unregister_pre_tool_hook(session);
    set_session_search_handler(session, None);
    assert!(result.is_err());
    assert!(
        !called.load(Ordering::SeqCst),
        "denied calls must not enter blocking storage"
    );
    Ok(())
}
