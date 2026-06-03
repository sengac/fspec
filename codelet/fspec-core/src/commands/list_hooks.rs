//! Stub for the `list-hooks` fspec command. See RPC-247 for the port work unit.
//! Original TypeScript implementation: src/commands/list-hooks.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-hooks",
        work_unit: "RPC-247",
    })
}
