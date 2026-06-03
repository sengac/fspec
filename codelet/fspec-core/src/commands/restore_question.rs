//! Stub for the `restore-question` fspec command. See RPC-290 for the port work unit.
//! Original TypeScript implementation: src/commands/restore-question.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "restore-question",
        work_unit: "RPC-290",
    })
}
