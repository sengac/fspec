//! Interactive TUI mode entry point
//!
//! Main REPL loop coordinating terminal events, agent streaming, and user input.
//! Based on OpenAI codex architecture with tokio::select! pattern.

mod agent_runner;
mod message_helpers;
pub mod output;
mod repl_loop;
mod stream_handlers;
pub mod stream_loop;

pub use output::{CliOutput, ContextFillInfo, StreamEvent, StreamOutput, TokenInfo, ToolCallEvent, ToolResultEvent};
pub use stream_loop::run_agent_stream;

use crate::session::Session;
use anyhow::Result;
use repl_loop::repl_loop;

pub async fn run_interactive_mode(provider_name: Option<&str>) -> Result<()> {
    // Initialize session with persistent context (CLI-008)
    let mut session = Session::new(provider_name)?;

    // CLI-016: Inject context reminders (CLAUDE.md discovery + environment info)
    session.inject_context_reminders();

    // Display startup card
    display_startup_card(&session)?;

    // Main REPL loop (raw mode is enabled/disabled per-request, not globally)
    let result = repl_loop(&mut session).await;

    result
}

fn display_startup_card(session: &Session) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    println!("\nCodelet v{version}");

    let manager = session.provider_manager();
    if !manager.has_any_provider() {
        println!("Available models: No providers configured");
        println!("Please set ANTHROPIC_API_KEY, OPENAI_API_KEY, or other credentials\n");
    } else {
        let providers = manager.list_available_providers();
        println!("Available models: {}\n", providers.join(", "));
    }

    Ok(())
}
