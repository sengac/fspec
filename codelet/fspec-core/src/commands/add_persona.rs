//! Stub for the `add-persona` fspec command. See RPC-186 for the port work unit.
//! Original TypeScript implementation: src/commands/register-add-persona.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-persona",
        work_unit: "RPC-186",
    })
}
