//! Stub for the `remove-rule` fspec command. See RPC-279 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-rule.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-rule",
        work_unit: "RPC-279",
    })
}
