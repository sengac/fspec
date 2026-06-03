//! Stub for the `check` fspec command. See RPC-201 for the port work unit.
//! Original TypeScript implementation: src/commands/check.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "check",
        work_unit: "RPC-201",
    })
}
