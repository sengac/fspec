//! Stub for the `export-dependencies` fspec command. See RPC-227 for the port work unit.
//! Original TypeScript implementation: src/commands/export-dependencies.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "export-dependencies",
        work_unit: "RPC-227",
    })
}
