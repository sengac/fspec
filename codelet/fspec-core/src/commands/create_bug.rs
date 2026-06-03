//! Stub for the `create-bug` fspec command. See RPC-210 for the port work unit.
//! Original TypeScript implementation: src/commands/create-bug.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "create-bug",
        work_unit: "RPC-210",
    })
}
