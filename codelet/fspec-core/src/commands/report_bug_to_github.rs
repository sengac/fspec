//! Stub for the `report-bug-to-github` fspec command. See RPC-285 for the port work unit.
//! Original TypeScript implementation: src/commands/report-bug-to-github.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "report-bug-to-github",
        work_unit: "RPC-285",
    })
}
