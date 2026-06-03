//! Stub for the `add-dependencies` fspec command. See RPC-176 for the port work unit.
//! Original TypeScript implementation: src/commands/add-dependencies.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-dependencies",
        work_unit: "RPC-176",
    })
}
