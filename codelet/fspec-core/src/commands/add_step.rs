//! Stub for the `add-step` fspec command. See RPC-192 for the port work unit.
//! Original TypeScript implementation: src/commands/add-step.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-step",
        work_unit: "RPC-192",
    })
}
