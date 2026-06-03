//! Stub for the `query-dependency-stats` fspec command. See RPC-257 for the port work unit.
//! Original TypeScript implementation: src/commands/query-dependency-stats.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "query-dependency-stats",
        work_unit: "RPC-257",
    })
}
