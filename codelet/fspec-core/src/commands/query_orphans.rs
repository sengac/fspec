//! Stub for the `query-orphans` fspec command. See RPC-262 for the port work unit.
//! Original TypeScript implementation: src/commands/query-orphans.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "query-orphans",
        work_unit: "RPC-262",
    })
}
