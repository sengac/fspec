use super::agent_runner::run_agent_with_interruption;
use crate::session::Session;
use anyhow::Result;
use codelet_common::debug_capture::{
    get_debug_capture_manager, handle_debug_command, SessionMetadata,
};
use codelet_tui::{create_event_stream, InputQueue};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

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
                            model: Some(session.current_provider_name().to_string()),
                            context_window: Some(session.provider_manager().context_window()),
                            max_output_tokens: None,
                        });
                    }
                }
            }
            // CLI-022: Capture command.executed event
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "command.executed",
                            serde_json::json!({
                                "command": "/debug",
                                "result": if result.enabled { "enabled" } else { "disabled" },
                            }),
                            None,
                        );
                    }
                }
            }
            println!("{}\n", result.message);
            continue;
        }

        // Check for provider switch - CLEARS CONTEXT (CLI-008)
        if input.starts_with('/') {
            let provider = input.trim_start_matches('/');
            // Capture provider.switch event - CLI-022
            if let Ok(manager_arc) = get_debug_capture_manager() {
                if let Ok(mut manager) = manager_arc.lock() {
                    if manager.is_enabled() {
                        manager.capture(
                            "provider.switch",
                            serde_json::json!({
                                "from": session.current_provider_name(),
                                "to": provider,
                            }),
                            None,
                        );
                    }
                }
            }
            match session.switch_provider(provider) {
                Ok(()) => {
                    info!("Provider switched to: {}", provider);
                    println!("Switched to {provider} provider\n");
                    continue;
                }
                Err(e) => {
                    debug!("Provider switch failed: {}", e);
                    eprintln!("Error switching provider: {e}\n");
                    continue;
                }
            }
        }

        if input.is_empty() {
            continue;
        }

        // Capture user.input event - CLI-022
        if let Ok(manager_arc) = get_debug_capture_manager() {
            if let Ok(mut manager) = manager_arc.lock() {
                if manager.is_enabled() {
                    manager.capture(
                        "user.input",
                        serde_json::json!({
                            "input": input,
                            "inputLength": input.len(),
                        }),
                        None,
                    );
                    // Increment turn for each user input
                    manager.increment_turn();
                }
            }
        }

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
            Err(e) => eprintln!("Error: {e}\n"),
        }
    }

    Ok(())
}
