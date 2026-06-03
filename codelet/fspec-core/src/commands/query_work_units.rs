//! Stub for the `query-work-units` fspec command. See RPC-263 for the port work unit.
//! Original TypeScript implementation: src/commands/query-work-units.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "query-work-units",
        work_unit: "RPC-263",
    })
}
