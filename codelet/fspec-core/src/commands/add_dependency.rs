//! Stub for the `add-dependency` fspec command. See RPC-177 for the port work unit.
//! Original TypeScript implementation: src/commands/add-dependency.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-dependency",
        work_unit: "RPC-177",
    })
}
