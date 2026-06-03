//! Stub for the `compare-implementations` fspec command. See RPC-207 for the port work unit.
//! Original TypeScript implementation: src/commands/compare-implementations.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "compare-implementations",
        work_unit: "RPC-207",
    })
}
