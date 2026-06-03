//! Stub for the `add-capability` fspec command. See RPC-173 for the port work unit.
//! Original TypeScript implementation: src/commands/register-add-capability.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-capability",
        work_unit: "RPC-173",
    })
}
