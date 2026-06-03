//! Stub for the `delete-scenario` fspec command. See RPC-219 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-scenario.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-scenario",
        work_unit: "RPC-219",
    })
}
