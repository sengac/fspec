//! Stub for the `restore-rule` fspec command. See RPC-291 for the port work unit.
//! Original TypeScript implementation: src/commands/restore-rule.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "restore-rule",
        work_unit: "RPC-291",
    })
}
