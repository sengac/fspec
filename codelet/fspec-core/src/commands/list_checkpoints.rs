//! Stub for the `list-checkpoints` fspec command. See RPC-242 for the port work unit.
//! Original TypeScript implementation: src/commands/list-checkpoints.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-checkpoints",
        work_unit: "RPC-242",
    })
}
