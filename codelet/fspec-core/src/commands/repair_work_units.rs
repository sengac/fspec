//! Stub for the `repair-work-units` fspec command. See RPC-284 for the port work unit.
//! Original TypeScript implementation: src/commands/repair-work-units.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "repair-work-units",
        work_unit: "RPC-284",
    })
}
