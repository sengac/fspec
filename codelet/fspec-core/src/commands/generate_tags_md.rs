//! Stub for the `generate-tags-md` fspec command. See RPC-236 for the port work unit.
//! Original TypeScript implementation: src/commands/generate-tags-md.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "generate-tags-md",
        work_unit: "RPC-236",
    })
}
