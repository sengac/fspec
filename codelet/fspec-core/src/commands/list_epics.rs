//! Stub for the `list-epics` fspec command. See RPC-243 for the port work unit.
//! Original TypeScript implementation: src/commands/list-epics.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-epics",
        work_unit: "RPC-243",
    })
}
