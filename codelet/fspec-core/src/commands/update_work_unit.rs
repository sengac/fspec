//! Stub for the `update-work-unit` fspec command. See RPC-317 for the port work unit.
//! Original TypeScript implementation: src/commands/update-work-unit.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "update-work-unit",
        work_unit: "RPC-317",
    })
}
