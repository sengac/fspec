# AST Research: Shell Job Execution Patterns

## Key Functions Found

### trigger_shell_job stub (engine.rs:300-303)
- Currently a stub that logs and returns Ok(())
- Signature: `async fn trigger_shell_job(name: &str) -> Result<(), anyhow::Error>`
- Needs to be updated to take `project_path` and `entry` params (matching trigger_agent_job)

### trigger_and_update routing (engine.rs:229-255)
- Routes by job_type: "agent" → trigger_agent_job, "shell" → trigger_shell_job
- Updates last_run_at and last_run_status after execution
- Status mapping: Ok(()) → "success", Err → "error"

### ShellConfig type (types.rs:41-44)
- Already exists with `command: String` field
- Part of ScheduleEntry.shell (Option<ShellConfig>)

### update_last_run (engine.rs:258-276)
- Reads schedules.json, updates entry, writes back
- Uses Utc::now().to_rfc3339() for timestamp

### trigger_agent_job pattern (engine.rs:280-297)
- Takes (name, project_path, entry)
- Resolves default_model from SessionManager
- Delegates to agent_job module

## Pattern to Follow for shell_job.rs
- Similar to agent_job.rs but simpler
- No session management needed
- Direct tokio::process::Command execution
- Return type should match Ok(())/Err convention for trigger_and_update
