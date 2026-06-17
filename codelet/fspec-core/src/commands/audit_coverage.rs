//! `audit-coverage` — Rust port of `src/commands/audit-coverage.ts` (RPC-197).
//!
//! Audits a `<feature>.feature.coverage` sidecar to verify that every test
//! file and implementation file referenced in its mappings actually exists on
//! disk. Produces an actionable report and a 0/1 exit code:
//!
//!   - exit 0 → all referenced files found (`✅ All files found (N/N)` +
//!     `All mappings valid`).
//!   - exit 1 → coverage file missing (`✗ Coverage file not found: <path>`),
//!     OR one or more referenced files missing (each rendered as
//!     `❌ Test file not found:` / `❌ Implementation file not found:` with a
//!     yellow `Recommendation:` line).
//!
//! ## Framing-A divergence (RPC-197)
//!
//! The documented `--help` output advertises a `--fix` flag and a richer
//! per-scenario report. The TS *CLI* (`src/commands/audit-coverage.ts`) does
//! NOT implement either — it only renders the file-existence report above and
//! has no `--fix` option in its Commander.js registration. We port the ACTUAL
//! TS behaviour; `--fix` lives in the help fixture only (byte-parity) and is
//! a no-op surface.
//!
//! ## Result envelope (dispatcher path)
//!
//! Returns `Ok(json)` carrying `{output, exitCode}` — exactly mirroring the TS
//! `AuditCoverageResult` interface. The dispatcher derives `success = true`
//! from the `Ok` (a missing-file audit is a successful *audit*, with
//! `exitCode == 1`). The CLI bridge prints `output` and exits with `exitCode`.
//!
//! ## Two-front-doors invariant (RPC-003 §7/§11)
//!
//! Both the LLM-facing dispatcher AND the standalone fspec binary's
//! `fspec audit-coverage` clap subcommand call this single `run` function. No
//! file-existence or rendering logic is duplicated in the CLI bridge.

use std::path::Path;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::types::coverage::CoverageFile;

/// CLI / dispatcher arguments accepted by `audit-coverage`.
///
/// Parity with the TS Commander.js registration at
/// `src/commands/audit-coverage.ts:126-136`: the only argument is the
/// positional `<feature-name>` (e.g. `"user-login"`). `--fix` is documented
/// but not implemented by the TS CLI (Framing-A divergence) so it is not
/// modelled here.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AuditCoverageArgs {
    /// Required: feature basename (e.g. `"user-login"`).
    feature_name: Option<String>,
}

/// One missing-file record discovered during the audit.
struct Missing {
    file: String,
    /// `"test"` or `"implementation"`.
    kind: MissingKind,
}

#[derive(Clone, Copy)]
enum MissingKind {
    Test,
    Implementation,
}

/// Dispatcher entry point. `project_root` is supplied by both front doors so
/// the same binary serves multiple working directories safely.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AuditCoverageArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "audit-coverage",
            reason: format!("failed to parse args: {e}"),
        })?;

    let feature_name = match args.feature_name.as_deref() {
        Some(s) if !s.trim().is_empty() => s.to_string(),
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "audit-coverage",
                reason: "missing or empty `featureName` field".to_string(),
            });
        }
    };

    let coverage_path = project_root
        .join("spec")
        .join("features")
        .join(format!("{feature_name}.feature.coverage"));

    // Missing coverage file → exit 1 with the `✗ Coverage file not found:`
    // sentinel. Mirrors `src/commands/audit-coverage.ts:37-42`.
    if !coverage_path.exists() {
        let report = format!("✗ Coverage file not found: {}", coverage_path.display());
        return ok(&report, 1);
    }

    // Read + parse the coverage sidecar. The TS implementation does not guard
    // the `JSON.parse` here — a malformed file throws and the action's
    // top-level catch surfaces it. We surface it as an InvalidArgs error so the
    // dispatcher reports failure and the CLI prints `Error:` + exit 1.
    let content = std::fs::read_to_string(&coverage_path).map_err(|e| FspecCoreError::Io {
        command: "audit-coverage",
        source: e,
    })?;
    let coverage: CoverageFile =
        serde_json::from_str(&content).map_err(|e| FspecCoreError::InvalidArgs {
            command: "audit-coverage",
            reason: format!(
                "invalid JSON in coverage file: {}",
                crate::io::json_error::parse_json_reason(&content, &e)
            ),
        })?;

    // Collect every referenced file (test + impl), recording which ones are
    // missing. `all_files` counts EVERY reference (a file referenced twice is
    // counted twice) — parity with the TS `allFiles.push` semantics where the
    // denominator is the total push count.
    let mut all_count = 0usize;
    let mut missing: Vec<Missing> = Vec::new();

    for scenario in &coverage.scenarios {
        for test_mapping in &scenario.test_mappings {
            all_count += 1;
            if !project_root.join(&test_mapping.file).exists() {
                missing.push(Missing {
                    file: test_mapping.file.clone(),
                    kind: MissingKind::Test,
                });
            }
            for impl_mapping in &test_mapping.impl_mappings {
                all_count += 1;
                if !project_root.join(&impl_mapping.file).exists() {
                    missing.push(Missing {
                        file: impl_mapping.file.clone(),
                        kind: MissingKind::Implementation,
                    });
                }
            }
        }
    }

    let report = render_report(all_count, &missing);
    let exit_code = if missing.is_empty() { 0 } else { 1 };
    ok(&report, exit_code)
}

/// Build the success or missing-file report text.
///
/// Mirrors `src/commands/audit-coverage.ts:73-117` exactly (modulo chalk ANSI,
/// which the dispatcher contract strips):
///   * all found → `✅ All files found (N/N)\nAll mappings valid`
///   * missing   → `✗ M missing file(s) out of N total files\n\n` followed by
///     one `❌ … not found:` + `   Recommendation:` block per missing file,
///     each terminated by a blank line.
fn render_report(all_count: usize, missing: &[Missing]) -> String {
    if missing.is_empty() {
        return format!("✅ All files found ({all_count}/{all_count})\nAll mappings valid");
    }

    let mut report = format!(
        "✗ {} missing file(s) out of {all_count} total files\n\n",
        missing.len()
    );

    for m in missing {
        match m.kind {
            MissingKind::Test => {
                report.push_str(&format!("❌ Test file not found: {}\n", m.file));
            }
            MissingKind::Implementation => {
                report.push_str(&format!("❌ Implementation file not found: {}\n", m.file));
            }
        }
        report.push_str("   Recommendation: Remove this mapping or restore the deleted file\n\n");
    }

    report
}

/// Serialize the `{output, exitCode}` envelope.
fn ok(output: &str, exit_code: u8) -> Result<String, FspecCoreError> {
    let payload = json!({ "output": output, "exitCode": exit_code });
    serde_json::to_string(&payload).map_err(|e| FspecCoreError::InvalidArgs {
        command: "audit-coverage",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn render_all_found() {
        let out = render_report(3, &[]);
        assert_eq!(out, "✅ All files found (3/3)\nAll mappings valid");
    }

    #[test]
    fn render_missing_test() {
        let out = render_report(
            1,
            &[Missing {
                file: "src/__tests__/deleted.test.ts".into(),
                kind: MissingKind::Test,
            }],
        );
        assert!(out.contains("✗ 1 missing file(s) out of 1 total files"));
        assert!(out.contains("❌ Test file not found: src/__tests__/deleted.test.ts"));
        assert!(out.contains("Recommendation: Remove this mapping or restore the deleted file"));
    }

    #[test]
    fn render_missing_impl() {
        let out = render_report(
            1,
            &[Missing {
                file: "src/gone.ts".into(),
                kind: MissingKind::Implementation,
            }],
        );
        assert!(out.contains("❌ Implementation file not found: src/gone.ts"));
    }

    #[test]
    fn args_parse_camel_case() {
        let a: AuditCoverageArgs =
            serde_json::from_str(r#"{"featureName":"user-login"}"#).unwrap();
        assert_eq!(a.feature_name.as_deref(), Some("user-login"));
    }
}
