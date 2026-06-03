//! Stub for the `export-example-map` fspec command. See RPC-228 for the port work unit.
//! Original TypeScript implementation: src/commands/export-example-map.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "export-example-map",
        work_unit: "RPC-228",
    })
}
