//! Schedule CRUD operations — RPC-058 lift from
//! codelet/napi/src/schedule_handler.rs.
//!
//! These five entry points (schedule_add / list / pause / resume /
//! remove) back the new RPC surface introduced by RPC-058. They reuse
//! the file-lock + atomic write protocol that the NAPI
//! [`ScheduleHandler`] already implements (compatible with the
//! TypeScript `proper-lockfile` protocol via
//! [`codelet_common::file_lock::with_file_lock`]).
//!
//! Returns flow uses `Result<*, String>` per the RPC-057 convention so
//! error messages propagate cleanly through the wire types layer.

use std::path::{Path, PathBuf};

use codelet_common::file_lock::with_file_lock;
use codelet_rpc_types::ScheduledJob;
use serde_json::{json, Value};

// =============================================================================
// File-system helpers
// =============================================================================

fn schedules_path(project: &str) -> PathBuf {
    Path::new(project).join("spec/schedules.json")
}

fn lock_dir_path(project: &str) -> PathBuf {
    Path::new(project).join("spec/schedules.json.lock")
}

/// Execute a closure while holding the schedules.json file lock.
fn with_schedules_lock<F, T>(project: &str, f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let lock_dir = lock_dir_path(project);
    match with_file_lock(&lock_dir, || -> Result<Result<T, String>, String> {
        Ok(f())
    }) {
        Ok(result) => result,
        Err(e) => Err(e),
    }
}

fn read_schedules_file(project: &str) -> Result<Value, String> {
    let path = schedules_path(project);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            // Treat missing file as the empty schedules document so list/add
            // both work against a fresh project root.
            return Ok(json!({"version": "1.0", "schedules": {}}));
        }
    };
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse schedules.json: {e}"))
}

fn write_schedules_file(project: &str, data: &Value) -> Result<(), String> {
    let path = schedules_path(project);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create schedules dir: {e}"))?;
    }
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize schedules: {e}"))?;
    let temp_path = path.with_extension(format!("json.tmp.{}", uuid::Uuid::new_v4()));
    std::fs::write(&temp_path, &content)
        .map_err(|e| format!("Failed to write temp schedules file: {e}"))?;
    std::fs::rename(&temp_path, &path).map_err(|e| {
        let _ = std::fs::remove_file(&temp_path);
        format!("Failed to atomically replace schedules.json: {e}")
    })
}

// =============================================================================
// Validation helpers
// =============================================================================

fn validate_cron(expr: &str) -> Result<(), String> {
    use croner::Cron;
    Cron::new(expr)
        .parse()
        .map(|_| ())
        .map_err(|e| format!("Invalid cron expression: {e}"))
}

fn validate_timezone(tz: &str) -> Result<(), String> {
    tz.parse::<chrono_tz::Tz>()
        .map(|_| ())
        .map_err(|_| format!("Invalid timezone: {tz}"))
}

fn validate_add(job: &ScheduledJob) -> Result<(), String> {
    if job.name.is_empty() {
        return Err("Schedule name is required".to_string());
    }
    if job.cron.is_empty() {
        return Err("Cron expression is required".to_string());
    }
    if job.timezone.is_empty() {
        return Err("Timezone is required".to_string());
    }
    if job.job_type.is_empty() {
        return Err("Job type is required".to_string());
    }

    validate_cron(&job.cron)?;
    validate_timezone(&job.timezone)?;

    match job.job_type.as_str() {
        "agent" => {
            let has_role = job.role.as_ref().is_some_and(|r| !r.is_empty());
            let has_prompt = job.prompt.as_ref().is_some_and(|p| !p.is_empty());
            if !has_role || !has_prompt {
                return Err("Agent jobs require role and prompt fields".to_string());
            }
        }
        "shell" => {
            let has_cmd = job.command.as_ref().is_some_and(|c| !c.is_empty());
            if !has_cmd {
                return Err("Shell jobs require a command field".to_string());
            }
        }
        other => {
            return Err(format!(
                "Invalid job_type: {other}. Must be 'agent' or 'shell'"
            ));
        }
    }
    Ok(())
}

// =============================================================================
// Wire-type ↔ schedules.json mapping
// =============================================================================

fn entry_to_job(name: &str, entry: &Value) -> ScheduledJob {
    let agent = entry.get("agent").and_then(|v| v.as_object());
    let shell = entry.get("shell").and_then(|v| v.as_object());
    ScheduledJob {
        name: name.to_string(),
        cron: entry["cron"].as_str().unwrap_or_default().to_string(),
        timezone: entry["timezone"].as_str().unwrap_or_default().to_string(),
        job_type: entry["job_type"].as_str().unwrap_or_default().to_string(),
        status: entry["status"].as_str().unwrap_or_default().to_string(),
        created_at: entry
            .get("created_at")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_run_at: entry
            .get("last_run_at")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_run_status: entry
            .get("last_run_status")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        role: agent
            .and_then(|a| a.get("role"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        prompt: agent
            .and_then(|a| a.get("prompt"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        command: shell
            .and_then(|s| s.get("command"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        overlap_policy: entry
            .get("overlap_policy")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }
}

// =============================================================================
// Public CRUD entry points
// =============================================================================

/// Add a new schedule. Mirrors `ScheduleHandler::handle_add` semantics.
pub fn schedule_add(project: &str, job: ScheduledJob) -> Result<ScheduledJob, String> {
    validate_add(&job)?;
    with_schedules_lock(project, || {
        let mut data = read_schedules_file(project)?;

        if data["schedules"].get(&job.name).is_some() {
            return Err(format!("Schedule already exists: {}", job.name));
        }

        let overlap_policy = job
            .overlap_policy
            .clone()
            .unwrap_or_else(|| "skip".to_string());
        let now = chrono::Utc::now().to_rfc3339();

        let mut entry = json!({
            "cron": job.cron,
            "timezone": job.timezone,
            "status": "active",
            "job_type": job.job_type,
            "overlap_policy": overlap_policy,
            "created_at": now,
        });

        if job.job_type == "agent" {
            entry["agent"] = json!({
                "role": job.role.clone().unwrap_or_default(),
                "prompt": job.prompt.clone().unwrap_or_default(),
            });
        } else if job.job_type == "shell" {
            entry["shell"] = json!({
                "command": job.command.clone().unwrap_or_default(),
            });
        }

        if data["schedules"].as_object().is_none() {
            data["schedules"] = json!({});
        }
        data["schedules"][&job.name] = entry.clone();

        write_schedules_file(project, &data)?;

        Ok(ScheduledJob {
            name: job.name,
            cron: job.cron,
            timezone: job.timezone,
            job_type: job.job_type,
            status: "active".to_string(),
            created_at: Some(now),
            last_run_at: None,
            last_run_status: None,
            role: job.role,
            prompt: job.prompt,
            command: job.command,
            overlap_policy: Some(overlap_policy),
        })
    })
}

/// List every schedule.
pub fn schedule_list(project: &str) -> Result<Vec<ScheduledJob>, String> {
    with_schedules_lock(project, || {
        let data = read_schedules_file(project)?;
        let schedules_obj = match data["schedules"].as_object() {
            Some(obj) => obj,
            None => return Ok(Vec::new()),
        };
        let mut out: Vec<ScheduledJob> = schedules_obj
            .iter()
            .map(|(name, entry)| entry_to_job(name, entry))
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    })
}

/// Pause a schedule by name.
pub fn schedule_pause(project: &str, name: &str) -> Result<ScheduledJob, String> {
    with_schedules_lock(project, || {
        let mut data = read_schedules_file(project)?;
        if data["schedules"].get(name).is_none() {
            return Err(format!("Schedule not found: {name}"));
        }
        data["schedules"][name]["status"] = json!("paused");
        write_schedules_file(project, &data)?;
        Ok(entry_to_job(name, &data["schedules"][name]))
    })
}

/// Resume a schedule by name.
pub fn schedule_resume(project: &str, name: &str) -> Result<ScheduledJob, String> {
    with_schedules_lock(project, || {
        let mut data = read_schedules_file(project)?;
        if data["schedules"].get(name).is_none() {
            return Err(format!("Schedule not found: {name}"));
        }
        data["schedules"][name]["status"] = json!("active");
        write_schedules_file(project, &data)?;
        Ok(entry_to_job(name, &data["schedules"][name]))
    })
}

/// Remove a schedule by name.
pub fn schedule_remove(project: &str, name: &str) -> Result<(), String> {
    with_schedules_lock(project, || {
        let mut data = read_schedules_file(project)?;
        let schedules = match data["schedules"].as_object_mut() {
            Some(obj) => obj,
            None => return Err(format!("Schedule not found: {name}")),
        };
        if schedules.remove(name).is_none() {
            return Err(format!("Schedule not found: {name}"));
        }
        write_schedules_file(project, &data)
    })
}
