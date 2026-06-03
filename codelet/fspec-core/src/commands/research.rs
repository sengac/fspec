//! Stub for the `research` fspec command. See RPC-286 for the port work unit.
//! Original TypeScript implementation: src/commands/research.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "research",
        work_unit: "RPC-286",
    })
}
