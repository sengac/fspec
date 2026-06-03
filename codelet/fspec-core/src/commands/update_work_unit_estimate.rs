//! Stub for the `update-work-unit-estimate` fspec command. See RPC-318 for the port work unit.
//! Original TypeScript implementation: src/commands/update-work-unit-estimate.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "update-work-unit-estimate",
        work_unit: "RPC-318",
    })
}
