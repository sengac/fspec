//! Stub for the `add-tag-to-scenario` fspec command. See RPC-194 for the port work unit.
//! Original TypeScript implementation: src/commands/add-tag-to-scenario.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-tag-to-scenario",
        work_unit: "RPC-194",
    })
}
