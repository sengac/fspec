//! Agent job execution — RPC-058 lift from
//! rust/napi/src/scheduler/agent_job.rs.
//!
//! When the scheduler engine determines an agent-type schedule should fire,
//! this module spawns a subordinate session via [`SchedulerHooks`] with the
//! configured role, initial prompt, and the user's default model.

use super::types::{AgentConfig, ScheduleEntry};
use super::{Hooks, ScheduleTrigger};
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
    hooks: Hooks,
) -> Result<(), anyhow::Error> {
    let agent_config = entry
        .agent
        .as_ref()
        .context("Missing agent configuration")?;

    trigger_agent_job(name, project_path, agent_config, default_model, hooks).await
}

/// Trigger an agent job by spawning a new session via [`SchedulerHooks`].
pub async fn trigger_agent_job(
    name: &str,
    project_path: &str,
    config: &AgentConfig,
    default_model: &str,
    hooks: Hooks,
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
    let session_name = format!("[scheduled] {name} — {timestamp}");

    info!(
        "Spawning agent session: name={}, model={}, schedule={}",
        session_name, default_model, name
    );

    let trigger = ScheduleTrigger {
        name: name.to_string(),
        project_path: project_path.to_string(),
        default_model: default_model.to_string(),
        role: config.role.clone(),
        prompt,
        session_id,
        session_name: session_name.clone(),
    };

    hooks
        .spawn_scheduled_session(trigger)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to spawn scheduled session for '{name}': {e}"))?;

    info!("Agent session spawned successfully: schedule={}", name);
    Ok(())
}
