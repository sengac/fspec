//! Stub for the `restore-checkpoint` fspec command. See RPC-288 for the port work unit.
//! Original TypeScript implementation: src/commands/restore-checkpoint.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "restore-checkpoint",
        work_unit: "RPC-288",
    })
}
