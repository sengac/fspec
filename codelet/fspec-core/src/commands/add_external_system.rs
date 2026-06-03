//! Stub for the `add-external-system` fspec command. See RPC-182 for the port work unit.
//! Original TypeScript implementation: src/commands/add-external-system.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-external-system",
        work_unit: "RPC-182",
    })
}
