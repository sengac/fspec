//! Stub for the `checkpoint` fspec command. See RPC-202 for the port work unit.
//! Original TypeScript implementation: src/commands/checkpoint.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "checkpoint",
        work_unit: "RPC-202",
    })
}
