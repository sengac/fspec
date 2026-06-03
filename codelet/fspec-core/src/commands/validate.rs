//! Stub for the `validate` fspec command. See RPC-320 for the port work unit.
//! Original TypeScript implementation: src/commands/validate.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "validate",
        work_unit: "RPC-320",
    })
}
