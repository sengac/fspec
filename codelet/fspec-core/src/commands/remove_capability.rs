//! Stub for the `remove-capability` fspec command. See RPC-269 for the port work unit.
//! Original TypeScript implementation: src/commands/register-remove-capability.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-capability",
        work_unit: "RPC-269",
    })
}
