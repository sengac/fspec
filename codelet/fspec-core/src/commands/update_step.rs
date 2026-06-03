//! Stub for the `update-step` fspec command. See RPC-315 for the port work unit.
//! Original TypeScript implementation: src/commands/update-step.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "update-step",
        work_unit: "RPC-315",
    })
}
