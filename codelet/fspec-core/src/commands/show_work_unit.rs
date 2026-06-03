//! Stub for the `show-work-unit` fspec command. See RPC-308 for the port work unit.
//! Original TypeScript implementation: src/commands/show-work-unit.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-work-unit",
        work_unit: "RPC-308",
    })
}
