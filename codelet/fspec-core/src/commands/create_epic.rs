//! Stub for the `create-epic` fspec command. See RPC-211 for the port work unit.
//! Original TypeScript implementation: src/commands/create-epic.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "create-epic",
        work_unit: "RPC-211",
    })
}
