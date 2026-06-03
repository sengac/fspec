//! Stub for the `add-diagram` fspec command. See RPC-178 for the port work unit.
//! Original TypeScript implementation: src/commands/add-diagram.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-diagram",
        work_unit: "RPC-178",
    })
}
