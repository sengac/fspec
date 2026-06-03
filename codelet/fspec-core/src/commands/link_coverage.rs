//! Stub for the `link-coverage` fspec command. See RPC-240 for the port work unit.
//! Original TypeScript implementation: src/commands/link-coverage.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "link-coverage",
        work_unit: "RPC-240",
    })
}
