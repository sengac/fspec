//! Minimal reproduction of the delayed `SessionStateChange { Paused }`
//! chunk (RPC-406 follow-up investigation).
//!
//! Constructs a real `BackgroundSession`, registers the SAME pause
//! handler closure `agent_loop.rs:500-518` registers, then calls
//! `pause_for_user` from inside a spawned tokio task (exactly how a rig
//! tool's async `call()` invokes it — blocking the worker thread).
//! Watches the chunks + typed-status broadcasts with timestamps, denies
//! after 3s, and reports WHEN the Paused chunk was delivered relative
//! to the pause window. Diagnostic scratch — not shipped.

use std::sync::Arc;
use std::time::Instant;

use codelet_providers::ProviderManager;
use codelet_rpc_types::SessionStatus;
use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::tool_pause::{
    pause_for_user, set_pause_handler, PauseHandler, PauseKind, PauseRequest, PauseResponse,
    PauseState,
};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let t0 = Instant::now();
    let elapsed = move || t0.elapsed().as_secs_f64();

    let tmp = std::env::temp_dir().join(format!("repro-pause-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    codelet_common::set_data_directory(tmp.clone())?;
    let provider_manager =
        ProviderManager::with_provider("claude").map_err(|e| format!("{e:?}"))?;
    let inner = codelet_cli::session::Session::from_provider_manager(provider_manager);

    let (chunks_tx, mut chunks_rx) =
        broadcast::channel::<(codelet_rpc_types::SessionId, codelet_rpc_types::StreamChunk)>(1024);
    let (status_tx, mut status_rx) = broadcast::channel::<(
        codelet_rpc_types::SessionId,
        codelet_rpc_types::SessionStatus,
    )>(1024);
    let (input_tx, _input_rx) = mpsc::channel(8);

    let id = Uuid::new_v4();
    let session = Arc::new(BackgroundSession::new(
        id,
        "repro".to_string(),
        "/tmp".to_string(),
        None,
        None,
        inner,
        input_tx,
        None,
        None,
        None,
        chunks_tx,
        status_tx,
    ));

    // Watchers.
    tokio::spawn(async move {
        while let Ok((_, chunk)) = chunks_rx.recv().await {
            println!("[{:>7.3}s] [chunk] {:?}", t0.elapsed().as_secs_f64(), chunk);
        }
    });
    tokio::spawn(async move {
        while let Ok((_, st)) = status_rx.recv().await {
            println!("[{:>7.3}s] [status] {st:?}", t0.elapsed().as_secs_f64());
        }
    });

    // Verbatim agent_loop.rs:500-518 pause handler.
    let session_for_pause = session.clone();
    let pause_handler: PauseHandler = Arc::new(move |request: PauseRequest| {
        let state = PauseState {
            kind: request.kind,
            tool_name: request.tool_name.clone(),
            message: request.message.clone(),
            details: request.details,
        };
        session_for_pause.set_pause_state(Some(state));
        session_for_pause.set_status(SessionStatus::Paused);

        let response = session_for_pause.wait_for_pause_response();

        session_for_pause.set_status(SessionStatus::Running);

        response
    });
    set_pause_handler(id, Some(pause_handler));

    // Simulate the running turn (status Running like agent_loop does).
    session.set_status(SessionStatus::Running);

    // "Tool" task: calls pause_for_user synchronously inside an async task,
    // blocking its worker thread — exactly like ReadTool::call() does.
    let tool = tokio::spawn(async move {
        println!("[{:>7.3}s] [tool] calling pause_for_user", 0.05);
        let resp = pause_for_user(
            id,
            PauseRequest {
                kind: PauseKind::Triple,
                tool_name: "Read".to_string(),
                message: "Environment files often contain secrets".to_string(),
                details: Some("/tmp/.env".to_string()),
            },
        );
        println!("[tool] pause_for_user returned {resp:?}");
    });

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    println!("[{:>7.3}s] [main] sending Denied", elapsed());
    session.send_pause_response(PauseResponse::Denied);

    let _ = tool.await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    println!("[{:>7.3}s] [main] done", elapsed());
    Ok(())
}
