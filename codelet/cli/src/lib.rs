//! CLI Interface bounded context
//!
//! Command parsing, configuration, entry points using clap v4.

pub mod compaction_threshold; // CLI-020: Autocompact buffer for compaction threshold
pub mod context; // Context management - token tracking
pub mod diff; // Diff rendering with diffy (CLI-006)
pub mod highlight; // Bash syntax highlighting with tree-sitter (CLI-006)
mod interactive;
mod interactive_helpers; // Compaction helpers for interactive mode (CLI-010)
pub mod large_write_intent; // CLI-019: Large write intent detection and chunking guidance
pub mod session; // Session management

#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::{Parser, Subcommand};
use codelet_core::RigAgent;
use futures::StreamExt;
use tracing::error;

/// Codelet - A multi-provider AI coding agent
#[derive(Debug, Parser)]
#[clap(name = "codelet", version, about)]
pub struct Cli {
    /// The prompt to send to the agent
    #[arg(value_name = "PROMPT")]
    pub prompt: Option<String>,

    /// Model to use (e.g., claude-3-5-sonnet, gpt-4o)
    #[arg(short, long)]
    pub model: Option<String>,

    /// Provider to use (anthropic, openai, google)
    #[arg(short, long)]
    pub provider: Option<String>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Subcommand to run
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// Available subcommands
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run the agent in non-interactive mode
    Exec {
        /// The prompt to execute
        prompt: String,
    },
    /// Generate shell completions
    Completion {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Show configuration
    Config {
        /// Show config file path
        #[arg(long)]
        path: bool,
    },
}

/// Main CLI entry point
pub async fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Exec { prompt }) => run_agent(&prompt, cli.provider.as_deref()).await,
        Some(Commands::Completion { shell }) => {
            generate_completions(shell);
            Ok(())
        }
        Some(Commands::Config { path }) => {
            if path {
                if let Some(config_dir) = dirs::config_dir() {
                    println!(
                        "{}",
                        config_dir.join("codelet").join("config.toml").display()
                    );
                } else {
                    println!("Could not determine config directory");
                }
            }
            Ok(())
        }
        None => {
            if let Some(prompt) = cli.prompt {
                run_agent(&prompt, cli.provider.as_deref()).await
            } else {
                // Run interactive TUI mode
                interactive::run_interactive_mode(cli.provider.as_deref()).await
            }
        }
    }
}

/// Run the agent with a prompt (with streaming)
async fn run_agent(prompt: &str, provider_name: Option<&str>) -> Result<()> {
    use codelet_providers::ProviderManager;

    // Use ProviderManager to select provider
    let manager = if let Some(name) = provider_name {
        ProviderManager::with_provider(name)?
    } else {
        ProviderManager::new()?
    };

    // Create rig agent and stream based on selected provider type
    // Each provider has a different agent type, so we handle them separately
    match manager.current_provider_name() {
        "claude" => {
            let provider = manager.get_claude()?;
            let rig_agent = provider.create_rig_agent();
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream(agent, prompt).await
        }
        "openai" => {
            let provider = manager.get_openai()?;
            let rig_agent = provider.create_rig_agent();
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream(agent, prompt).await
        }
        "codex" => {
            let provider = manager.get_codex()?;
            let rig_agent = provider.create_rig_agent();
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream(agent, prompt).await
        }
        "gemini" => {
            let provider = manager.get_gemini()?;
            let rig_agent = provider.create_rig_agent();
            let agent = RigAgent::with_default_depth(rig_agent);
            run_agent_stream(agent, prompt).await
        }
        _ => Err(anyhow::anyhow!("Unknown provider")),
    }
}

/// Stream agent responses (generic over completion model type)
async fn run_agent_stream<M>(agent: RigAgent<M>, prompt: &str) -> Result<()>
where
    M: rig::completion::CompletionModel,
{
    use rig::agent::MultiTurnStreamItem;
    use rig::streaming::StreamedAssistantContent;
    use std::io::Write;

    // Stream the response in real-time
    let mut stream = agent.prompt_streaming(prompt).await;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::Text(text))) => {
                print!("{}", text.text);
                std::io::stdout().flush()?;
            }
            Ok(MultiTurnStreamItem::StreamAssistantItem(StreamedAssistantContent::ToolCall(_))) => {
                // Tool calls handled automatically by rig multi-turn
            }
            Ok(MultiTurnStreamItem::StreamUserItem(_)) => {
                // Tool results sent back to LLM
            }
            Ok(MultiTurnStreamItem::FinalResponse(_)) => {
                // Stream complete
            }
            Ok(_) => {
                // Other stream items we don't need to handle
            }
            Err(e) => {
                error!(error = %e, "Agent execution failed");
                return Err(e);
            }
        }
    }

    println!(); // Final newline
    Ok(())
}

/// Generate shell completions
fn generate_completions(shell: clap_complete::Shell) {
    use clap::CommandFactory;
    clap_complete::generate(
        shell,
        &mut Cli::command(),
        "codelet",
        &mut std::io::stdout(),
    );
}
