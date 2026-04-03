use super::agent_runner::run_agent_with_interruption;
use crate::interactive_helpers::{compression_ratio, execute_compaction};
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::{
    capture_event, get_debug_capture_manager, handle_debug_command, increment_debug_turn,
    SessionMetadata,
};
use codelet_tui::{create_event_stream, InputQueue};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info};

pub(super) async fn repl_loop(session: &mut Session) -> Result<()> {
    let mut input_queue = InputQueue::new();
    let is_interrupted = Arc::new(AtomicBool::new(false));

    println!("Enter your prompt (or 'exit' to quit):");

    loop {
        // Read user input with provider-prefixed prompt
        print!("{}", session.provider_manager().get_prompt_prefix());
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Check for exit
        if matches!(input, "exit" | "/quit" | "quit") {
            println!("Goodbye!");
            break;
        }

        // Handle /debug command - CLI-022
        if input == "/debug" {
            let result = handle_debug_command();
            // Set session metadata when enabling debug capture
            if result.enabled {
                if let Ok(manager_arc) = get_debug_capture_manager() {
                    if let Ok(mut manager) = manager_arc.lock() {
                        manager.set_session_metadata(SessionMetadata {
                            provider: Some(session.current_provider_name().to_string()),
                            model: session.current_model_id().or_else(|| Some(session.current_provider_name().to_string())),
                            context_window: Some(session.provider_manager().context_window()),
                            max_output_tokens: None,
                        });
                    }
                }
            }
            // CLI-022: Capture command.executed event
            capture_event(
                "command.executed",
                serde_json::json!({
                    "command": "/debug",
                    "result": if result.enabled { "enabled" } else { "disabled" },
                }),
            );
            println!("{}\n", result.message);
            continue;
        }

        // Handle /compact command - NAPI-001: Manual compaction trigger
        if input == "/compact" {
            // Check if there's anything to compact
            if session.messages.is_empty() {
                println!("Nothing to compact - session is empty.\n");
                continue;
            }

            // Get current token count for reporting
            let original_tokens = session.token_tracker.input_tokens;

            // Capture compaction.manual.start event
            capture_event(
                "compaction.manual.start",
                serde_json::json!({
                    "command": "/compact",
                    "originalTokens": original_tokens,
                    "messageCount": session.messages.len(),
                }),
            );

            println!("[Compacting context...]");

            // Use in-view DAG flow. /compact is agent-initiated, pass None for last_user_message.
            let compaction_flag = Arc::new(AtomicBool::new(false));
            match execute_compaction(session, compaction_flag, None).await {
                Ok(()) => {
                    let compacted_tokens = session.token_tracker.input_tokens;
                    let ratio = compression_ratio(original_tokens, compacted_tokens);
                    let compression_pct = ratio * 100.0;

                    // Capture compaction.manual.complete event
                    capture_event(
                        "compaction.manual.complete",
                        serde_json::json!({
                            "command": "/compact",
                            "type": "in-view-dag",
                            "originalTokens": original_tokens,
                            "compactedTokens": compacted_tokens,
                            "compressionRatio": ratio,
                        }),
                    );

                    println!(
                        "[Context compacted: {original_tokens}→{compacted_tokens} tokens, {compression_pct:.0}% compression]"
                    );
                    println!(
                        "[In-view DAG flow — agent will build summary via SessionSearch]\n"
                    );
                    info!(
                        "/compact: {original_tokens}→{compacted_tokens} tokens ({compression_pct:.0}% compression, in-view DAG flow)"
                    );
                }
                Err(e) => {
                    // Capture compaction.manual.failed event
                    capture_event(
                        "compaction.manual.failed",
                        serde_json::json!({
                            "command": "/compact",
                            "error": e.to_string(),
                        }),
                    );

                    error!("/compact failed: {}", e);
                }
            }
            continue;
        }

        // Check for provider switch - CLEARS CONTEXT (CLI-008)
        if input.starts_with('/') {
            let provider = input.trim_start_matches('/');
            // Capture provider.switch event - CLI-022
            capture_event(
                "provider.switch",
                serde_json::json!({
                    "from": session.current_provider_name(),
                    "to": provider,
                }),
            );
            match session.switch_provider(provider) {
                Ok(()) => {
                    info!("Provider switched to: {}", provider);
                    println!("Switched to {provider} provider\n");
                    continue;
                }
                Err(e) => {
                    debug!("Provider switch failed: {}", e);
                    error!("Error switching provider: {e}\n");
                    continue;
                }
            }
        }

        if input.is_empty() {
            continue;
        }

        // Capture user.input event - CLI-022
        capture_event(
            "user.input",
            serde_json::json!({
                "input": input,
                "inputLength": input.len(),
            }),
        );
        // Increment turn for each user input
        increment_debug_turn();

        // Run agent with interruption support and persistent context (CLI-008)
        // Enable raw mode only during agent execution for ESC key detection
        is_interrupted.store(false, Ordering::Relaxed);
        enable_raw_mode()?;
        let mut event_stream = create_event_stream();

        let agent_result = run_agent_with_interruption(
            session,
            input,
            &mut event_stream,
            &mut input_queue,
            is_interrupted.clone(),
        )
        .await;

        // Always disable raw mode after agent completes
        disable_raw_mode()?;

        match agent_result {
            Ok(()) => println!("\n"),
            Err(e) => error!("Error: {e}\n"),
        }
    }

    Ok(())
}
