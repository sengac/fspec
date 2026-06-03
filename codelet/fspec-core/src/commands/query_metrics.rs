//! Stub for the `query-metrics` fspec command. See RPC-261 for the port work unit.
//! Original TypeScript implementation: src/commands/query-metrics.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "query-metrics",
        work_unit: "RPC-261",
    })
}
