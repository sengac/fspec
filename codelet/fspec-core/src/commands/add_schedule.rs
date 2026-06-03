//! Stub for the `add-schedule` fspec command. See RPC-191 for the port work unit.
//! Original TypeScript implementation: src/commands/schedule/add-schedule.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-schedule",
        work_unit: "RPC-191",
    })
}
