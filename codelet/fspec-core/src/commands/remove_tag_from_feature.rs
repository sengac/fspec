//! Stub for the `remove-tag-from-feature` fspec command. See RPC-281 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-tag-from-feature.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-tag-from-feature",
        work_unit: "RPC-281",
    })
}
