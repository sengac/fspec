//! Stub for the `create-task` fspec command. See RPC-215 for the port work unit.
//! Original TypeScript implementation: src/commands/create-task.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "create-task",
        work_unit: "RPC-215",
    })
}
