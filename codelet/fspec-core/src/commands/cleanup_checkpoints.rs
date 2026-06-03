//! Stub for the `cleanup-checkpoints` fspec command. See RPC-203 for the port work unit.
//! Original TypeScript implementation: src/commands/cleanup-checkpoints.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "cleanup-checkpoints",
        work_unit: "RPC-203",
    })
}
