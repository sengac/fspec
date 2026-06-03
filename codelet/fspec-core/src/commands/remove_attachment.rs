//! Stub for the `remove-attachment` fspec command. See RPC-268 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-attachment.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-attachment",
        work_unit: "RPC-268",
    })
}
