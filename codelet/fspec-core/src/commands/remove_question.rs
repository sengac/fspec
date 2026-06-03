//! Stub for the `remove-question` fspec command. See RPC-278 for the port work unit.
//! Original TypeScript implementation: src/commands/remove-question.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "remove-question",
        work_unit: "RPC-278",
    })
}
