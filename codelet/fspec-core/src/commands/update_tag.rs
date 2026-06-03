//! Stub for the `update-tag` fspec command. See RPC-316 for the port work unit.
//! Original TypeScript implementation: src/commands/update-tag.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "update-tag",
        work_unit: "RPC-316",
    })
}
