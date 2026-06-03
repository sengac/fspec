//! Stub for the `compact-work-unit` fspec command. See RPC-206 for the port work unit.
//! Original TypeScript implementation: src/commands/compact-work-unit.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "compact-work-unit",
        work_unit: "RPC-206",
    })
}
