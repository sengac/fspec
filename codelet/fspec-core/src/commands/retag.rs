//! Stub for the `retag` fspec command. See RPC-293 for the port work unit.
//! Original TypeScript implementation: src/commands/retag.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "retag",
        work_unit: "RPC-293",
    })
}
