//! Stub for the `add-question` fspec command. See RPC-188 for the port work unit.
//! Original TypeScript implementation: src/commands/add-question.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "add-question",
        work_unit: "RPC-188",
    })
}
