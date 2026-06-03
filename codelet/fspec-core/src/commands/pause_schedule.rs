//! Stub for the `pause-schedule` fspec command. See RPC-254 for the port work unit.
//! Original TypeScript implementation: src/commands/schedule/pause-schedule.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "pause-schedule",
        work_unit: "RPC-254",
    })
}
