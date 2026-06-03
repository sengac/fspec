//! Stub for the `remove-tag-from-scenario` fspec command. See RPC-282 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-tag-from-scenario.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-tag-from-scenario",
        work_unit: "RPC-282",
    })
}
