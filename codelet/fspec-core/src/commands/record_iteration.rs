//! Stub for the `record-iteration` fspec command. See RPC-264 for the port work unit.
//! Original TypeScript implementation: src/commands/record-iteration.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "record-iteration",
        work_unit: "RPC-264",
    })
}
