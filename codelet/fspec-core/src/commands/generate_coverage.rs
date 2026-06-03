//! Stub for the `generate-coverage` fspec command. See RPC-231 for the port work unit.
//! Original TypeScript implementation: src/commands/generate-coverage.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "generate-coverage",
        work_unit: "RPC-231",
    })
}
