use super::stream_loop::run_agent_stream_with_interruption;
use crate::session::Session;
use anyhow::Result;
use codelet_core::RigAgent;
use codelet_tui::{InputQueue, TuiEvent};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(super) async fn run_agent_with_interruption(
    session: &mut Session,
    prompt: &str,
    event_stream: &mut (dyn futures::Stream<Item = TuiEvent> + Unpin + Send),
    input_queue: &mut InputQueue,
    is_interrupted: Arc<AtomicBool>,
) -> Result<()> {
    // Get provider name before mutable borrow (to satisfy borrow checker)
    let provider_name = session.current_provider_name().to_string();
    let manager = session.provider_manager_mut();

    // Macro to eliminate code duplication across provider branches (DRY principle)
    // PROV-006: Pass preamble to enable cache_control for API key mode
    macro_rules! run_with_provider {
        ($get_provider:ident, $preamble:expr) => {{
            let provider = manager.$get_provider()?;
            let rig_agent = provider.create_rig_agent($preamble);
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream_with_interruption(
                agent,
                prompt,
                session, // Pass entire session for token tracking and compaction (CLI-010)
                event_stream,
                input_queue,
                is_interrupted,
            )
            .await
        }};
    }

    // Dispatch to provider-specific agent
    // PROV-006: For now pass None - preamble comes from session.messages as user messages
    // Future enhancement: extract CLAUDE.md content as preamble for API key mode
    match provider_name.as_str() {
        "claude" => run_with_provider!(get_claude, None),
        "openai" => run_with_provider!(get_openai, None),
        "codex" => run_with_provider!(get_codex, None),
        "gemini" => run_with_provider!(get_gemini, None),
        _ => Err(anyhow::anyhow!("Unknown provider")),
    }
}


