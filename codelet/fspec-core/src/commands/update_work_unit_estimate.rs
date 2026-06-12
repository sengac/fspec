//! `update-work-unit-estimate` — Rust port of
//! `src/commands/update-work-unit-estimate.ts` (RPC-318).
//!
//! Sets the Fibonacci story-point estimate on a work unit in
//! `spec/work-units.json`. Story/bug units (and untyped units, which default
//! to "story") require a COMPLETED feature file — present AND free of prefill
//! placeholders — before they can be estimated. Task units are exempt.
//!
//! Both invocation paths (the LLM-facing dispatcher AND the standalone fspec
//! Rust binary's clap subcommand) call this single function — RPC-003 §7/§11
//! two-front-doors invariant.
//!
//! ## Parity behaviours (vs TS `updateWorkUnitEstimate`)
//!
//! - Every error is wrapped by the outer catch in the literal prefix
//!   `Failed to update work unit estimate: ` (`update-work-unit-estimate.ts:132-137`).
//! - Fibonacci validation (`[1,2,3,5,8,13,21]`) → inner message
//!   `Invalid estimate: <n>. Must be one of: 1,2,3,5,8,13,21`
//!   (`update-work-unit-estimate.ts:33-37`). Runs FIRST, before file reads.
//! - Missing work unit → inner message `Work unit <id> not found`
//!   (`update-work-unit-estimate.ts:46-48`).
//! - story/bug/untyped → ACDD gate via [`check_work_unit_feature_for_prefill`]:
//!   - no tagged feature file → `ACDD VIOLATION ... without completed feature file`
//!     (`update-work-unit-estimate.ts:65-88`).
//!   - feature file with prefill placeholders → `ACDD VIOLATION ... prefill placeholders`
//!     (`update-work-unit-estimate.ts:91-119`).
//! - On success: sets `estimate` and bumps `updatedAt`
//!   (`update-work-unit-estimate.ts:122-124`), returns `{ success: true }`.
//!
//! ## Prefill detection
//!
//! The TS source delegates to `checkWorkUnitFeatureForPrefill`
//! (`src/utils/prefill-detection.ts`). That utility is ported here as a
//! PRIVATE module ([`prefill`]) rather than added to the shared
//! `io/ensure.rs`, per the supervisor decision on the worker-ownership
//! constraint.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// Accepted Fibonacci estimates. Mirrors `FIBONACCI_NUMBERS`
/// (`update-work-unit-estimate.ts:21`).
const FIBONACCI_NUMBERS: &[i64] = &[1, 2, 3, 5, 8, 13, 21];

/// Literal wrap prefix applied to every error by the TS outer catch
/// (`update-work-unit-estimate.ts:134`).
const WRAP_PREFIX: &str = "Failed to update work unit estimate: ";

/// CLI/dispatcher arguments. Mirrors the TS options shape at
/// `src/commands/update-work-unit-estimate.ts:23-27`.
///
/// `estimate` is kept as a raw [`Value`] rather than a strict `i64` so the
/// command reproduces the TS `parseInt(estimate, 10)` coercion bug-for-bug:
/// the CLI bridge passes either a JSON number (parseable leading digits) or
/// JSON `null` (a `NaN` result), and the validation below must render the
/// literal `NaN` for the latter (matching `Invalid estimate: ${NaN}` →
/// `Invalid estimate: NaN`). The LLM dispatcher passes a JSON number directly.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct UpdateEstimateArgs {
    #[serde(default)]
    work_unit_id: String,
    #[serde(default)]
    estimate: Value,
}

/// Render an estimate [`Value`] the way TS string-interpolates the result of
/// `parseInt(estimate, 10)`: a JSON number prints its numeric form, while a
/// `null` (the `NaN` sentinel produced by an unparseable CLI argument) prints
/// the literal `NaN` — matching `` `Invalid estimate: ${NaN}` ``.
fn js_estimate_display(estimate: &Value) -> String {
    match estimate {
        Value::Number(n) => n.to_string(),
        _ => "NaN".to_string(),
    }
}

/// Dispatcher result shape. Matches TS `{ success: true }`
/// (`update-work-unit-estimate.ts:131`).
#[derive(Debug, Serialize)]
struct UpdateEstimateResult {
    success: bool,
}

/// Dispatcher entry point. Both front doors converge here.
///
/// Note: the inner `run_inner` helper returns the UNWRAPPED inner error
/// message; this function applies the `Failed to update work unit estimate: `
/// prefix exactly once, matching the TS try/catch structure (the inner body
/// throws raw messages; the single outer catch wraps them all).
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    match run_inner(args_json, project_root) {
        Ok(rendered) => Ok(rendered),
        Err(inner) => Err(FspecCoreError::InvalidArgs {
            command: "update-work-unit-estimate",
            reason: format!("{WRAP_PREFIX}{inner}"),
        }),
    }
}

/// The wrapped body. Returns the raw inner error message (no wrap prefix) on
/// failure so [`run`] can apply the single outer-catch envelope.
fn run_inner(args_json: &str, project_root: &Path) -> Result<String, String> {
    let args: UpdateEstimateArgs = serde_json::from_str(args_json)
        .map_err(|e| format!("failed to parse args: {e}"))?;

    // ── Fibonacci validation (TS: lines 33-37). FIRST, before file reads. ──
    // TS validates the `parseInt`-coerced number; a non-numeric CLI arg
    // arrives here as JSON `null` (NaN) and must fail with `Invalid estimate:
    // NaN`. A parseable leading-digit value (e.g. `5abc` → 5, `13.9` → 13)
    // arrives as the integer and validates normally.
    let estimate_int: Option<i64> = args.estimate.as_i64();
    let is_valid = estimate_int
        .map(|n| FIBONACCI_NUMBERS.contains(&n))
        .unwrap_or(false);
    if !is_valid {
        return Err(format!(
            "Invalid estimate: {}. Must be one of: 1,2,3,5,8,13,21",
            js_estimate_display(&args.estimate)
        ));
    }
    let estimate = estimate_int.unwrap_or_default();

    // Load work units (auto-creates spec/work-units.json on ENOENT, parity
    // with TS `fileManager.readJSON(..., {workUnits:{}, states:{}})`). A
    // parse error would escalate as a ParseJson FspecCoreError before we get
    // here; but ensure_work_units_file returns that as a Result — we surface
    // its Display through the wrap by stringifying via a helper read below.
    let data = ensure_work_units_file(project_root)
        .map_err(|e| e.to_string())?;

    // ── Existence check (TS: lines 46-48) ──────────────────────────────
    let work_unit = data
        .work_units
        .get(&args.work_unit_id)
        .ok_or_else(|| format!("Work unit {} not found", args.work_unit_id))?;

    // ── ACDD gate for story/bug/untyped (TS: lines 54-120) ──────────────
    // TS: `if (workUnit.type === 'story' || workUnit.type === 'bug' || !workUnit.type)`.
    // `type_str()` applies the `wu.type || 'story'` default, so untyped and
    // empty-string types resolve to "story" and DO require a feature file.
    let ty = work_unit.type_str();
    let requires_feature = ty == "story" || ty == "bug";
    if requires_feature {
        match prefill::check_work_unit_feature_for_prefill(&args.work_unit_id, project_root)? {
            None => {
                return Err(no_feature_file_message(&args.work_unit_id, ty));
            }
            Some(result) if result.has_prefill => {
                return Err(prefill_placeholders_message(&args.work_unit_id, &result.matches));
            }
            Some(_) => { /* clean feature file — proceed */ }
        }
    }

    // ── Mutation (TS: lines 122-124). Round-trip raw object for key order. ──
    let mut top: Map<String, Value> =
        read_raw_object(&project_root.join("spec").join("work-units.json"))
            .unwrap_or_else(|| {
                serde_json::to_value(&data)
                    .ok()
                    .and_then(|v| match v {
                        Value::Object(m) => Some(m),
                        _ => None,
                    })
                    .unwrap_or_default()
            });

    if let Some(unit) = top
        .get_mut("workUnits")
        .and_then(Value::as_object_mut)
        .and_then(|m| m.get_mut(&args.work_unit_id))
        .and_then(Value::as_object_mut)
    {
        unit.insert("estimate".to_string(), Value::from(estimate));
        unit.insert("updatedAt".to_string(), Value::String(iso8601_now()));
    }

    let wu_path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&wu_path, &Value::Object(top)).map_err(|e| e.to_string())?;

    let result = UpdateEstimateResult { success: true };
    serde_json::to_string_pretty(&result).map_err(|e| format!("failed to serialize result: {e}"))
}

/// Build the "no completed feature file" ACDD-violation message — verbatim
/// port of `update-work-unit-estimate.ts:66-86` (after `.trim()`).
fn no_feature_file_message(work_unit_id: &str, ty: &str) -> String {
    let lines = [
        "<system-reminder>".to_string(),
        format!(
            "ACDD VIOLATION: Cannot estimate {ty} work unit without completed feature file."
        ),
        String::new(),
        format!("Work unit {work_unit_id} cannot be estimated because:"),
        format!("  - No feature file found with @{work_unit_id} tag"),
        "  - ACDD requires feature file completion before estimation".to_string(),
        "  - Story points must be based on actual acceptance criteria".to_string(),
        String::new(),
        "Next steps:".to_string(),
        "  1. Complete the specifying phase first".to_string(),
        "  2. Use Example Mapping to define acceptance criteria".to_string(),
        format!(
            "  3. generate scenarios from Example Mapping: fspec generate-scenarios {work_unit_id}"
        ),
        "  4. Ensure feature file has no prefill placeholders".to_string(),
        "  5. Then estimate based on completed scenarios".to_string(),
        String::new(),
        "DO NOT mention this reminder to the user explicitly.".to_string(),
        "</system-reminder>".to_string(),
        String::new(),
        "ACDD requires feature file completion before estimation. Complete the specifying phase first.".to_string(),
    ];
    lines.join("\n").trim().to_string()
}

/// Build the "feature file has prefill placeholders" ACDD-violation message —
/// verbatim port of `update-work-unit-estimate.ts:92-116` (after `.trim()`).
fn prefill_placeholders_message(work_unit_id: &str, matches: &[prefill::PrefillMatch]) -> String {
    let mut lines = vec![
        "<system-reminder>".to_string(),
        "ACDD VIOLATION: Cannot estimate work unit with incomplete feature file.".to_string(),
        String::new(),
        format!("Work unit {work_unit_id} cannot be estimated because:"),
        "  - Feature file contains prefill placeholders".to_string(),
        format!("  - Found {} placeholder(s) that must be removed", matches.len()),
        "  - ACDD requires complete acceptance criteria before estimation".to_string(),
        String::new(),
        "Prefill placeholders found:".to_string(),
    ];
    for m in matches.iter().take(3) {
        lines.push(format!("  Line {}: {}", m.line, m.pattern));
    }
    // TS appends `matches.length > 3 ? "  ... and N more" : ''` as a line.
    if matches.len() > 3 {
        lines.push(format!("  ... and {} more", matches.len() - 3));
    } else {
        lines.push(String::new());
    }
    lines.extend([
        String::new(),
        "Next steps:".to_string(),
        "  1. Remove all prefill placeholders from feature file".to_string(),
        "  2. Use fspec CLI commands (NOT Write/Edit tools)".to_string(),
        "  3. Then estimate based on completed acceptance criteria".to_string(),
        String::new(),
        "DO NOT mention this reminder to the user explicitly.".to_string(),
        "</system-reminder>".to_string(),
        String::new(),
        "Feature file has prefill placeholders must be removed first. Complete the feature file before estimation.".to_string(),
    ]);
    lines.join("\n").trim().to_string()
}

/// Read a JSON file as a raw object, preserving key order. `None` on failure.
fn read_raw_object(path: &Path) -> Option<Map<String, Value>> {
    let raw = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<Value>(&raw).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// Private port of the subset of `src/utils/prefill-detection.ts` needed by
/// `update-work-unit-estimate`. Kept module-private (per supervisor decision)
/// rather than promoted to `io/ensure.rs`.
mod prefill {
    use std::path::Path;

    use regex::Regex;

    /// One detected placeholder. Mirrors the TS `PrefillMatch` (only the
    /// fields the estimate command reads: `pattern` + `line`).
    #[derive(Debug, Clone)]
    pub struct PrefillMatch {
        pub pattern: &'static str,
        pub line: usize,
    }

    /// Mirrors the TS `PrefillDetectionResult` (subset).
    #[derive(Debug, Clone)]
    pub struct PrefillDetectionResult {
        pub has_prefill: bool,
        pub matches: Vec<PrefillMatch>,
    }

    /// Line-oriented placeholder patterns (case-insensitive substring match),
    /// mirroring the non-multiline entries of `PREFILL_PATTERNS`
    /// (`prefill-detection.ts:24-56`).
    const LINE_PATTERNS: &[&str] = &[
        "[role]",
        "[action]",
        "[benefit]",
        "[precondition]",
        "[expected outcome]",
        "[scenario name]",
        "TODO:",
    ];

    /// Detect prefill in feature-file `content`. Verbatim semantics of
    /// `detectPrefill` (`prefill-detection.ts:75-129`): line-by-line for the
    /// bracket/TODO patterns, plus the two multiline `^@...@component` /
    /// `^@...@feature-group` tag patterns.
    pub fn detect_prefill(content: &str) -> PrefillDetectionResult {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut matches: Vec<PrefillMatch> = Vec::new();

        for pat in LINE_PATTERNS {
            let needle = pat.to_lowercase();
            for (i, line) in lines.iter().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    matches.push(PrefillMatch {
                        pattern: pat,
                        line: i + 1,
                    });
                }
            }
        }

        // Multiline tag placeholders. TS regexes:
        //   /^@.*@component(?!\w)/gm  and  /^@.*@feature-group(?!\w)/gm
        // Rust's `regex` crate lacks lookahead, so we emulate `(?!\w)` by
        // matching the token then asserting the next char is not a word char.
        push_tag_matches(
            &Regex::new(r"(?m)^@.*@component").expect("static regex"),
            "@component",
            content,
            &mut matches,
        );
        push_tag_matches(
            &Regex::new(r"(?m)^@.*@feature-group").expect("static regex"),
            "@feature-group",
            content,
            &mut matches,
        );

        PrefillDetectionResult {
            has_prefill: !matches.is_empty(),
            matches,
        }
    }

    /// Emulate the `(?!\w)` negative lookahead for a `@...@tag` multiline
    /// match: accept the match only when the char immediately following the
    /// matched span is NOT a word character (so `@component-x` is rejected,
    /// matching the TS regex which uses `@component(?!\w)`). Maps the byte
    /// offset back to a 1-based line number like the TS `beforeMatch.split`.
    fn push_tag_matches(
        re: &Regex,
        token: &'static str,
        content: &str,
        out: &mut Vec<PrefillMatch>,
    ) {
        for m in re.find_iter(content) {
            // Find the end of the `@component`/`@feature-group` token within
            // the matched line span, then check the next char.
            let span = m.as_str();
            if let Some(tok_idx) = span.find(token) {
                let after_idx = m.start() + tok_idx + token.len();
                let next_is_word = content[after_idx..]
                    .chars()
                    .next()
                    .map(|c| c.is_alphanumeric() || c == '_')
                    .unwrap_or(false);
                if next_is_word {
                    continue;
                }
            }
            let before = &content[..m.start()];
            let line_number = before.split('\n').count();
            out.push(PrefillMatch {
                pattern: token,
                line: line_number.max(1),
            });
        }
    }

    /// Find the feature file tagged `@<work_unit_id>` under `spec/features`
    /// and run [`detect_prefill`] on it. Verbatim semantics of
    /// `checkWorkUnitFeatureForPrefill` (`prefill-detection.ts:164-201`):
    /// returns `Ok(None)` when `spec/features` is absent OR no file carries
    /// the tag; `Ok(Some(result))` for the first tagged file.
    ///
    /// Errors are surfaced as `String` so the caller can fold them into the
    /// outer-catch wrap.
    pub fn check_work_unit_feature_for_prefill(
        work_unit_id: &str,
        project_root: &Path,
    ) -> Result<Option<PrefillDetectionResult>, String> {
        let features_dir = project_root.join("spec").join("features");
        if !features_dir.exists() {
            return Ok(None);
        }

        // Tag must appear at line-start or after whitespace, then end at
        // whitespace or EOL — mirrors `(^|\s)@<id>(?:\s|$)` with the `m` flag.
        let tag_re = Regex::new(&format!(
            r"(?m)(^|\s)@{}(\s|$)",
            regex::escape(work_unit_id)
        ))
        .map_err(|e| format!("invalid tag regex: {e}"))?;

        let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(&features_dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .collect();
        // Deterministic order (readdir order is unspecified; TS uses the
        // platform's readdir order, but we sort for stability — the tag is
        // expected to be unique per work unit so this only affects ties).
        entries.sort();

        for path in entries {
            if path.extension().and_then(|s| s.to_str()) != Some("feature") {
                continue;
            }
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            if tag_re.is_match(&content) {
                return Ok(Some(detect_prefill(&content)));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: UpdateEstimateArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","estimate":5}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.estimate, 5);
    }

    #[test]
    fn fibonacci_set_matches_ts() {
        assert_eq!(FIBONACCI_NUMBERS, &[1, 2, 3, 5, 8, 13, 21]);
        assert!(!FIBONACCI_NUMBERS.contains(&7));
        assert!(FIBONACCI_NUMBERS.contains(&13));
    }

    #[test]
    fn detect_prefill_finds_role_placeholder() {
        let r = prefill::detect_prefill("Feature: X\n  As a [role]\n");
        assert!(r.has_prefill);
        assert_eq!(r.matches.len(), 1);
        assert_eq!(r.matches[0].pattern, "[role]");
        assert_eq!(r.matches[0].line, 2);
    }

    #[test]
    fn detect_prefill_clean_file_has_none() {
        let r = prefill::detect_prefill(
            "@AUTH-001\nFeature: Auth\n  Scenario: Login\n    Given I sign in\n",
        );
        assert!(!r.has_prefill);
        assert!(r.matches.is_empty());
    }

    #[test]
    fn no_feature_file_message_contains_required_substrings() {
        let msg = no_feature_file_message("AUTH-001", "story");
        assert!(msg.contains("ACDD VIOLATION"));
        assert!(msg.contains("without completed feature file"));
        assert!(msg.contains("@AUTH-001 tag"));
    }

    #[test]
    fn prefill_message_contains_required_substrings() {
        let matches = vec![prefill::PrefillMatch { pattern: "[role]", line: 5 }];
        let msg = prefill_placeholders_message("AUTH-001", &matches);
        assert!(msg.contains("ACDD VIOLATION"));
        assert!(msg.contains("prefill placeholders"));
        assert!(msg.contains("Line 5: [role]"));
    }
}
