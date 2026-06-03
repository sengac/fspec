//! Stub for the `query-estimation-guide` fspec command. See RPC-259 for the port work unit.
//! Original TypeScript implementation: src/commands/query-estimation-guide.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "query-estimation-guide",
        work_unit: "RPC-259",
    })
}
