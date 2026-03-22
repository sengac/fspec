//! Agent job execution — SCHED-004
//!
//! When the scheduler engine determines an agent-type schedule should fire,
//! this module spawns a subordinate session via SessionManager with the
//! configured role, initial prompt, and the user's default model.

use super::types::{AgentConfig, ScheduleEntry};
use anyhow::{bail, Context as _};
use chrono::Utc;
use tracing::info;
use uuid::Uuid;

/// Trigger an agent job given its schedule entry.
///
/// Extracts the AgentConfig from the entry and delegates to
/// `trigger_agent_job`. Returns an error if agent config is missing.
pub async fn trigger_agent_job_from_entry(
    name: &str,
    project_path: &str,
    entry: &ScheduleEntry,
    default_model: &str,
) -> Result<(), anyhow::Error> {
    let agent_config = entry
        .agent
        .as_ref()
        .context("Missing agent configuration")?;

    trigger_agent_job(name, project_path, agent_config, default_model).await
}

/// Trigger an agent job by spawning a new session.
///
/// Validates the config, resolves the model, creates a session via
/// SessionManager, sets role (if present), marks schedule metadata,
/// and sends the initial prompt.
pub async fn trigger_agent_job(
    name: &str,
    project_path: &str,
    config: &AgentConfig,
    default_model: &str,
) -> Result<(), anyhow::Error> {
    // Validate model
    if default_model.is_empty() {
        bail!("No default model configured");
    }

    // Validate prompt
    let prompt = match &config.prompt {
        Some(p) if !p.is_empty() => p.clone(),
        Some(_) => bail!("Missing agent prompt"),
        None => bail!("Missing agent prompt"),
    };

    // Generate session ID and name
    let session_id = Uuid::new_v4();
    let timestamp = Utc::now().to_rfc3339();
    let session_name = format!("[scheduled] {} — {}", name, timestamp);

    info!(
        "Spawning agent session: name={}, model={}, schedule={}",
        session_name, default_model, name
    );

    // Access the global SessionManager to spawn the session
    #[cfg(not(feature = "noop"))]
    {
        let session_manager = crate::session_manager::SessionManager::instance();

        // Create the session
        session_manager
            .spawn_scheduled_session(
                &session_id.to_string(),
                default_model,
                project_path,
                &session_name,
                name,
                config.role.as_deref(),
                &prompt,
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to spawn scheduled session for '{}': {}",
                    name,
                    e
                )
            })?;

        info!(
            "Agent session spawned successfully: schedule={}",
            name
        );

        Ok(())
    }

    #[cfg(feature = "noop")]
    {
        let _ = (project_path, &session_name, &session_id, &prompt);
        Err(anyhow::anyhow!("SessionManager not available in noop mode"))
    }
}
