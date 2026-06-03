//! Stub for the `update-scenario` fspec command. See RPC-314 for the port work unit.
//! Original TypeScript implementation: src/commands/update-scenario.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "update-scenario",
        work_unit: "RPC-314",
    })
}
