//! Stub for the `suggest-dependencies` fspec command. See RPC-309 for the port work unit.
//! Original TypeScript implementation: src/commands/suggest-dependencies.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "suggest-dependencies",
        work_unit: "RPC-309",
    })
}
