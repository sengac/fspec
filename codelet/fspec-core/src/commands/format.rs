//! `format` — Rust port of `src/commands/format.ts` (RPC-230).
//!
//! Parses each target feature file to a Gherkin AST (via the lenient parser
//! in [`crate::io::gherkin`]) and re-emits it through the hand-ported
//! AST-based formatter ([`crate::io::gherkin_format`]), which reproduces
//! `src/utils/gherkin-formatter.ts`. Returns the JSON envelope
//! `{formattedCount}`.
//!
//! ## Behaviour
//! - With no `file` argument: glob `spec/features/**/*.feature`. Empty → return
//!   `{formattedCount: 0}` with NO error. Files that fail to parse are SKIPPED
//!   (no abort); only successfully formatted files increment the count.
//! - With a `file` argument: format only that file. A missing file surfaces
//!   the error `File not found: <file>` (parity with TS `access()` ENOENT).
//!
//! ## Two-front-doors
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge owns
//! all rendering decisions.

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;
use crate::io::gherkin_format::format_feature;

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct FormatArgs {
    file: Option<String>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: FormatArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "format",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Resolve the list of relative files to format.
    let files: Vec<String> = if let Some(file) = args.file.as_deref() {
        // Single-file mode: the file MUST exist (TS access() ENOENT branch).
        let abs = project_root.join(file);
        if !abs.exists() {
            // Surface the message VERBATIM (no wrapping prefix) so the CLI
            // bridge can print `Error: File not found: <file>` byte-for-byte
            // (parity with TS `throw new Error('File not found: <file>')`).
            return Err(FspecCoreError::Message(format!("File not found: {file}")));
        }
        vec![file.to_string()]
    } else {
        // All-files mode: glob spec/features/**/*.feature. Missing directory
        // → empty list (formattedCount=0, no error).
        match glob_feature_files(project_root) {
            Ok(f) => f,
            Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
            Err(other) => return Err(other),
        }
    };

    if files.is_empty() {
        return ok(0);
    }

    let mut formatted_count: u64 = 0;

    for file in &files {
        let abs = project_root.join(file);
        let content = match std::fs::read_to_string(&abs) {
            Ok(c) => c,
            // In all-files mode a per-file read error is skipped with a
            // warning in TS; here we simply skip (no count increment).
            Err(_) => continue,
        };

        let feature = match parse_feature_lenient(&content) {
            Ok(f) => f,
            // Skip files that fail to parse (TS: continue on parse error).
            Err(_) => continue,
        };

        let formatted = format_feature(&feature, &content);

        std::fs::write(&abs, formatted).map_err(|source| FspecCoreError::Io {
            command: "format",
            source,
        })?;

        formatted_count += 1;
    }

    ok(formatted_count)
}

/// Serialise the `{formattedCount}` envelope.
fn ok(count: u64) -> Result<String, FspecCoreError> {
    let value = json!({ "formattedCount": count });
    serde_json::to_string(&value).map_err(|e| FspecCoreError::InvalidArgs {
        command: "format",
        reason: format!("failed to serialise response: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_file() {
        let a: FormatArgs = serde_json::from_str(r#"{"file":"spec/features/a.feature"}"#).unwrap();
        assert_eq!(a.file.as_deref(), Some("spec/features/a.feature"));
    }

    #[test]
    fn args_default_empty() {
        let a: FormatArgs = serde_json::from_str("{}").unwrap();
        assert!(a.file.is_none());
    }

    #[test]
    fn ok_envelope_shape() {
        let s = ok(3).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["formattedCount"].as_u64(), Some(3));
    }
}
