//! Stub for the `list-foundation-sections` fspec command. See RPC-246 for the port work unit.
//! Original TypeScript implementation: src/commands/list-foundation-sections.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-foundation-sections",
        work_unit: "RPC-246",
    })
}
