//! Stub for the `list-tags` fspec command. See RPC-251 for the port work unit.
//! Original TypeScript implementation: src/commands/list-tags.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-tags",
        work_unit: "RPC-251",
    })
}
