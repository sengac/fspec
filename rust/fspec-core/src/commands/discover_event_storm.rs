//! `discover-event-storm` — Rust port of `src/commands/discover-event-storm.ts` (RPC-225).
//!
//! READ-ONLY command. Validates that the target work unit exists and is in
//! `specifying` status, then emits the Event Storm discovery guidance wrapped
//! in a `<system-reminder>` block. It performs NO writes — `spec/work-units.json`
//! must be byte-identical after the call.
//!
//! ## Missing-file behaviour — Option B (INLINE existsSync + read, NO auto-create)
//!
//! The TS implementation checks `existsSync(workUnitsPath)` and exits with
//! `spec/work-units.json not found. Run fspec init first.` WITHOUT creating
//! the file (`src/commands/discover-event-storm.ts:34-38`). We therefore do
//! NOT use [`crate::io::ensure::ensure_work_units_file`] (which auto-creates);
//! instead we inline the existence check, read, and parse.
//!
//! ## Inlined helpers (per port decisions)
//!
//! * `EVENT_STORM_GUIDANCE_PREFIX` / `EVENT_STORM_GUIDANCE_SUFFIX` +
//!   `event_storm_guidance()` — the guidance string returned by the TS
//!   `getEventStormSection()` (`src/utils/slashCommandSections/eventStorm.ts`),
//!   split around the Research-First Workflow block which is rendered
//!   per-mode (CLI-015).
//! * [`wrap_in_system_reminder`] — private analog of the TS
//!   `wrapInSystemReminder` (`src/utils/system-reminder.ts`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `rust/fspec/src/discover_event_storm.rs` is JSON marshalling +
//! stdout/stderr rendering only — no domain logic.

use std::path::Path;

use serde::Deserialize;

use crate::error::FspecCoreError;
use crate::types::work_unit::WorkUnitsData;

/// CLI arguments accepted by `discover-event-storm`. Mirrors the TS
/// `DiscoverEventStormOptions` interface at `src/commands/discover-event-storm.ts:16-19`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoverEventStormArgs {
    work_unit_id: String,
}

mod guidance;
use guidance::{event_storm_guidance, wrap_in_system_reminder};

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: DiscoverEventStormArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "discover-event-storm",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = crate::io::spec_paths::work_units(project_root)?;

    // Option B — INLINE existsSync check, NO auto-create. Mirrors TS
    // `src/commands/discover-event-storm.ts:34-38`.
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "discover-event-storm",
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }

    // Read + parse (escalate malformed JSON, parity with TS JSON.parse throw).
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "discover-event-storm",
        source,
    })?;
    let data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?;

    // Validate work unit exists (mirrors src/commands/discover-event-storm.ts:44-47).
    let wu = match data.work_units.get(&args.work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "discover-event-storm",
                reason: format!("Work unit {} not found", args.work_unit_id),
            });
        }
    };

    // Validate work unit is in specifying status (mirrors
    // src/commands/discover-event-storm.ts:52-65). The two TS `output.error`
    // lines are combined into one error reason (newline-joined) so both
    // surface through the single dispatcher error channel.
    let status_str = wu.status.as_str();
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "discover-event-storm",
            reason: format!(
                "Work unit {id} must be in specifying status (currently: {status})\n  Run: fspec update-work-unit-status {id} specifying",
                id = args.work_unit_id,
                status = status_str
            ),
        });
    }

    // Emit Event Storm guidance as a system-reminder (mirrors
    // src/commands/discover-event-storm.ts:68-83). READ-ONLY: no write.
    let reminder = wrap_in_system_reminder(&format!(
        "EVENT STORM DISCOVERY - {id}\n\n{guidance}\n\nWork unit: {id}\n\nUse the commands listed above to capture Event Storm artifacts.\nWhen done, run: fspec generate-example-mapping-from-event-storm {id}",
        id = args.work_unit_id,
        guidance = event_storm_guidance()
    ));

    Ok(format!(
        "✓ Event Storm discovery session started for {id}\n{reminder}",
        id = args.work_unit_id
    ))
}

#[cfg(test)]
mod tests;
