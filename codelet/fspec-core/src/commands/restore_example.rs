//! Stub for the `restore-example` fspec command. See RPC-289 for the port work unit.
//! Original TypeScript implementation: src/commands/restore-example.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "restore-example",
        work_unit: "RPC-289",
    })
}
