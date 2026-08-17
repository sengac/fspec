//! End-to-end runtime reproduction against a live fspec WS daemon.
//!
//! Creates a NEW session, sends "read the .env file", and observes:
//! - every StreamChunk from chunks_rx (variant names)
//! - every (SessionId, SessionStatus) from status_changes_rx
//! - get_pause_state polls
//!
//! On pause detection (or timeout) sends pause_triple(Deny) to unblock,
//! then exits. Diagnostic scratch tool — not shipped.

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use codelet_rpc_types::ApprovalChoice;
use std::time::Instant;
use url::Url;

fn variant_name(chunk: &codelet_rpc_types::StreamChunk) -> String {
    let dbg = format!("{chunk:?}");
    dbg.chars().take(120).collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url_str = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:37999".to_string());
    let url = Url::parse(&url_str)?;
    let backend = WebSocketFspecBackend::connect(url).await?;

    let mut chunks = backend.chunks_rx();
    let mut statuses = backend.status_changes_rx();

    let sid = backend.create_session(None).await?;
    println!("created session: {}", sid.value);

    backend
        .send_input(
            sid.clone(),
            "Use the Read tool to read the file .env in the project root. Do not use Bash."
                .to_string(),
        )
        .await?;
    println!("input sent; observing for 90s...");

    let my_sid = sid.clone();
    // Discriminator: late subscriber created mid-pause. If it receives the
    // Paused chunk, the send happened AFTER subscription time.
    let late_backend = &backend;
    let mut late_rx: Option<
        tokio::sync::broadcast::Receiver<(
            codelet_rpc_types::SessionId,
            codelet_rpc_types::StreamChunk,
        )>,
    > = None;
    let t0 = Instant::now();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(90);
    let mut poll = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut denied = false;

    loop {
        tokio::select! {
            r = chunks.recv() => {
                match r {
                    Ok((id, chunk)) => {
                        if id.value == my_sid.value {
                            println!("[{:>8.3}s] [chunk] {}", t0.elapsed().as_secs_f64(), variant_name(&chunk));
                        }
                    }
                    Err(e) => { println!("[chunk] recv err: {e}"); break; }
                }
            }
            r = statuses.recv() => {
                if let Ok((id, st)) = r {
                    if id.value == my_sid.value {
                        println!("[{:>8.3}s] [status] {st:?}", t0.elapsed().as_secs_f64());
                    }
                }
            }
            _ = poll.tick() => {
                let pause = backend.get_pause_state(my_sid.clone()).await;
                println!("[{:>8.3}s] [poll] pause_state: {pause:?}", t0.elapsed().as_secs_f64());
                let secs = t0.elapsed().as_secs();
                if secs >= 12 && late_rx.is_none() {
                    late_rx = Some(late_backend.chunks_rx());
                    println!("[{:>8.3}s] [late] subscribed fresh chunks_rx", t0.elapsed().as_secs_f64());
                }
                if let Some(rx) = late_rx.as_mut() {
                    while let Ok((id, c)) = rx.try_recv() {
                        if id.value == my_sid.value {
                            println!("[{:>8.3}s] [late-chunk] {}", t0.elapsed().as_secs_f64(), variant_name(&c));
                        }
                    }
                }
                if (8..14).contains(&secs) {
                    match backend.get_buffered_output(my_sid.clone(), 1000).await {
                        Ok(chunks) => {
                            println!("[{:>8.3}s] [buffer] {} chunks:", t0.elapsed().as_secs_f64(), chunks.len());
                            for c in chunks.iter() {
                                println!("    [buffer] {}", variant_name(c));
                            }
                        }
                        Err(e) => println!("[buffer] err: {e}"),
                    }
                }
                if secs > 30 { if let Ok(Some(_)) = pause {
                    if !denied {
                        println!("[poll] pause detected -> sending Deny to unblock");
                        let r = backend.pause_triple(my_sid.clone(), ApprovalChoice::Deny).await;
                        println!("[poll] pause_triple(Deny) -> {r:?}");
                        denied = true;
                    } }
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                println!("timeout reached");
                break;
            }
        }
        if denied && tokio::time::Instant::now() + std::time::Duration::from_secs(60) > deadline {
            // keep observing a bit after deny, then bail early
        }
    }

    // Final state dump.
    let pause = backend.get_pause_state(my_sid.clone()).await;
    println!("final pause_state: {pause:?}");
    if let Ok(chunks) = backend.get_buffered_output(my_sid.clone(), 10000).await {
        println!("final buffer ({} chunks):", chunks.len());
        for c in chunks.iter() {
            println!("    [final-buffer] {}", variant_name(c));
        }
    }
    Ok(())
}
