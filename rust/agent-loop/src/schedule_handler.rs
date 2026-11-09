//! Schedule handler implementation — SCHED-009
//!
//! Feature: spec/features/schedule-ai-tool.feature
//!
//! Creates the handler closure that reads/writes spec/schedules.json,
//! dispatching on action (add, list, pause, resume, remove).
//! Registered per-session in session_manager.rs agent_loop.
//!
//! File locking (BUG-477): every action holds `codelet_common::spec_lock`'s
//! spec lock — the cross-process lock every fspec command takes — and
//! writes replace schedules.json atomically (unique temp file, fsync,
//! rename, directory fsync).

use std::sync::Arc;

use serde_json::{json, Value};

use codelet_tools::schedule::types::{ScheduleRequest, ScheduleResult};
use codelet_tools::ScheduleHandler;

/// Create a schedule handler closure for a given project directory.
///
/// The returned handler reads/writes `{project}/spec/schedules.json`
/// and dispatches on the request action.
///
/// # Arguments
/// * `project` - Absolute path to the project root directory
pub fn create_handler(project: String) -> ScheduleHandler {
    Arc::new(move |request: ScheduleRequest| -> ScheduleResult {
        match request.action.as_str() {
            "add" => {
                // Validate before acquiring lock (fail-fast, no lock contention)
                if let Some(early_err) = validate_add_request(&request) {
                    return early_err;
                }
                with_schedules_lock(&project, || handle_add(&project, &request))
            }
            "list" => with_schedules_lock(&project, || handle_list(&project)),
            "pause" => with_schedules_lock(&project, || handle_pause(&project, &request)),
            "resume" => with_schedules_lock(&project, || handle_resume(&project, &request)),
            "remove" => with_schedules_lock(&project, || handle_remove(&project, &request)),
            _ => ScheduleResult::error(&format!("Unknown action: {}", request.action)),
        }
    })
}

// =============================================================================
// File I/O helpers — locking delegates to codelet_common::file_lock (RPC-017)
// =============================================================================

fn schedules_path(project: &str) -> Result<std::path::PathBuf, String> {
    let roots = codelet_common::spec_paths::ProjectRoots::resolve(std::path::Path::new(project))
        .map_err(|e| e.to_string())?;
    Ok(roots.schedules())
}

/// Execute a closure while holding the spec lock (BUG-477), the one
/// cross-process lock shared by every writer of spec/ JSON, adapting a lock
/// failure into a `ScheduleResult::error`.
fn with_schedules_lock<F>(project: &str, f: F) -> ScheduleResult
where
    F: FnOnce() -> ScheduleResult,
{
    let spec_dir = codelet_common::spec_paths::spec_dir_of(std::path::Path::new(project));
    if let Err(e) = std::fs::create_dir_all(&spec_dir) {
        return ScheduleResult::error(&format!("Failed to create schedules dir: {e}"));
    }
    match codelet_common::spec_lock::SpecLock::acquire(&spec_dir) {
        Ok(_spec_lock) => f(),
        Err(e) => ScheduleResult::error(&format!("Failed to lock {}: {e}", spec_dir.display())),
    }
}

fn read_schedules_file(project: &str) -> Result<Value, String> {
    let path = schedules_path(project)?;
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read schedules.json: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse schedules.json: {e}"))
}

/// Write schedules.json atomically using temp-file + rename.
///
/// This prevents partial writes from corrupting the file on crash.
fn write_schedules_file(project: &str, data: &Value) -> Result<(), String> {
    let path = schedules_path(project)?;
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("Failed to serialize schedules: {e}"))?;

    codelet_common::spec_lock::replace_file_atomically(&path, content.as_bytes())
        .map_err(|e| format!("Failed to atomically replace schedules.json: {e}"))
}

// =============================================================================
// Validation helpers
// =============================================================================

/// Validate a cron expression using the croner crate (same as scheduler engine)
fn validate_cron(expr: &str) -> Result<(), String> {
    use croner::Cron;
    Cron::new(expr)
        .parse()
        .map(|_| ())
        .map_err(|e| format!("Invalid cron expression: {e}"))
}

/// Validate an IANA timezone using chrono-tz
fn validate_timezone(tz: &str) -> Result<(), String> {
    tz.parse::<chrono_tz::Tz>()
        .map(|_| ())
        .map_err(|_| format!("Invalid timezone: {tz}"))
}

/// Pre-validate an add request before acquiring the file lock.
///
/// Returns `Some(ScheduleResult)` with an error if validation fails,
/// or `None` if validation passes and we can proceed with the locked operation.
fn validate_add_request(request: &ScheduleRequest) -> Option<ScheduleResult> {
    // Required fields
    let name = match &request.name {
        Some(n) if !n.is_empty() => n,
        _ => return Some(ScheduleResult::error("Schedule name is required")),
    };
    let cron = match &request.cron {
        Some(c) if !c.is_empty() => c,
        _ => return Some(ScheduleResult::error("Cron expression is required")),
    };
    let timezone = match &request.timezone {
        Some(tz) if !tz.is_empty() => tz,
        _ => return Some(ScheduleResult::error("Timezone is required")),
    };
    let job_type = match &request.job_type {
        Some(jt) if !jt.is_empty() => jt,
        _ => return Some(ScheduleResult::error("Job type is required")),
    };

    // Validate cron
    if let Err(e) = validate_cron(cron) {
        return Some(ScheduleResult::error(&e));
    }

    // Validate timezone
    if let Err(e) = validate_timezone(timezone) {
        return Some(ScheduleResult::error(&e));
    }

    // Validate job-type-specific fields
    match job_type.as_str() {
        "agent" => {
            let has_role = request.role.as_ref().is_some_and(|r| !r.is_empty());
            let has_prompt = request.prompt.as_ref().is_some_and(|p| !p.is_empty());
            if !has_role || !has_prompt {
                return Some(ScheduleResult::error(
                    "Agent jobs require role and prompt fields",
                ));
            }
        }
        "shell" => {
            let has_cmd = request.command.as_ref().is_some_and(|c| !c.is_empty());
            if !has_cmd {
                return Some(ScheduleResult::error("Shell jobs require a command field"));
            }
        }
        other => {
            return Some(ScheduleResult::error(&format!(
                "Invalid job_type: {other}. Must be 'agent' or 'shell'"
            )));
        }
    }

    // Suppress unused-variable warnings — we only check presence/format above
    let _ = (name, cron, timezone, job_type);

    None // Validation passed
}

// =============================================================================
// Action handlers
// =============================================================================

fn handle_add(project: &str, request: &ScheduleRequest) -> ScheduleResult {
    // Fields are pre-validated by validate_add_request() — safe to extract
    let name = request.name.as_deref().unwrap_or_default().to_string();
    let cron = request.cron.as_deref().unwrap_or_default().to_string();
    let timezone = request.timezone.as_deref().unwrap_or_default().to_string();
    let job_type = request.job_type.as_deref().unwrap_or_default().to_string();

    // Read existing schedules
    let mut data = match read_schedules_file(project) {
        Ok(d) => d,
        Err(e) => return ScheduleResult::error(&e),
    };

    // Check for duplicate name
    if data["schedules"].get(&name).is_some() {
        return ScheduleResult::error(&format!("Schedule already exists: {name}"));
    }

    // Build the schedule entry
    let overlap_policy = request
        .overlap_policy
        .as_deref()
        .unwrap_or("skip")
        .to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let mut entry = json!({
        "cron": cron,
        "timezone": timezone,
        "status": "active",
        "job_type": job_type,
        "overlap_policy": overlap_policy,
        "created_at": now,
    });

    if job_type == "agent" {
        entry["agent"] = json!({
            "role": request.role.as_deref().unwrap_or(""),
            "prompt": request.prompt.as_deref().unwrap_or(""),
        });
    } else if job_type == "shell" {
        entry["shell"] = json!({
            "command": request.command.as_deref().unwrap_or(""),
        });
    }

    // Insert into schedules
    data["schedules"][&name] = entry.clone();

    // Write back
    if let Err(e) = write_schedules_file(project, &data) {
        return ScheduleResult::error(&e);
    }

    // Build response schedule (camelCase for LLM consistency)
    let response_schedule = json!({
        "name": name,
        "cron": cron,
        "timezone": timezone,
        "jobType": job_type,
        "overlapPolicy": overlap_policy,
        "status": "active",
    });

    ScheduleResult::success_schedule("add", response_schedule)
}

fn handle_list(project: &str) -> ScheduleResult {
    let data = match read_schedules_file(project) {
        Ok(d) => d,
        Err(e) => return ScheduleResult::error(&e),
    };

    let schedules_obj = match data["schedules"].as_object() {
        Some(obj) => obj,
        None => return ScheduleResult::success_list(vec![]),
    };

    let schedules: Vec<Value> = schedules_obj
        .iter()
        .map(|(name, entry)| {
            json!({
                "name": name,
                "cron": entry["cron"],
                "timezone": entry["timezone"],
                "type": entry["job_type"],
                "status": entry["status"],
                "lastRun": entry.get("last_run_at").unwrap_or(&Value::Null),
                "overlapPolicy": entry.get("overlap_policy").unwrap_or(&Value::Null),
            })
        })
        .collect();

    ScheduleResult::success_list(schedules)
}

fn handle_pause(project: &str, request: &ScheduleRequest) -> ScheduleResult {
    let name = match &request.name {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return ScheduleResult::error("Schedule name is required for pause"),
    };

    let mut data = match read_schedules_file(project) {
        Ok(d) => d,
        Err(e) => return ScheduleResult::error(&e),
    };

    if data["schedules"].get(&name).is_none() {
        return ScheduleResult::error(&format!("Schedule not found: {name}"));
    }

    data["schedules"][&name]["status"] = json!("paused");

    if let Err(e) = write_schedules_file(project, &data) {
        return ScheduleResult::error(&e);
    }

    let schedule = json!({
        "name": name,
        "status": "paused",
    });
    ScheduleResult::success_schedule("pause", schedule)
}

fn handle_resume(project: &str, request: &ScheduleRequest) -> ScheduleResult {
    let name = match &request.name {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return ScheduleResult::error("Schedule name is required for resume"),
    };

    let mut data = match read_schedules_file(project) {
        Ok(d) => d,
        Err(e) => return ScheduleResult::error(&e),
    };

    if data["schedules"].get(&name).is_none() {
        return ScheduleResult::error(&format!("Schedule not found: {name}"));
    }

    data["schedules"][&name]["status"] = json!("active");

    if let Err(e) = write_schedules_file(project, &data) {
        return ScheduleResult::error(&e);
    }

    let schedule = json!({
        "name": name,
        "status": "active",
    });
    ScheduleResult::success_schedule("resume", schedule)
}

fn handle_remove(project: &str, request: &ScheduleRequest) -> ScheduleResult {
    let name = match &request.name {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return ScheduleResult::error("Schedule name is required for remove"),
    };

    let mut data = match read_schedules_file(project) {
        Ok(d) => d,
        Err(e) => return ScheduleResult::error(&e),
    };

    let schedules = match data["schedules"].as_object_mut() {
        Some(obj) => obj,
        None => return ScheduleResult::error(&format!("Schedule not found: {name}")),
    };

    if schedules.remove(&name).is_none() {
        return ScheduleResult::error(&format!("Schedule not found: {name}"));
    }

    if let Err(e) = write_schedules_file(project, &data) {
        return ScheduleResult::error(&e);
    }

    ScheduleResult::success_remove(&name)
}
