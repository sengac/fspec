//! Stub for the `add-rule` fspec command. See RPC-189 for the port work unit.
//! Original TypeScript implementation: src/commands/add-rule.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-rule",
        work_unit: "RPC-189",
    })
}
