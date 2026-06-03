//! Stub for the `audit-coverage` fspec command. See RPC-197 for the port work unit.
//! Original TypeScript implementation: src/commands/audit-coverage.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "audit-coverage",
        work_unit: "RPC-197",
    })
}
