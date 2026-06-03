//! Stub for the `show-coverage` fspec command. See RPC-300 for the port work unit.
//! Original TypeScript implementation: src/commands/show-coverage.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-coverage",
        work_unit: "RPC-300",
    })
}
