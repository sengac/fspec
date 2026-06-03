//! Stub for the `remove-schedule` fspec command. See RPC-280 for the port work unit.
//! Original TypeScript implementation: src/commands/schedule/remove-schedule.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-schedule",
        work_unit: "RPC-280",
    })
}
