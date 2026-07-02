//! One-shot diagnostic probe against a live fspec WS daemon.
//!
//! Usage: cargo run --release -p codelet-fspec-tui --example probe_pause -- ws://127.0.0.1:37999
//!
//! Prints every session's info, then get_pause_state / get_hitl_request
//! for each session. Diagnostic scratch tool — not shipped.

use codelet_fspec_tui::{FspecBackend, WebSocketFspecBackend};
use url::Url;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url_str = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "ws://127.0.0.1:37999".to_string());
    let url = Url::parse(&url_str)?;
    let backend = WebSocketFspecBackend::connect(url).await?;

    let sessions = backend.list_sessions().await?;
    println!("sessions: {}", sessions.len());
    for s in &sessions {
        println!(
            "--- session id={} name={:?} status={} provider={:?} model={:?} messages={}",
            s.id, s.name, s.status, s.provider_id, s.model_id, s.message_count
        );
        let sid = codelet_rpc_types::SessionId {
            value: s.id.clone(),
        };
        let pause = backend.get_pause_state(sid.clone()).await;
        println!("    pause_state: {pause:?}");
        let hitl = backend.get_hitl_request(sid).await;
        println!("    hitl_request: {hitl:?}");
    }
    Ok(())
}
