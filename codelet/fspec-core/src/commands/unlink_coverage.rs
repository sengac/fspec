//! Stub for the `unlink-coverage` fspec command. See RPC-311 for the port work unit.
//! Original TypeScript implementation: src/commands/unlink-coverage.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "unlink-coverage",
        work_unit: "RPC-311",
    })
}
