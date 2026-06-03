//! Stub for the `list-scenario-tags` fspec command. See RPC-249 for the port work unit.
//! Original TypeScript implementation: src/commands/list-scenario-tags.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-scenario-tags",
        work_unit: "RPC-249",
    })
}
