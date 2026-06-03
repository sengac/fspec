//! Stub for the `dependencies` fspec command. See RPC-224 for the port work unit.
//! Original TypeScript implementation: src/commands/dependencies.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "dependencies",
        work_unit: "RPC-224",
    })
}
