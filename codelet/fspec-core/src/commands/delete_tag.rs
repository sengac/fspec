//! Stub for the `delete-tag` fspec command. See RPC-222 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-tag.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-tag",
        work_unit: "RPC-222",
    })
}
