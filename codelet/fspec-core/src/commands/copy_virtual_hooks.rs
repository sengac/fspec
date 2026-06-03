//! Stub for the `copy-virtual-hooks` fspec command. See RPC-209 for the port work unit.
//! Original TypeScript implementation: src/commands/copy-virtual-hooks.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "copy-virtual-hooks",
        work_unit: "RPC-209",
    })
}
