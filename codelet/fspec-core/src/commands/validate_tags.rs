//! Stub for the `validate-tags` fspec command. See RPC-324 for the port work unit.
//! Original TypeScript implementation: src/commands/validate-tags.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "validate-tags",
        work_unit: "RPC-324",
    })
}
