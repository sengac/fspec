//! Stub for the `import-example-map` fspec command. See RPC-238 for the port work unit.
//! Original TypeScript implementation: src/commands/import-example-map.ts

use crate::error::FspecCoreError;

pub async fn run(_args_json: &str) -> Result<String, FspecCoreError> {
    Err(FspecCoreError::NotYetPorted {
        command: "import-example-map",
        work_unit: "RPC-238",
    })
}
