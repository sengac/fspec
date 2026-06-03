//! Stub for the `add-attachment` fspec command. See RPC-170 for the port work unit.
//! Original TypeScript implementation: src/commands/add-attachment.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-attachment",
        work_unit: "RPC-170",
    })
}
