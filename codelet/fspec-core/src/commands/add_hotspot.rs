//! Stub for the `add-hotspot` fspec command. See RPC-185 for the port work unit.
//! Original TypeScript implementation: src/commands/add-hotspot.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-hotspot",
        work_unit: "RPC-185",
    })
}
