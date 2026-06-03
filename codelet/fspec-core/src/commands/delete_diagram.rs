//! Stub for the `delete-diagram` fspec command. See RPC-216 for the port work unit.
//! Original TypeScript implementation: src/commands/delete-diagram.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "delete-diagram",
        work_unit: "RPC-216",
    })
}
