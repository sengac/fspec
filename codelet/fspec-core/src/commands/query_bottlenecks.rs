//! Stub for the `query-bottlenecks` fspec command. See RPC-256 for the port work unit.
//! Original TypeScript implementation: src/commands/query-bottlenecks.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "query-bottlenecks",
        work_unit: "RPC-256",
    })
}
