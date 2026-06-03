//! Stub for the `list-feature-tags` fspec command. See RPC-244 for the port work unit.
//! Original TypeScript implementation: src/commands/list-feature-tags.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-feature-tags",
        work_unit: "RPC-244",
    })
}
