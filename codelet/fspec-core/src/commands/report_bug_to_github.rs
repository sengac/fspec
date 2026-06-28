//! `report-bug-to-github` — Rust port of `src/commands/report-bug-to-github.ts`
//! (RPC-285, DETERMINISTIC-CORE scope).
//!
//! Feature: spec/features/report-bug-to-github-rust-port.feature
//!
//! Gathers system + git + work-unit context, formats a Markdown bug report,
//! and constructs a pre-filled GitHub issue URL for `sengac/fspec`. Both
//! invocation paths (the LLM-facing dispatcher AND the standalone fspec Rust
//! binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## Scope (supervisor ruling, RPC-285)
//!
//! DETERMINISTIC-CORE only. We:
//!   * gather environment (pinned fspec version + OS/arch via
//!     [`std::env::consts`]),
//!   * gather git context via BLOCKING [`std::process::Command::output`]
//!     (NO in-command network),
//!   * gather work-unit context (faithfully replicating the TS
//!     `ensureWorkUnitsFile` side-effect — `spec/work-units.json` is created
//!     when missing),
//!   * format the issue body + title and build the GitHub issue URL.
//!
//! The interactive stdin prompts and the browser launch are DEFERRED (same
//! class as `research` EXECUTE): every path returns `browserOpened = false`
//! and the constructed URL. No `.await` on a tokio resource — the function
//! resolves on the first poll under [`crate::dispatch::poll_sync_future`].

use std::path::Path;
use std::process::Command;

use serde::Deserialize;
use serde_json::json;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;

/// fspec version reported in the Environment section. Pinned to match
/// `commands::init::FSPEC_VERSION` (the TS `getVersion()` reads package.json,
/// pinned to 0.9.3 across the Rust port).
const FSPEC_VERSION: &str = "0.9.3";

/// CLI arguments accepted by `report-bug-to-github`. Mirrors the TS
/// Commander.js registration at `src/commands/report-bug-to-github.ts:364-374`.
/// `projectRoot` is accepted for surface parity but ignored — the dispatcher
/// always passes the canonical `project_root` so the same binary can serve
/// multiple working directories safely.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ReportBugArgs {
    project_root: Option<String>,
    bug_description: Option<String>,
    expected_behavior: Option<String>,
    actual_behavior: Option<String>,
    /// Accepted for surface parity; the interactive prompt path is DEFERRED.
    interactive: bool,
}

/// In-memory bug-report context (mirrors the TS `BugReportContext`).
struct BugReportContext {
    fspec_version: String,
    arch: String,
    platform: String,
    current_branch: Option<String>,
    has_uncommitted_changes: bool,
    work_unit_id: Option<String>,
    work_unit_title: Option<String>,
    work_unit_status: Option<String>,
    feature_file: Option<String>,
}

/// Dispatcher entry point. Returns a JSON envelope:
/// `{ title, markdown, url, browserOpened, context }`.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ReportBugArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "report-bug-to-github",
            reason: format!("failed to parse args: {e}"),
        })?;

    let context = gather_context(project_root)?;

    let bug_description = args
        .bug_description
        .unwrap_or_else(|| "Bug description".to_string());
    let expected_behavior = args
        .expected_behavior
        .unwrap_or_else(|| "Expected behavior".to_string());
    let actual_behavior = args
        .actual_behavior
        .unwrap_or_else(|| "Actual behavior".to_string());
    let steps_to_reproduce = ["Run fspec command", "Observe error"];

    let markdown = format_bug_report_markdown(
        &context,
        &bug_description,
        &expected_behavior,
        &actual_behavior,
        &steps_to_reproduce,
    );

    // Title: `Bug: ${bugDescription.substring(0, 60)}` (TS line 315).
    let title = format!("Bug: {}", truncate_chars(&bug_description, 60));

    let url = construct_github_url(&title, &markdown);

    // Browser launch + interactive editing/confirmation are DEFERRED.
    let browser_opened = false;

    let envelope = json!({
        "title": title,
        "markdown": markdown,
        "url": url,
        "browserOpened": browser_opened,
        "previewShown": false,
        "cancelled": false,
        "context": {
            "fspecVersion": context.fspec_version,
            "arch": context.arch,
            "platform": context.platform,
            "currentBranch": context.current_branch,
            "hasUncommittedChanges": context.has_uncommitted_changes,
            "workUnitId": context.work_unit_id,
            "workUnitTitle": context.work_unit_title,
            "workUnitStatus": context.work_unit_status,
            "featureFile": context.feature_file,
        },
    });

    serde_json::to_string(&envelope).map_err(|e| FspecCoreError::InvalidArgs {
        command: "report-bug-to-github",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Gathers system + git + work-unit context. Mirrors `gatherContext`
/// (`src/commands/report-bug-to-github.ts:67-172`) within the deterministic
/// scope (no network, no error-log enrichment beyond best-effort read).
fn gather_context(project_root: &Path) -> Result<BugReportContext, FspecCoreError> {
    // fspec version — pinned (TS reads package.json).
    let fspec_version = FSPEC_VERSION.to_string();

    // Node has no Rust analogue; per the RPC-285 scope ruling the Environment
    // block reports the build architecture from std::env::consts::ARCH in place
    // of the TS `process.version` line.
    let arch = std::env::consts::ARCH.to_string();

    // Platform — TS uses process.platform; we use std::env::consts::OS so the
    // Environment-line assertion is behaviour-level (present + well-formed).
    let platform = std::env::consts::OS.to_string();

    // Git context — best-effort via BLOCKING git subprocess; NO network.
    let (current_branch, has_uncommitted_changes) = gather_git_context(project_root);

    // Work-unit context — faithfully replicate the TS `ensureWorkUnitsFile`
    // side-effect (creates spec/work-units.json when missing).
    let mut work_unit_id = None;
    let mut work_unit_title = None;
    let mut work_unit_status = None;
    let mut feature_file = None;

    if let Ok(work_units_data) = ensure_work_units_file(project_root) {
        // Most recently updated non-done work unit (TS sorts by updatedAt desc).
        let mut candidates: Vec<&crate::types::work_unit::WorkUnit> = work_units_data
            .work_units
            .values()
            .filter(|wu| wu.status.as_str() != "done")
            .collect();
        candidates.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        if let Some(most_recent) = candidates.first() {
            let id = most_recent.id.clone();
            work_unit_title = Some(most_recent.title.clone());
            work_unit_status = Some(most_recent.status.as_str().to_string());
            feature_file = find_feature_file(project_root, &id);
            work_unit_id = Some(id);
        }
    }

    Ok(BugReportContext {
        fspec_version,
        arch,
        platform,
        current_branch,
        has_uncommitted_changes,
        work_unit_id,
        work_unit_title,
        work_unit_status,
        feature_file,
    })
}

/// Best-effort git context: `(current_branch, has_uncommitted_changes)`.
/// Any failure (not a git repo, git missing) → `(None, false)`, matching the
/// TS bare `catch {}` around `getCurrentBranch` / `getGitStatus`.
fn gather_git_context(project_root: &Path) -> (Option<String>, bool) {
    let branch = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty());

    // If we couldn't determine a branch, treat git as unavailable entirely
    // (parity with the TS try-block that aborts on the first throw).
    if branch.is_none() {
        return (None, false);
    }

    let has_uncommitted = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);

    (branch, has_uncommitted)
}

/// Find the first `spec/features/*.feature` whose content references
/// `@{work_unit_id}`. Returns `"spec/features/<file>"`. Mirrors TS lines
/// 120-135 (bare `catch {}` on a missing features directory → `None`).
fn find_feature_file(project_root: &Path, work_unit_id: &str) -> Option<String> {
    let features_dir = project_root.join("spec").join("features");
    let needle = format!("@{work_unit_id}");

    let mut entries: Vec<String> = std::fs::read_dir(&features_dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| name.ends_with(".feature"))
        .collect();
    // Deterministic order (read_dir order is filesystem-dependent).
    entries.sort();

    for file in entries {
        let path = features_dir.join(&file);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if content.contains(&needle) {
                return Some(format!("spec/features/{file}"));
            }
        }
    }
    None
}

/// Format the bug report as Markdown. Byte-faithful port of
/// `formatBugReportMarkdown` (`src/commands/report-bug-to-github.ts:177-243`).
fn format_bug_report_markdown(
    context: &BugReportContext,
    description: &str,
    expected_behavior: &str,
    actual_behavior: &str,
    steps_to_reproduce: &[&str],
) -> String {
    let mut md = String::new();

    md.push_str("## Description\n\n");
    md.push_str(&format!("{description}\n\n"));

    md.push_str("## Expected Behavior\n\n");
    md.push_str(&format!("{expected_behavior}\n\n"));

    md.push_str("## Actual Behavior\n\n");
    md.push_str(&format!("{actual_behavior}\n\n"));

    md.push_str("## Steps to Reproduce\n\n");
    for (index, step) in steps_to_reproduce.iter().enumerate() {
        md.push_str(&format!("{}. {step}\n", index + 1));
    }
    md.push('\n');

    md.push_str("## Environment\n\n");
    md.push_str(&format!("- fspec version: {}\n", context.fspec_version));
    md.push_str(&format!("- OS: {}\n", context.platform));
    md.push_str(&format!("- Arch: {}\n", context.arch));
    if let Some(branch) = &context.current_branch {
        md.push_str(&format!("- Git branch: {branch}\n"));
    }
    md.push('\n');

    md.push_str("## Additional Context\n\n");

    if let Some(id) = &context.work_unit_id {
        let title = context.work_unit_title.as_deref().unwrap_or("");
        md.push_str(&format!("**Work Unit**: {id} - {title}\n"));
        let status = context.work_unit_status.as_deref().unwrap_or("");
        md.push_str(&format!("**Status**: {status}\n"));
        if let Some(feature) = &context.feature_file {
            md.push_str(&format!("**Feature File**: {feature}\n"));
        }
        md.push('\n');
    }

    if context.has_uncommitted_changes {
        md.push_str("**Note**: There are uncommitted changes in the working directory.\n");
        md.push_str("If relevant, please provide git diff output.\n\n");
    }

    md
}

/// Constructs the GitHub issue URL. Port of `constructGitHubURL`
/// (`src/commands/report-bug-to-github.ts:248-258`).
fn construct_github_url(title: &str, body: &str) -> String {
    let owner = "sengac";
    let repo = "fspec";
    let labels = "bug,needs-triage";

    let encoded_title = encode_uri_component(title);
    let encoded_body = encode_uri_component(body);
    let encoded_labels = encode_uri_component(labels);

    format!(
        "https://github.com/{owner}/{repo}/issues/new?title={encoded_title}&body={encoded_body}&labels={encoded_labels}"
    )
}

/// Percent-encode a string exactly like JavaScript's `encodeURIComponent`:
/// the unreserved set `A-Za-z0-9` plus `- _ . ! ~ * ' ( )` is passed through
/// verbatim; every other byte of the UTF-8 encoding becomes `%XX` (uppercase
/// hex).
fn encode_uri_component(input: &str) -> String {
    const UNRESERVED_PUNCT: &[u8] = b"-_.!~*'()";
    let mut out = String::with_capacity(input.len() * 3);
    for &byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || UNRESERVED_PUNCT.contains(&byte) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push_str(&format!("{byte:02X}"));
        }
    }
    out
}

/// JS `String.prototype.substring(0, n)` for ASCII-safe truncation. Truncates
/// to at most `n` characters (Unicode scalar values).
fn truncate_chars(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn encode_uri_component_passes_unreserved() {
        assert_eq!(
            encode_uri_component("abcABC123-_.!~*'()"),
            "abcABC123-_.!~*'()"
        );
    }

    #[test]
    fn encode_uri_component_encodes_space_hash_comma() {
        assert_eq!(encode_uri_component("a b#c,d"), "a%20b%23c%2Cd");
    }

    #[test]
    fn construct_url_targets_sengac_fspec_with_encoded_labels() {
        let url = construct_github_url("Bug: x", "body");
        assert!(url.starts_with("https://github.com/sengac/fspec/issues/new?title="));
        assert!(url.contains("labels=bug%2Cneeds-triage"));
    }

    #[test]
    fn markdown_contains_all_sections() {
        let ctx = BugReportContext {
            fspec_version: "0.9.3".into(),
            arch: "aarch64".into(),
            platform: "linux".into(),
            current_branch: None,
            has_uncommitted_changes: false,
            work_unit_id: None,
            work_unit_title: None,
            work_unit_status: None,
            feature_file: None,
        };
        let md = format_bug_report_markdown(&ctx, "d", "e", "a", &["s1", "s2"]);
        for section in [
            "## Description",
            "## Expected Behavior",
            "## Actual Behavior",
            "## Steps to Reproduce",
            "## Environment",
            "## Additional Context",
        ] {
            assert!(md.contains(section), "missing {section}: {md}");
        }
        assert!(md.contains("- fspec version: 0.9.3"));
    }

    #[test]
    fn truncate_chars_caps_at_n() {
        assert_eq!(truncate_chars("abcdef", 3), "abc");
        assert_eq!(truncate_chars("abc", 60), "abc");
    }
}
