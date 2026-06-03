//! Stub for the `remove-persona` fspec command. See RPC-277 for the port work unit.
//! Original TypeScript implementation: src/commands/register-remove-persona.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-persona",
        work_unit: "RPC-277",
    })
}
