//! BUG-435: test files for the temporal gate, found through the linked
//! features' coverage in any language (not only TypeScript `__tests__`).

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::prefill::linked_feature_files;

/// Test files mapped in the coverage of `id`'s linked features that mention
/// `id`. A feature without a coverage file maps nothing, and a mapped file
/// that no longer exists cannot have been written early; any other read or
/// parse failure is an error naming the path.
pub(super) fn mapped_test_files(project_root: &Path, id: &str) -> Result<Vec<PathBuf>, String> {
    let features = linked_feature_files(project_root, id).map_err(|err| err.to_string())?;
    let mut files: Vec<PathBuf> = Vec::new();
    for (feature, _) in features {
        let coverage = PathBuf::from(format!("{}.coverage", feature.display()));
        let raw = match std::fs::read_to_string(&coverage) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("cannot read {}: {error}", coverage.display())),
        };
        let data: Value = serde_json::from_str(&raw)
            .map_err(|error| format!("cannot parse {}: {error}", coverage.display()))?;
        let mapped = data["scenarios"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|scenario| scenario["testMappings"].as_array().into_iter().flatten())
            .filter_map(|mapping| mapping["file"].as_str());
        for file in mapped {
            let path = project_root.join(file);
            if !files.contains(&path) {
                files.push(path);
            }
        }
    }
    let mut out = Vec::new();
    for path in files {
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("cannot read test file {}: {error}", path.display())),
        };
        if content.contains(id) {
            out.push(path);
        }
    }
    Ok(out)
}
