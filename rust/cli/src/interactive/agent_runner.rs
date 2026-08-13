use super::output::CliOutput;
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
    // CONT-002: this is a real user message — reset the zero-progress nudge
    // count, and capture the armed state before the provider-manager borrow.
    // CONT-003: a goal implies auto-continue (derived mode Goal), and the
    // goal is synced into the done() registry alongside the armed flag.
    crate::interactive::auto_continue::reset_for_new_user_turn(session);
    let continue_armed = session.continue_enabled || session.goal.is_some();
    let goal_spec = session.goal.as_ref().map(|g| codelet_tools::GoalSpec {
        text: g.text.clone(),
        verify: g.verify.clone(),
    });

    // Get provider name before mutable borrow (to satisfy borrow checker)
    let provider_name = session.current_provider_name().to_string();
    let manager = session.provider_manager_mut();

    // CLI output handler
    let output = CliOutput;

    // CLI callers create a local compaction_in_progress flag
    // (CLI doesn't have a BackgroundSession with a persistent flag)
    let compaction_in_progress = Arc::new(AtomicBool::new(false));

    // TOOL-012: Generate session_id for tool handler lookup
    // In CLI interactive mode, tools like Fspec won't have handlers registered,
    // but we still need a session_id for API consistency. Tools will fail
    // gracefully with "handler not configured" if invoked.
    let session_id = uuid::Uuid::new_v4();

    // CONT-002: sync the per-session armed registry immediately before
    // create_rig_agent so done() is registered only while armed.
    // CONT-003: sync the goal at the same dispatch site so DoneTool's
    // definition() and Tier 1/2 checks see it.
    codelet_tools::set_continue_armed(session_id, continue_armed);
    codelet_tools::set_session_goal(session_id, goal_spec);

    // Macro to eliminate code duplication across provider branches (DRY principle)
    // PROV-006: Pass preamble to enable cache_control for API key mode
    // TOOL-012: Pass session_id as first parameter
    macro_rules! run_with_provider {
        ($get_provider:ident, $preamble:expr) => {{
            let provider = manager.$get_provider()?;
            // TOOL-010: Pass None for thinking_config in CLI (keywords not supported yet)
            // TOOL-012: Pass session_id as first parameter
            let rig_agent = provider.create_rig_agent(session_id, $preamble, None);
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream_with_interruption(
                agent,
                prompt,
                session, // Pass entire session for token tracking and compaction (CLI-010)
                event_stream,
                input_queue,
                is_interrupted,
                compaction_in_progress.clone(),
                &output,
                session_id,
            )
            .await
        }};
    }

    // Dispatch to provider-specific agent
    // PROV-006: For now pass None - preamble comes from session.messages as user messages
    // Future enhancement: extract CLAUDE.md content as preamble for API key mode
    // PROV-051: get_openai requires session_id for cache optimization headers
    let result = match provider_name.as_str() {
        "claude" => run_with_provider!(get_claude, None),
        "openai" => {
            let provider = manager.get_openai(session_id)?;
            let rig_agent = provider.create_rig_agent(session_id, None, None);
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream_with_interruption(
                agent,
                prompt,
                session,
                event_stream,
                input_queue,
                is_interrupted,
                compaction_in_progress.clone(),
                &output,
                session_id,
            )
            .await
        }
        "codex" => run_with_provider!(get_codex, None),
        "gemini" => run_with_provider!(get_gemini, None),
        _ => Err(anyhow::anyhow!("Unknown provider")),
    };

    // CONT-002: clear the per-message armed-registry entry at stream end
    // (the CLI generates a fresh session_id per message; this also clears
    // any unread done() acceptance for that id).
    // CONT-003: clear the per-message goal entry too — Session.goal is the
    // source of truth and is re-synced on the next dispatch.
    codelet_tools::set_continue_armed(session_id, false);
    codelet_tools::set_session_goal(session_id, None);

    result
}
