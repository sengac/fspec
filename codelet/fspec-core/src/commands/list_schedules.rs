//! Stub for the `list-schedules` fspec command. See RPC-250 for the port work unit.
//! Original TypeScript implementation: src/commands/schedule/list-schedules.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-schedules",
        work_unit: "RPC-250",
    })
}
