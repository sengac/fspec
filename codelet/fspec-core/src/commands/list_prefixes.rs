//! Stub for the `list-prefixes` fspec command. See RPC-248 for the port work unit.
//! Original TypeScript implementation: src/commands/list-prefixes.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "list-prefixes",
        work_unit: "RPC-248",
    })
}
