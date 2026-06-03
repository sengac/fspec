//! Stub for the `show-epic` fspec command. See RPC-302 for the port work unit.
//! Original TypeScript implementation: src/commands/show-epic.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "show-epic",
        work_unit: "RPC-302",
    })
}
