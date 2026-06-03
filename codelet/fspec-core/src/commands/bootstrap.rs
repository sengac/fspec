//! Stub for the `bootstrap` fspec command. See RPC-200 for the port work unit.
//! Original TypeScript implementation: src/commands/bootstrap.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "bootstrap",
        work_unit: "RPC-200",
    })
}
