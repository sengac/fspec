//! `check` — Rust port of `src/commands/check.ts` (RPC-201).
//!
//! Runs the aggregate validation suite over `spec/features/**/*.feature`:
//!
//!   1. **Gherkin syntax** — every feature file must parse via the lenient
//!      gherkin front-end ([`crate::io::gherkin::parse_feature_lenient`]).
//!   2. **Tag validation** — every feature/scenario tag must be registered
//!      and placed correctly (delegates to the SAME logic the `validate-tags`
//!      command uses, surfaced here through its `{results, validCount,
//!      invalidCount}` envelope).
//!   3. **Formatting** — every feature file is parsed, re-serialised through
//!      the AST-based formatter ([`crate::io::gherkin_format::format_feature`]),
//!      and compared byte-for-byte against the on-disk content. A file whose
//!      bytes differ from its canonical formatting is reported as a `FAIL`
//!      (error: `Formatting check failed: <file> needs formatting`). Files that
//!      fail to parse are skipped here — the Gherkin sub-check already reports
//!      them.
//!
//! ## Formatting sub-check (RPC-332)
//!
//! Now that the gherkin formatter has reached byte-parity (RPC-330), the
//! format sub-check is wired in (replacing the earlier Framing-A `SKIP`
//! divergence). `formatStatus` starts at `"PASS"` and flips to `"FAIL"` only
//! when at least one feature file is not in canonical form. Unlike the TS
//! original — which wraps the whole formatter setup in a try/catch and
//! degrades `formatStatus` to `"SKIP"` on a global throw — the Rust formatter
//! has no such global failure mode, so `formatStatus` is only ever `"PASS"` or
//! `"FAIL"`. Overall success requires a non-`FAIL` Gherkin, tag, AND format
//! status.
//!
//! ## Result envelope
//!
//! Returns `Ok(json)` carrying `{success, gherkinStatus, tagStatus,
//! formatStatus, fileCount, message?, errors?}` — mirroring the TS
//! `CheckResult` interface. The no-feature-files case returns
//! `{success:true, message:"No feature files found", fileCount:0}` with the
//! status fields omitted (parity with `src/commands/check.ts:42-48`). The
//! dispatcher always derives `success=true` from the `Ok`; the *check* result
//! (pass/fail) lives in the `success` field and the CLI bridge maps it to the
//! process exit code.
//!
//! ## Two-front-doors invariant (RPC-003 §7/§11)
//!
//! Both the LLM dispatcher AND the standalone binary's `fspec check` clap
//! subcommand call this single `run`. No parsing, tag-validation, or
//! check-aggregation logic lives in the CLI bridge.

use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::FspecCoreError;
use crate::io::feature_glob::glob_feature_files;
use crate::io::gherkin::parse_feature_lenient;

/// CLI / dispatcher arguments accepted by `check`.
///
/// Parity with the TS Commander.js registration at
/// `src/commands/check.ts:229-234`: a single `-v/--verbose` boolean (default
/// false). The structured payload includes the per-check statuses regardless;
/// `verbose` is accepted for parity and surfaced in the `details` block.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct CheckArgs {
    /// `-v/--verbose`: include a `details` block in the payload.
    verbose: bool,
}

/// Dispatcher entry point. `project_root` is supplied by both front doors.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: CheckArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "check",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- Enumerate feature files ----
    // A missing spec/features directory is treated like an empty glob result
    // (TS tinyglobby returns [] rather than throwing) → the no-files branch.
    let files: Vec<String> = match glob_feature_files(project_root) {
        Ok(v) => v,
        Err(FspecCoreError::DirectoryNotFound { .. }) => Vec::new(),
        Err(other) => return Err(other),
    };

    if files.is_empty() {
        // Parity with src/commands/check.ts:42-48.
        let payload = json!({
            "success": true,
            "message": "No feature files found",
            "fileCount": 0,
        });
        return ok(payload);
    }

    let file_count = files.len();
    let mut errors: Vec<String> = Vec::new();

    // ---- 1. Gherkin syntax ----
    let mut gherkin_status = "PASS";
    for file in &files {
        let abs = project_root.join(file);
        match std::fs::read_to_string(&abs) {
            Ok(content) => {
                if parse_feature_lenient(&content).is_err() {
                    gherkin_status = "FAIL";
                    errors.push(format!("Gherkin syntax error in {file}"));
                }
            }
            Err(e) => {
                gherkin_status = "FAIL";
                errors.push(format!("Gherkin syntax error in {file}: {e}"));
            }
        }
    }

    // ---- 2. Tag validation ----
    // Delegates to the SAME validate-tags implementation (single source of
    // truth) and reads its `{results, validCount, invalidCount}` envelope.
    let mut tag_status = "PASS";
    match crate::commands::validate_tags::run("{}", project_root).await {
        Ok(env_json) => {
            let env: Value =
                serde_json::from_str(&env_json).map_err(|e| FspecCoreError::InvalidArgs {
                    command: "check",
                    reason: format!("failed to parse validate-tags envelope: {e}"),
                })?;
            let invalid_count = env.get("invalidCount").and_then(Value::as_u64).unwrap_or(0);
            if invalid_count > 0 {
                tag_status = "FAIL";
                // Collect per-tag error messages (parity with check.ts:78-84).
                if let Some(results) = env.get("results").and_then(Value::as_array) {
                    for result in results {
                        let valid = result.get("valid").and_then(Value::as_bool).unwrap_or(true);
                        if valid {
                            continue;
                        }
                        if let Some(errs) = result.get("errors").and_then(Value::as_array) {
                            for err in errs {
                                if let Some(msg) = err.get("message").and_then(Value::as_str) {
                                    errors.push(msg.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            tag_status = "FAIL";
            errors.push(format!("Tag validation error: {e}"));
        }
    }

    // ---- 3. Formatting ----
    // For each feature file: parse, re-serialise via the AST formatter, and
    // compare byte-for-byte against the on-disk content. A difference means
    // the file needs formatting. Files that fail to parse are skipped here
    // (the Gherkin sub-check above already reports them). Parity with the TS
    // `check` formatting sub-check at src/commands/check.ts:91-120.
    let mut format_status = "PASS";
    for file in &files {
        let abs = project_root.join(file);
        let content = match std::fs::read_to_string(&abs) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let feature = match parse_feature_lenient(&content) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let formatted = crate::io::gherkin_format::format_feature(&feature, &content);
        if formatted != content {
            format_status = "FAIL";
            errors.push(format!("Formatting check failed: {file} needs formatting"));
        }
    }

    // ---- Determine overall success ----
    // A FAIL in any contributing check fails the run.
    let success = gherkin_status != "FAIL" && tag_status != "FAIL" && format_status != "FAIL";

    let mut payload = json!({
        "success": success,
        "gherkinStatus": gherkin_status,
        "tagStatus": tag_status,
        "formatStatus": format_status,
        "fileCount": file_count,
    });

    if !errors.is_empty() {
        payload["errors"] = json!(errors);
    }
    if success {
        payload["message"] = json!("All checks passed");
    }
    if args.verbose {
        payload["details"] = json!({
            "files": files,
            "gherkinChecked": gherkin_status != "SKIP",
            "tagsChecked": tag_status != "SKIP",
            "formattingChecked": format_status != "SKIP",
        });
    }

    ok(payload)
}

/// Serialize the result envelope.
fn ok(payload: Value) -> Result<String, FspecCoreError> {
    serde_json::to_string(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "check",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use std::fs;

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }

    fn write_tags(root: &Path) {
        let data = json!({
            "categories": [
                { "name": "Component Tags", "description": "", "required": true,
                  "tags": [ { "name": "@comp", "description": "x" } ] },
                { "name": "Feature Group Tags", "description": "", "required": true,
                  "tags": [ { "name": "@grp", "description": "x" } ] },
                { "name": "Technical Tags", "description": "", "required": false, "tags": [] }
            ]
        });
        write(
            root,
            "spec/tags.json",
            &serde_json::to_string_pretty(&data).unwrap(),
        );
    }

    #[tokio::test]
    async fn no_files_reports_success_count_zero() {
        let ws = tempfile::tempdir().unwrap();
        let out = run("{}", ws.path()).await.unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["success"], true);
        assert_eq!(v["fileCount"], 0);
        assert_eq!(v["message"], "No feature files found");
    }

    /// Compute the canonical formatted form of a feature source so test
    /// fixtures can be guaranteed byte-identical to what the format sub-check
    /// expects. Uses the SAME parse→format pipeline the production code uses.
    fn canonical(body: &str) -> String {
        let feature = parse_feature_lenient(body).unwrap();
        crate::io::gherkin_format::format_feature(&feature, body)
    }

    #[tokio::test]
    async fn valid_features_pass() {
        let ws = tempfile::tempdir().unwrap();
        write_tags(ws.path());
        let body = canonical("@comp @grp\nFeature: A\n\n  Scenario: A\n    Given x\n");
        write(ws.path(), "spec/features/a.feature", &body);
        let out = run("{}", ws.path()).await.unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["gherkinStatus"], "PASS");
        assert_eq!(v["tagStatus"], "PASS");
        assert_eq!(v["formatStatus"], "PASS");
        assert_eq!(v["success"], true);
        assert_eq!(v["message"], "All checks passed");
    }

    #[tokio::test]
    async fn well_formatted_feature_format_passes() {
        let ws = tempfile::tempdir().unwrap();
        write_tags(ws.path());
        let body = canonical("@comp @grp\nFeature: Well Formatted\n\n  Scenario: S\n    Given a\n    When b\n    Then c\n");
        write(ws.path(), "spec/features/wf.feature", &body);
        let out = run("{}", ws.path()).await.unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["formatStatus"], "PASS");
        assert_eq!(v["success"], true);
    }

    #[tokio::test]
    async fn unformatted_feature_format_fails() {
        let ws = tempfile::tempdir().unwrap();
        write_tags(ws.path());
        // Valid Gherkin but NOT canonical: extra blank lines / odd indentation.
        write(
            ws.path(),
            "spec/features/messy.feature",
            "@comp @grp\nFeature: Messy\n\n\n  Scenario: S\n        Given a\n",
        );
        let out = run("{}", ws.path()).await.unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["gherkinStatus"], "PASS");
        assert_eq!(v["formatStatus"], "FAIL");
        assert_eq!(v["success"], false);
        let errors = v["errors"].as_array().unwrap();
        assert!(errors.iter().any(|e| e.as_str()
            == Some("Formatting check failed: spec/features/messy.feature needs formatting")));
    }

    #[tokio::test]
    async fn multi_paragraph_description_canonical_passes() {
        let ws = tempfile::tempdir().unwrap();
        write_tags(ws.path());
        // Multi-paragraph feature description (guards RPC-330 regression).
        let body = canonical(
            "@comp @grp\nFeature: Described\n  Paragraph A line one.\n\n  Paragraph B line two.\n\n  Scenario: S\n    Given x\n",
        );
        write(ws.path(), "spec/features/desc.feature", &body);
        let out = run("{}", ws.path()).await.unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["gherkinStatus"], "PASS");
        assert_eq!(v["formatStatus"], "PASS");
        assert_eq!(v["success"], true);
    }

    #[tokio::test]
    async fn invalid_gherkin_fails() {
        let ws = tempfile::tempdir().unwrap();
        write_tags(ws.path());
        write(
            ws.path(),
            "spec/features/broken.feature",
            "this is not gherkin",
        );
        let out = run("{}", ws.path()).await.unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["gherkinStatus"], "FAIL");
        assert_eq!(v["success"], false);
    }
}
