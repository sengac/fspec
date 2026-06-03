//! Stub for the `register-tag` fspec command. See RPC-265 for the port work unit.
//! Original TypeScript implementation: src/commands/register-tag.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "register-tag",
        work_unit: "RPC-265",
    })
}
