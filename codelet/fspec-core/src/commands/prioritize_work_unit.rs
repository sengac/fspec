//! Stub for the `prioritize-work-unit` fspec command. See RPC-255 for the port work unit.
//! Original TypeScript implementation: src/commands/prioritize-work-unit.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "prioritize-work-unit",
        work_unit: "RPC-255",
    })
}
