//! Stub for the `generate-summary-report` fspec command. See RPC-235 for the port work unit.
//! Original TypeScript implementation: src/commands/generate-summary-report.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "generate-summary-report",
        work_unit: "RPC-235",
    })
}
