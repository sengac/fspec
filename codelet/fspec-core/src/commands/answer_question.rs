//! Stub for the `answer-question` fspec command. See RPC-196 for the port work unit.
//! Original TypeScript implementation: src/commands/answer-question.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "answer-question",
        work_unit: "RPC-196",
    })
}
