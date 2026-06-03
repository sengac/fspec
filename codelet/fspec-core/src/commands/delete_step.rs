//! Stub for the `delete-step` fspec command. See RPC-221 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-step.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-step",
        work_unit: "RPC-221",
    })
}
