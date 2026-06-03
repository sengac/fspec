//! Stub for the `add-scenario` fspec command. See RPC-190 for the port work unit.
//! Original TypeScript implementation: src/commands/add-scenario.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-scenario",
        work_unit: "RPC-190",
    })
}
