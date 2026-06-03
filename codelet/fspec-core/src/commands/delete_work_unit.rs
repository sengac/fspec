//! Stub for the `delete-work-unit` fspec command. See RPC-223 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-work-unit.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-work-unit",
        work_unit: "RPC-223",
    })
}
