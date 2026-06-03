//! Stub for the `list-virtual-hooks` fspec command. See RPC-252 for the port work unit.
//! Original TypeScript implementation: src/commands/list-virtual-hooks.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-virtual-hooks",
        work_unit: "RPC-252",
    })
}
