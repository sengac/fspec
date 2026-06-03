//! Stub for the `list-work-units` fspec command. See RPC-253 for the port work unit.
//! Original TypeScript implementation: src/commands/list-work-units.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-work-units",
        work_unit: "RPC-253",
    })
}
