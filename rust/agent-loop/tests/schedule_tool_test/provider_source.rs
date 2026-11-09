//! BUG-450: a provider's source as its module tree (the file plus its
//! file-backed submodules, e.g. openai.rs + openai/agent.rs).

use std::path::Path;

pub fn read_provider_source(filename: &str) -> Result<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("agent-loop has no parent directory")?
        .join("providers")
        .join("src")
        .join(filename);
    codelet_test_helpers::source_composition::read_tree_or_file(&path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))
}
