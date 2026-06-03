//! Stub for the `clear-dependencies` fspec command. See RPC-204 for the port work unit.
//! Original TypeScript implementation: src/commands/clear-dependencies.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "clear-dependencies",
        work_unit: "RPC-204",
    })
}
