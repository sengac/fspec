//! `generate-scenarios` — Rust port of `src/commands/generate-scenarios.ts`
//! (RPC-234).
//!
//! Builds a CONTEXT-ONLY feature file (Example Mapping comments + Background,
//! ZERO scenarios) from a work unit's example-mapping data. The TS source
//! NEVER auto-creates scenarios — `scenariosCount` is always 0 — so this port
//! renders the same "context scaffold" and the same system-reminders.
//!
//! ## Two front doors
//!
//! Both the LLM dispatcher and the standalone fspec binary's clap subcommand
//! converge on this single [`run`] function (RPC-003 §7/§11). The CLI bridge
//! (`codelet/fspec/src/generate_scenarios.rs`) performs ZERO domain logic.
//!
//! ## Parity behaviours (vs TS `generateScenarios`)
//!
//! - Missing work unit → `Work unit '<id>' does not exist`
//!   (`generate-scenarios.ts:302-304`).
//! - Unanswered questions → `Cannot generate scenarios: N unanswered
//!   question(s) found.` + reminder (`:316-328`).
//! - Empty Example Mapping → `Cannot generate scenarios: No Example Mapping
//!   data found.` + reminder (`:334-345`).
//! - No active examples → `Work unit <id> has no examples to generate
//!   scenarios from` (`:350-354`).
//! - Duplicate scenario above threshold (without `--ignore-possible-
//!   duplicates`) → `Cannot generate scenarios: N duplicate scenarios detected
//!   above threshold.` + DUPLICATE SCENARIOS DETECTED reminder (`:384-413`).
//! - Target file already exists → `Feature file <path> already exists.` + the
//!   context-only explanation (`:467-473`).
//! - On success: writes the context-only file and returns a rendered human
//!   string (the CLI creation lines + consolidated system-reminders), mirroring
//!   the `reverse` port's full-string convention.
//!
//! ## Soft-failure surfacing
//!
//! Every gate failure surfaces as [`FspecCoreError::Message`] carrying the
//! fully-rendered body so the dispatcher reports `success=false` and the CLI
//! prints the body to stderr with the `✗ Failed to generate scenarios:` prefix.
//!
//! ## Ported support modules (private, in-file)
//!
//! Per the worker-ownership constraint, the similarity algorithms
//! (`src/utils/similarity-algorithms.ts` + `scenario-similarity.ts`), step
//! extraction (`step-extraction.ts`), prefill detection (`prefill-detection.ts`)
//! and the four verbatim system-reminder strings are ported as PRIVATE modules
//! in this file rather than added to shared `io/` helpers.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::gherkin::parse_feature_lenient;

/// CLI/dispatcher arguments. Mirrors the TS `GenerateScenariosOptions`
/// (`generate-scenarios.ts:33-40`). `cwd`/`confirmUpdate`/`template` are not
/// modelled — `cwd` is the `project_root` parameter and the other two are
/// unused by the context-only path.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct GenerateScenariosArgs {
    work_unit_id: String,
    feature: Option<String>,
    ignore_possible_duplicates: bool,
}

/// A detected near-duplicate scenario match (TS `ScenarioMatch`,
/// `generate-scenarios.ts:26-31`).
#[derive(Debug, Clone)]
struct ScenarioMatch {
    /// Feature file name (basename, e.g. `existing.feature`).
    feature: String,
    /// Matched scenario name.
    scenario: String,
    /// Hybrid similarity score (0-1).
    similarity_score: f64,
}

/// Dispatcher/CLI entry point. Both front doors converge here.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: GenerateScenariosArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "generate-scenarios",
            reason: format!("failed to parse args: {e}"),
        })?;
    run_inner(&args, project_root)
}

/// Wrapped body. Returns [`FspecCoreError::Message`] for every soft gate
/// failure (rendered body) and surfaces hard errors via other variants.
fn run_inner(
    args: &GenerateScenariosArgs,
    project_root: &Path,
) -> Result<String, FspecCoreError> {
    // Read work units (auto-creates file if missing), parity with TS
    // `ensureWorkUnitsFile`.
    let data = ensure_work_units_file(project_root)?;

    // ── Validate work unit exists (TS :302-304) ────────────────────────
    let work_unit = data.work_units.get(&args.work_unit_id).ok_or_else(|| {
        FspecCoreError::Message(format!(
            "Work unit '{}' does not exist",
            args.work_unit_id
        ))
    })?;

    // Example-mapping fields live in `WorkUnit.extra` as raw JSON.
    let extra = &work_unit.extra;

    // ── Check for unanswered questions BEFORE generation (TS :308-329) ──
    let questions = array_field(extra, "questions");
    if !questions.is_empty() {
        let unanswered = questions
            .iter()
            .filter(|q| {
                let deleted = q.get("deleted").and_then(Value::as_bool).unwrap_or(false);
                let selected = q.get("selected").and_then(Value::as_bool).unwrap_or(false);
                !deleted && !selected
            })
            .count();
        if unanswered > 0 {
            // TS only throws when the reminder is non-null (i.e. reminders
            // enabled); when disabled the gate is skipped entirely.
            if let Some(reminder) = reminders::unanswered_questions(&args.work_unit_id, unanswered) {
                let plural = if unanswered > 1 { "s" } else { "" };
                return Err(FspecCoreError::Message(format!(
                    "Cannot generate scenarios: {unanswered} unanswered question{plural} found.\n\n{reminder}\n\nAnswer questions with 'fspec answer-question {} <index>' before generating.",
                    args.work_unit_id
                )));
            }
        }
    }

    // ── Check for empty Example Mapping (TS :331-345) ───────────────────
    let rules = array_field(extra, "rules");
    let examples = array_field(extra, "examples");
    let has_rules = !rules.is_empty();
    let has_examples = !examples.is_empty();
    if !has_rules && !has_examples {
        // TS only throws when the reminder is non-null (reminders enabled).
        if let Some(reminder) =
            reminders::empty_example_mapping(&args.work_unit_id, has_rules, has_examples)
        {
            return Err(FspecCoreError::Message(format!(
                "Cannot generate scenarios: No Example Mapping data found.\n\n{reminder}\n\nComplete Example Mapping before generating scenarios."
            )));
        }
    }

    // ── Validate active examples exist (TS :347-354) ────────────────────
    let active_examples: Vec<&Value> = examples
        .iter()
        .copied()
        .filter(|e| !e.get("deleted").and_then(Value::as_bool).unwrap_or(false))
        .collect();
    if active_examples.is_empty() {
        return Err(FspecCoreError::Message(format!(
            "Work unit {} has no examples to generate scenarios from",
            args.work_unit_id
        )));
    }
    let active_example_texts: Vec<String> = active_examples
        .iter()
        .map(|e| string_field(e, "text"))
        .collect();

    // ── Scan existing features for matches (TS :356-360) ────────────────
    let detected = scan_existing_features(project_root, &active_example_texts);

    // ── Duplicate handling (TS :362-431) ────────────────────────────────
    if !detected.is_empty() && !args.ignore_possible_duplicates {
        // Build the DUPLICATE SCENARIOS DETECTED reminder (TS :369-413).
        let mut feature_files: Vec<String> = Vec::new();
        let mut match_details: Vec<String> = Vec::new();
        for (example_index, matches) in &detected {
            let best = &matches[0];
            if !feature_files.contains(&best.feature) {
                feature_files.push(best.feature.clone());
            }
            match_details.push(format!(
                "  - Example {}: \"{}\"\n    Matches: \"{}\" in {}\n    Similarity: {}%",
                example_index + 1,
                active_example_texts[*example_index],
                best.scenario,
                best.feature,
                fmt_pct(best.similarity_score)
            ));
        }
        let plural = if detected.len() > 1 { "s" } else { "" };
        let files_block = feature_files
            .iter()
            .map(|f| format!("  - spec/features/{f}"))
            .collect::<Vec<_>>()
            .join("\n");
        let system_reminder = format!(
            "<system-reminder>\nDUPLICATE SCENARIOS DETECTED\n\nFound {count} potential duplicate scenario{plural} in existing feature files.\n\nDetected matches:\n{details}\n\nFeature files to investigate:\n{files}\n\nNext steps:\n  1. Investigate the feature files listed above\n  2. Determine if the scenarios are truly duplicates\n  3. If they are duplicates:\n     - Consider refactoring to reuse existing scenarios\n     - Or update the existing feature file instead of creating a new one\n  4. If they are NOT duplicates (false positive):\n     - Run: fspec generate-scenarios {wu} --ignore-possible-duplicates\n     - This will bypass the duplicate check and proceed with generation\n\nThis check prevents accidental duplication across feature files.\nDO NOT mention this reminder to the user explicitly.\n</system-reminder>",
            count = detected.len(),
            details = match_details.join("\n\n"),
            files = files_block,
            wu = args.work_unit_id,
        );
        return Err(FspecCoreError::Message(format!(
            "Cannot generate scenarios: {} duplicate scenarios detected above threshold.\n\n{system_reminder}\n\nInvestigate feature files or use --ignore-possible-duplicates to proceed.",
            detected.len()
        )));
    }
    // ignoring duplicates → TS logs warnings to stdout; ACCEPTED divergence
    // (DAG D2): we fold them into the returned String instead. Handled at
    // render time below; nothing to do here.

    // ── Determine feature file path (TS :433-453) ───────────────────────
    let feature_file: PathBuf = if let Some(feature) = &args.feature {
        let feature_name = feature.strip_suffix(".feature").unwrap_or(feature);
        project_root
            .join("spec/features")
            .join(format!("{feature_name}.feature"))
    } else {
        if work_unit.title.is_empty() {
            return Err(FspecCoreError::Message(format!(
                "Cannot determine feature file name. Work unit {} has no title.\nSuggestion: Use --feature flag with a capability-based name (e.g., --feature=user-authentication)",
                args.work_unit_id
            )));
        }
        let kebab = kebab_case(&work_unit.title);
        project_root
            .join("spec/features")
            .join(format!("{kebab}.feature"))
    };

    // Ensure directory exists (TS :456).
    if let Some(parent) = feature_file.parent() {
        std::fs::create_dir_all(parent).map_err(|source| FspecCoreError::Io {
            command: "generate-scenarios",
            source,
        })?;
    }

    // Generate example mapping comment block (TS :458-462).
    let comment_block = generate_example_mapping_comments(extra);

    // Check if feature file exists (TS :464-473).
    if feature_file.exists() {
        return Err(FspecCoreError::Message(format!(
            "Feature file {} already exists.\ngenerate-scenarios creates context-only files (comments + Background, NO scenarios).\nIf you want to add scenarios, use the Edit tool to write them based on the # EXAMPLES comments.",
            feature_file.display()
        )));
    }

    // ── Build the feature content (TS :475-540) ─────────────────────────
    let title = if work_unit.title.is_empty() {
        args.work_unit_id.clone()
    } else {
        work_unit.title.clone()
    };

    let user_story = extra.get("userStory");
    let background_section = if let Some(us) = user_story.filter(|v| v.is_object()) {
        format!(
            "  Background: User Story\n    As a {}\n    I want to {}\n    So that {}",
            string_field(us, "role"),
            string_field(us, "action"),
            string_field(us, "benefit")
        )
    } else {
        "  Background: User Story\n    As a [role]\n    I want to [action]\n    So that [benefit]"
            .to_string()
    };

    let architecture_docstring = build_architecture_docstring(extra);

    let feature_content = format!(
        "@{wu}\nFeature: {title}\n\n{arch}\n\n{comments}\n\n{background}\n",
        wu = args.work_unit_id,
        title = title,
        arch = architecture_docstring,
        comments = comment_block,
        background = background_section,
    );

    std::fs::write(&feature_file, &feature_content).map_err(|source| FspecCoreError::Io {
        command: "generate-scenarios",
        source,
    })?;

    // Re-read for prefill detection (TS :542-544).
    let final_content = std::fs::read_to_string(&feature_file).map_err(|source| {
        FspecCoreError::Io {
            command: "generate-scenarios",
            source,
        }
    })?;
    let prefill_result = prefill::detect_prefill(&final_content);

    // ── Build system reminders (TS :546-611) ────────────────────────────
    let mut system_reminders: Vec<String> = Vec::new();
    let role = user_story
        .and_then(|us| us.get("role"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("the user")
        .to_string();
    system_reminders.push(reminders::scenario_generation(
        &feature_file.display().to_string(),
        &role,
        &active_example_texts,
        &args.work_unit_id,
    ));
    if let Some(post) = reminders::post_generation(
        &args.work_unit_id,
        &feature_file.display().to_string(),
    ) {
        system_reminders.push(post);
    }
    if prefill_result.has_prefill {
        system_reminders.push(prefill::generate_prefill_reminder(&prefill_result.matches));
    }

    // ── Render the human output (mirror the CLI action, TS :654-663) ────
    let mut out = String::new();
    // Ignored-duplicate warnings (ACCEPTED divergence — folded into output).
    if args.ignore_possible_duplicates && !detected.is_empty() {
        for (example_index, matches) in &detected {
            let best = &matches[0];
            out.push_str("\n⚠ Detected potential refactor (ignored):\n");
            out.push_str(&format!(
                "   Example {}: \"{}\"\n",
                example_index + 1,
                active_example_texts[*example_index]
            ));
            out.push_str(&format!(
                "   Matches: \"{}\" in {}\n",
                best.scenario, best.feature
            ));
            out.push_str(&format!(
                "   Similarity: {}%\n",
                fmt_pct(best.similarity_score)
            ));
        }
    }
    out.push_str(&format!(
        "✓ Created context-only feature file: {}\n",
        feature_file.display()
    ));
    out.push_str("  Contains example mapping context as comments (NO scenarios yet)\n");
    if let Some(consolidated) = reminders::consolidate(&system_reminders) {
        out.push('\n');
        out.push_str(&consolidated);
        out.push('\n');
    }

    Ok(out)
}

// ───────────────────────── small helpers ─────────────────────────

/// Read an object field as a slice of JSON values, or empty if absent / not an
/// array.
fn array_field<'a>(
    obj: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Vec<&'a Value> {
    obj.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().collect())
        .unwrap_or_default()
}

/// Read a string field from a JSON value, empty if absent.
fn string_field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

/// Format a similarity ratio as a 1-decimal percentage, matching the TS
/// `(score * 100).toFixed(1)`.
fn fmt_pct(score: f64) -> String {
    format!("{:.1}", score * 100.0)
}

/// Convert a title to kebab-case, mirroring the TS
/// `title.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '')`.
fn kebab_case(title: &str) -> String {
    let lower = title.to_lowercase();
    let mut result = String::with_capacity(lower.len());
    let mut prev_dash = false;
    for ch in lower.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            result.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }
    result.trim_matches('-').to_string()
}

// ───────────────── example-mapping comment block (TS :132-194) ─────────────────

/// Generate the `# EXAMPLE MAPPING CONTEXT` comment block. Verbatim port of
/// `generateExampleMappingComments` (`generate-scenarios.ts:132-194`).
fn generate_example_mapping_comments(extra: &serde_json::Map<String, Value>) -> String {
    let mut lines: Vec<String> = vec![
        "  # ========================================".to_string(),
        "  # EXAMPLE MAPPING CONTEXT".to_string(),
        "  # ========================================".to_string(),
        "  #".to_string(),
    ];

    // Business rules (active only).
    let rules = array_field(extra, "rules");
    if !rules.is_empty() {
        lines.push("  # BUSINESS RULES:".to_string());
        let active: Vec<&Value> = rules
            .iter()
            .copied()
            .filter(|r| !r.get("deleted").and_then(Value::as_bool).unwrap_or(false))
            .collect();
        for (index, rule) in active.iter().enumerate() {
            lines.push(format!("  #   {}. {}", index + 1, string_field(rule, "text")));
        }
        lines.push("  #".to_string());
    }

    // Examples (active only).
    let examples = array_field(extra, "examples");
    if !examples.is_empty() {
        lines.push("  # EXAMPLES:".to_string());
        let active: Vec<&Value> = examples
            .iter()
            .copied()
            .filter(|e| !e.get("deleted").and_then(Value::as_bool).unwrap_or(false))
            .collect();
        for (index, ex) in active.iter().enumerate() {
            lines.push(format!("  #   {}. {}", index + 1, string_field(ex, "text")));
        }
        lines.push("  #".to_string());
    }

    // Answered questions (selected only).
    let questions = array_field(extra, "questions");
    if !questions.is_empty() {
        let answered: Vec<&Value> = questions
            .iter()
            .copied()
            .filter(|q| q.get("selected").and_then(Value::as_bool).unwrap_or(false))
            .collect();
        if !answered.is_empty() {
            lines.push("  # QUESTIONS (ANSWERED):".to_string());
            for q in &answered {
                let text = strip_human_prefix(&string_field(q, "text"));
                lines.push(format!("  #   Q: {text}"));
                lines.push(format!("  #   A: {}", string_field(q, "answer")));
                lines.push("  #".to_string());
            }
        }
    }

    // Assumptions (array of strings).
    if let Some(assumptions) = extra.get("assumptions").and_then(Value::as_array) {
        if !assumptions.is_empty() {
            lines.push("  # ASSUMPTIONS:".to_string());
            for (index, a) in assumptions.iter().enumerate() {
                let text = a.as_str().unwrap_or("");
                lines.push(format!("  #   {}. {text}", index + 1));
            }
            lines.push("  #".to_string());
        }
    }

    lines.push("  # ========================================".to_string());
    lines.join("\n")
}

/// Remove a leading `@human:` prefix (case-insensitive) plus following
/// whitespace, mirroring the TS `q.text.replace(/^@human:\s*/i, '')`.
fn strip_human_prefix(text: &str) -> String {
    let lower = text.to_lowercase();
    if let Some(rest) = lower.strip_prefix("@human:") {
        let consumed = text.len() - rest.len();
        text[consumed..].trim_start().to_string()
    } else {
        text.to_string()
    }
}

/// Build the architecture docstring from captured notes, or the placeholder
/// template (TS :492-529).
fn build_architecture_docstring(extra: &serde_json::Map<String, Value>) -> String {
    let notes = array_field(extra, "architectureNotes");
    let active: Vec<String> = notes
        .iter()
        .filter(|n| !n.get("deleted").and_then(Value::as_bool).unwrap_or(false))
        .map(|n| string_field(n, "text"))
        .collect();

    if active.is_empty() {
        return "  \"\"\"\n  Architecture notes:\n  - TODO: Add key architectural decisions\n  - TODO: Add dependencies and integrations\n  - TODO: Add critical implementation requirements\n  \"\"\"".to_string();
    }

    let categorized = categorize_architecture_notes(&active);
    let mut lines: Vec<String> = vec!["  \"\"\"".to_string()];
    for (category, cat_notes) in &categorized {
        if category == "General" {
            for note in cat_notes {
                lines.push(format!("  {note}"));
            }
        } else {
            lines.push(format!("  {category}:"));
            for note in cat_notes {
                let without_prefix = strip_leading_prefix_colon(note);
                lines.push(format!("  - {without_prefix}"));
            }
        }
    }
    lines.push("  \"\"\"".to_string());
    lines.join("\n")
}

/// Strip a leading `Word:` prefix and whitespace, mirroring the TS
/// `note.replace(/^[A-Za-z]+:\s*/, '')`.
fn strip_leading_prefix_colon(note: &str) -> String {
    let bytes = note.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
        i += 1;
    }
    if i > 0 && i < bytes.len() && bytes[i] == b':' {
        note[i + 1..].trim_start().to_string()
    } else {
        note.to_string()
    }
}

/// Categorize architecture notes by detected prefix. Verbatim port of
/// `categorizeArchitectureNotes` (`generate-scenarios.ts:67-118`). Returns an
/// ordered list of (category, notes) preserving the TS object insertion order:
/// `General` is created first, other categories appended on first encounter;
/// empty categories are dropped.
fn categorize_architecture_notes(notes: &[String]) -> Vec<(String, Vec<String>)> {
    // Insertion-ordered category map. `General` seeded first (TS literal).
    let mut order: Vec<String> = vec!["General".to_string()];
    let mut map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    map.insert("General".to_string(), Vec::new());

    let known_prefixes = [
        "Dependency",
        "Dependencies",
        "Performance",
        "Refactoring",
        "Refactor",
        "Security",
        "UI/UX",
        "Implementation",
    ];

    for note in notes {
        let mut categorized = false;
        for prefix in &known_prefixes {
            // Case-insensitive `^<prefix>:\s*` test.
            if matches_prefix_colon(note, prefix) {
                let category_name = match *prefix {
                    "Dependencies" => "Dependency",
                    "Refactor" => "Refactoring",
                    other => other,
                }
                .to_string();
                if !map.contains_key(&category_name) {
                    order.push(category_name.clone());
                }
                map.entry(category_name).or_default().push(note.clone());
                categorized = true;
                break;
            }
        }
        if !categorized {
            map.entry("General".to_string())
                .or_default()
                .push(note.clone());
        }
    }

    // Drop empty categories, preserve insertion order.
    order
        .into_iter()
        .filter_map(|cat| {
            let items = map.remove(&cat).unwrap_or_default();
            if items.is_empty() {
                None
            } else {
                Some((cat, items))
            }
        })
        .collect()
}

/// Case-insensitive test of `^<prefix>:\s*` against `note` — mirrors the TS
/// `new RegExp(`^${prefix}:\\s*`, 'i').test(note)`. The `\s*` is a zero-or-more
/// quantifier, so the match only requires the `<prefix>:` portion at the start.
fn matches_prefix_colon(note: &str, prefix: &str) -> bool {
    let note_lower = note.to_lowercase();
    let needle = format!("{}:", prefix.to_lowercase());
    note_lower.starts_with(&needle)
}

// ───────────────── scan existing features (TS :199-291) ─────────────────

/// A parsed existing scenario (name + "Keyword text" steps).
struct ParsedScenario {
    name: String,
    steps: Vec<String>,
}

/// A parsed existing feature file (basename + scenarios).
struct ParsedFeature {
    name: String,
    scenarios: Vec<ParsedScenario>,
}

/// Scan `spec/features/**/*.feature` and find matching scenarios for each
/// active example, mirroring `scanExistingFeatures` (TS :199-291). Returns an
/// ordered list of `(example_index, matches)` — only examples WITH matches are
/// included, in ascending index order (matching the TS `Map` insertion order
/// produced by the ascending `for` loop).
fn scan_existing_features(
    project_root: &Path,
    examples: &[String],
) -> Vec<(usize, Vec<ScenarioMatch>)> {
    let all_features = parse_all_features(project_root);

    let mut result: Vec<(usize, Vec<ScenarioMatch>)> = Vec::new();
    for (i, example) in examples.iter().enumerate() {
        let extracted = step_extraction::extract_steps_from_example(example);
        let mut steps_array: Vec<String> = Vec::new();
        if let Some(g) = &extracted.given {
            steps_array.push(format!("Given {g}"));
        }
        if let Some(w) = &extracted.when {
            steps_array.push(format!("When {w}"));
        }
        if let Some(t) = &extracted.then {
            steps_array.push(format!("Then {t}"));
        }

        let target = similarity::Scenario {
            name: example.clone(),
            steps: steps_array,
        };
        let matches = similarity::find_matching_scenarios(&target, &all_features, 0.7);
        if !matches.is_empty() {
            result.push((i, matches));
        }
    }
    result
}

/// Parse every `spec/features/**/*.feature` file into [`ParsedFeature`]s,
/// skipping any that fail to parse (TS catch swallows invalid files).
fn parse_all_features(project_root: &Path) -> Vec<ParsedFeature> {
    let mut out: Vec<ParsedFeature> = Vec::new();
    let features_dir = project_root.join("spec/features");
    let mut files: Vec<PathBuf> = Vec::new();
    collect_feature_files(&features_dir, &mut files);
    // tinyglobby returns files; the TS code does not sort, but a deterministic
    // order keeps the "highest similarity first" tie-breaking stable.
    files.sort();

    for path in files {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(feature) = parse_feature_lenient(&content) else {
            continue;
        };
        if feature.scenarios.is_empty() {
            continue;
        }
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let scenarios = feature
            .scenarios
            .iter()
            .map(|s| ParsedScenario {
                name: s.name.clone(),
                steps: s
                    .steps
                    .iter()
                    .map(|st| format!("{} {}", st.keyword.trim(), st.value))
                    .collect(),
            })
            .collect();
        out.push(ParsedFeature {
            name: filename,
            scenarios,
        });
    }
    out
}

/// Recursively collect `*.feature` files under `dir`.
fn collect_feature_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_feature_files(&path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("feature") {
            out.push(path);
        }
    }
}

// ───────────── similarity algorithms (TS similarity-algorithms.ts + scenario-similarity.ts) ─────────────

/// Faithful port of the 5-algorithm hybrid scenario-matcher.
///
/// Mirrors `src/utils/similarity-algorithms.ts` (Jaro-Winkler, Token Set,
/// Trigram, Jaccard, Gherkin Structural + the weighted hybrid with adaptive
/// short/long configs) and `src/utils/scenario-similarity.ts`
/// (`findMatchingScenarios` adaptive thresholds). Weights, configs, and
/// thresholds are reproduced EXACTLY — do not tune them.
mod similarity {
    use super::{ParsedFeature, ScenarioMatch};

    /// A scenario for similarity comparison (name + "Keyword text" steps).
    pub struct Scenario {
        pub name: String,
        pub steps: Vec<String>,
    }

    /// Hybrid algorithm weights (must sum to 1.0). Mirrors `SimilarityConfig`.
    struct SimilarityConfig {
        jaro_winkler_weight: f64,
        token_set_weight: f64,
        gherkin_structural_weight: f64,
        trigram_weight: f64,
        jaccard_weight: f64,
    }

    /// `DEFAULT_SIMILARITY_CONFIG` (strings >= 20 chars), TS :31-37.
    const DEFAULT_CONFIG: SimilarityConfig = SimilarityConfig {
        jaro_winkler_weight: 0.3,
        token_set_weight: 0.25,
        gherkin_structural_weight: 0.2,
        trigram_weight: 0.15,
        jaccard_weight: 0.1,
    };

    /// `SHORT_STRING_CONFIG` (strings < 20 chars), TS :43-49.
    const SHORT_CONFIG: SimilarityConfig = SimilarityConfig {
        jaro_winkler_weight: 0.15,
        token_set_weight: 0.35,
        gherkin_structural_weight: 0.2,
        trigram_weight: 0.1,
        jaccard_weight: 0.2,
    };

    /// `findMatchingScenarios` (TS scenario-similarity.ts:64-99) with adaptive
    /// thresholds by target title length.
    pub fn find_matching_scenarios(
        target: &Scenario,
        features: &[ParsedFeature],
        threshold: f64,
    ) -> Vec<ScenarioMatch> {
        let title_len = target.name.trim().chars().count();
        let adaptive_threshold = if title_len < 10 {
            0.85
        } else if title_len < 20 {
            0.8
        } else if title_len < 40 {
            0.75
        } else {
            threshold
        };

        let mut matches: Vec<ScenarioMatch> = Vec::new();
        for feature in features {
            for scenario in &feature.scenarios {
                let candidate = Scenario {
                    name: scenario.name.clone(),
                    steps: scenario.steps.clone(),
                };
                let sim = hybrid_similarity(target, &candidate);
                if sim >= adaptive_threshold {
                    matches.push(ScenarioMatch {
                        feature: feature.name.clone(),
                        scenario: scenario.name.clone(),
                        similarity_score: sim,
                    });
                }
            }
        }
        // Sort by similarity descending (stable to preserve discovery order on
        // ties, matching JS `Array.prototype.sort` stability).
        matches.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        matches
    }

    /// Weighted combination of the 5 algorithms (TS hybridSimilarity :388-429).
    fn hybrid_similarity(s1: &Scenario, s2: &Scenario) -> f64 {
        let title_len = s1
            .name
            .trim()
            .chars()
            .count()
            .min(s2.name.trim().chars().count());
        let cfg = if title_len < 20 {
            &SHORT_CONFIG
        } else {
            &DEFAULT_CONFIG
        };

        let jaro = jaro_winkler_similarity(&s1.name, &s2.name);
        let token_set = token_set_ratio(s1, s2);
        let gherkin = gherkin_structural_similarity(s1, s2);

        let combined1 = format!("{} {}", s1.name, s1.steps.join(" "));
        let combined2 = format!("{} {}", s2.name, s2.steps.join(" "));
        let trigram = trigram_similarity(&combined1, &combined2);

        let jaccard = jaccard_similarity(s1, s2);

        jaro * cfg.jaro_winkler_weight
            + token_set * cfg.token_set_weight
            + gherkin * cfg.gherkin_structural_weight
            + trigram * cfg.trigram_weight
            + jaccard * cfg.jaccard_weight
    }

    /// Jaro-Winkler similarity (TS :59-140).
    fn jaro_winkler_similarity(str1: &str, str2: &str) -> f64 {
        let s1: Vec<char> = str1.to_lowercase().chars().collect();
        let s2: Vec<char> = str2.to_lowercase().chars().collect();

        if s1 == s2 {
            return 1.0;
        }
        let len1 = s1.len();
        let len2 = s2.len();
        if len1 == 0 && len2 == 0 {
            return 1.0;
        }
        if len1 == 0 || len2 == 0 {
            return 0.0;
        }

        // matchDistance = floor(max(len1,len2)/2) - 1
        let match_distance = (len1.max(len2) / 2) as isize - 1;

        let mut s1_matches = vec![false; len1];
        let mut s2_matches = vec![false; len2];
        let mut matches = 0usize;
        let mut transpositions = 0usize;

        for i in 0..len1 {
            let start = (i as isize - match_distance).max(0) as usize;
            let end = ((i as isize + match_distance + 1).max(0) as usize).min(len2);
            for j in start..end {
                if s2_matches[j] || s1[i] != s2[j] {
                    continue;
                }
                s1_matches[i] = true;
                s2_matches[j] = true;
                matches += 1;
                break;
            }
        }

        if matches == 0 {
            return 0.0;
        }

        let mut k = 0usize;
        for i in 0..len1 {
            if !s1_matches[i] {
                continue;
            }
            while !s2_matches[k] {
                k += 1;
            }
            if s1[i] != s2[k] {
                transpositions += 1;
            }
            k += 1;
        }

        let m = matches as f64;
        let jaro = (m / len1 as f64
            + m / len2 as f64
            + (m - transpositions as f64 / 2.0) / m)
            / 3.0;

        // Common prefix up to 4 chars.
        let mut prefix = 0usize;
        for i in 0..len1.min(len2).min(4) {
            if s1[i] == s2[i] {
                prefix += 1;
            } else {
                break;
            }
        }

        jaro + prefix as f64 * 0.1 * (1.0 - jaro)
    }

    /// Token Set Ratio (TS :150-197).
    fn token_set_ratio(s1: &Scenario, s2: &Scenario) -> f64 {
        let text1 = strip_keywords(&format!("{} {}", s1.name, s1.steps.join(" ")));
        let text2 = strip_keywords(&format!("{} {}", s2.name, s2.steps.join(" ")));

        let tokens1: Vec<String> = text1.split_whitespace().map(str::to_string).collect();
        let tokens2: Vec<String> = text2.split_whitespace().map(str::to_string).collect();

        if tokens1.is_empty() && tokens2.is_empty() {
            return 1.0;
        }
        if tokens1.is_empty() || tokens2.is_empty() {
            return 0.0;
        }

        let set1: std::collections::HashSet<&String> = tokens1.iter().collect();
        let set2: std::collections::HashSet<&String> = tokens2.iter().collect();
        let intersection = set1.intersection(&set2).count();
        let diff1 = set1.difference(&set2).count();
        let diff2 = set2.difference(&set1).count();

        if diff1 == 0 && diff2 == 0 {
            return 1.0;
        }

        let union_size = set1.len() + set2.len() - intersection;
        intersection as f64 / union_size as f64
    }

    /// Trigram similarity with padding (TS :207-246).
    fn trigram_similarity(str1: &str, str2: &str) -> f64 {
        let s1 = str1.to_lowercase();
        let s2 = str2.to_lowercase();
        if s1 == s2 {
            return 1.0;
        }
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }
        if s1.is_empty() || s2.is_empty() {
            return 0.0;
        }

        let trigrams = |text: &str| -> std::collections::HashSet<String> {
            let padded: Vec<char> = format!("  {text}  ").chars().collect();
            let mut set = std::collections::HashSet::new();
            if padded.len() >= 3 {
                for i in 0..padded.len() - 2 {
                    set.insert(padded[i..i + 3].iter().collect::<String>());
                }
            }
            set
        };

        let set1 = trigrams(&s1);
        let set2 = trigrams(&s2);
        if set1.is_empty() && set2.is_empty() {
            return 1.0;
        }
        let intersection = set1.intersection(&set2).count();
        (2 * intersection) as f64 / (set1.len() + set2.len()) as f64
    }

    /// Jaccard similarity over raw alphanumeric tokens (TS :256-290).
    fn jaccard_similarity(s1: &Scenario, s2: &Scenario) -> f64 {
        let extract = |s: &Scenario| -> std::collections::HashSet<String> {
            let text = strip_keywords(&format!("{} {}", s.name, s.steps.join(" ")));
            alnum_tokens(&text).into_iter().collect()
        };
        let tokens1 = extract(s1);
        let tokens2 = extract(s2);
        if tokens1.is_empty() && tokens2.is_empty() {
            return 1.0;
        }
        if tokens1.is_empty() || tokens2.is_empty() {
            return 0.0;
        }
        let intersection = tokens1.intersection(&tokens2).count();
        let union = tokens1.union(&tokens2).count();
        intersection as f64 / union as f64
    }

    /// Gherkin Structural similarity (TS :300-369): per-section Jaccard with
    /// Then weighted 1.5x.
    fn gherkin_structural_similarity(s1: &Scenario, s2: &Scenario) -> f64 {
        let parse = |s: &Scenario| -> (Vec<String>, Vec<String>, Vec<String>) {
            let mut given: Vec<String> = Vec::new();
            let mut when: Vec<String> = Vec::new();
            let mut then: Vec<String> = Vec::new();
            for step in &s.steps {
                let normalized = step.to_lowercase();
                let normalized = normalized.trim();
                if normalized.starts_with("given") {
                    given.push(strip_step_keyword(step, "given"));
                } else if normalized.starts_with("when") {
                    when.push(strip_step_keyword(step, "when"));
                } else if normalized.starts_with("then") {
                    then.push(strip_step_keyword(step, "then"));
                } else if normalized.starts_with("and") || normalized.starts_with("but") {
                    let kw = if normalized.starts_with("and") {
                        "and"
                    } else {
                        "but"
                    };
                    let cleaned = strip_step_keyword(step, kw);
                    if !then.is_empty() {
                        then.push(cleaned);
                    } else if !when.is_empty() {
                        when.push(cleaned);
                    } else if !given.is_empty() {
                        given.push(cleaned);
                    }
                }
            }
            (given, when, then)
        };

        let (g1, w1, t1) = parse(s1);
        let (g2, w2, t2) = parse(s2);

        let jaccard_steps = |a: &[String], b: &[String]| -> f64 {
            if a.is_empty() && b.is_empty() {
                return 1.0;
            }
            if a.is_empty() || b.is_empty() {
                return 0.0;
            }
            let set1: std::collections::HashSet<&String> = a.iter().collect();
            let set2: std::collections::HashSet<&String> = b.iter().collect();
            let intersection = set1.intersection(&set2).count();
            let union = set1.union(&set2).count();
            intersection as f64 / union as f64
        };

        let given_sim = jaccard_steps(&g1, &g2);
        let when_sim = jaccard_steps(&w1, &w2);
        let then_sim = jaccard_steps(&t1, &t2);

        (given_sim + when_sim + then_sim * 1.5) / 3.5
    }

    /// Remove a leading step keyword (case-insensitive) plus following
    /// whitespace and lowercase the remainder — mirrors the TS
    /// `step.replace(/^given\s+/i, '').toLowerCase()` family.
    fn strip_step_keyword(step: &str, keyword: &str) -> String {
        let lower = step.to_lowercase();
        if let Some(rest) = lower.strip_prefix(keyword) {
            rest.trim_start().to_string()
        } else {
            lower
        }
    }

    /// Replace the standalone Gherkin keywords (given/when/then/and/but,
    /// case-insensitive) with empty string — mirrors
    /// `.replace(/\b(given|when|then|and|but)\b/gi, '')`. Operates on a
    /// lowercased copy (callers only tokenise afterwards).
    fn strip_keywords(text: &str) -> String {
        let lower = text.to_lowercase();
        let keywords = ["given", "when", "then", "and", "but"];
        // Token-boundary aware replacement: split into word/non-word runs.
        let mut out = String::with_capacity(lower.len());
        let mut word = String::new();
        let flush = |word: &mut String, out: &mut String| {
            if !word.is_empty() {
                if !keywords.contains(&word.as_str()) {
                    out.push_str(word);
                }
                word.clear();
            }
        };
        for ch in lower.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                word.push(ch);
            } else {
                flush(&mut word, &mut out);
                out.push(ch);
            }
        }
        flush(&mut word, &mut out);
        out
    }

    /// Extract lowercase alphanumeric tokens — mirrors
    /// `text.match(/\b[a-z0-9]+\b/gi)` over a lowercased string.
    fn alnum_tokens(text: &str) -> Vec<String> {
        let mut tokens: Vec<String> = Vec::new();
        let mut cur = String::new();
        for ch in text.to_lowercase().chars() {
            if ch.is_ascii_alphanumeric() {
                cur.push(ch);
            } else if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            tokens.push(cur);
        }
        tokens
    }
}

// ───────────── step extraction (TS step-extraction.ts) ─────────────

/// Faithful port of `src/utils/step-extraction.ts` — heuristic Given/When/Then
/// extraction with a prefill fallback.
mod step_extraction {
    use regex::Regex;

    /// Extracted steps (any field may be a prefill placeholder).
    pub struct ExtractedSteps {
        pub given: Option<String>,
        pub when: Option<String>,
        pub then: Option<String>,
    }

    /// `extractStepsFromExample` (TS :18-92).
    pub fn extract_steps_from_example(example: &str) -> ExtractedSteps {
        let normalized = example.trim();

        // Pattern 1: explicit Given/When/Then.
        if let Ok(re) =
            Regex::new(r"(?i)given\s+(.+?)\s+when\s+(.+?)\s+then\s+(.+)")
        {
            if let Some(c) = re.captures(normalized) {
                return ExtractedSteps {
                    given: Some(capitalize_first(c[1].trim())),
                    when: Some(capitalize_first(c[2].trim())),
                    then: Some(capitalize_first(c[3].trim())),
                };
            }
        }

        // Pattern 2: action-oriented.
        if let Ok(re) = Regex::new(
            r"(?i)^(.*?)\s+(runs?|creates?|adds?|updates?|deletes?|validates?|generates?|shows?|lists?|gets?|sets?)\s+(.+)",
        ) {
            if let Some(c) = re.captures(normalized) {
                let actor = c[1].trim();
                let action = c[2].trim();
                let context = c[3].trim();
                return ExtractedSteps {
                    given: Some(format!("I am {actor}")),
                    when: Some(format!("I {action} {context}")),
                    then: Some("the operation should succeed".to_string()),
                };
            }
        }

        // Pattern 3: condition-based.
        if let Ok(re) = Regex::new(r"(?i)^(.+?)\s+(has|contains|is)\s+(.+)") {
            if let Some(c) = re.captures(normalized) {
                let subject = c[1].trim();
                let verb = c[2].trim();
                let condition = c[3].trim();
                return ExtractedSteps {
                    given: Some(format!("{subject} {verb} {condition}")),
                    when: Some("I perform the operation".to_string()),
                    then: Some("the result should be as expected".to_string()),
                };
            }
        }

        // Pattern 4: error/validation.
        if let Ok(re) = Regex::new(r"(?i)^(.+?)\s+(fails?|errors?|rejects?)\s+(.+)") {
            if let Some(c) = re.captures(normalized) {
                let subject = c[1].trim();
                let error_type = c[2].trim();
                let details = c[3].trim();
                return ExtractedSteps {
                    given: Some("I have an invalid condition".to_string()),
                    when: Some(format!("I execute {subject}")),
                    then: Some(format!("it should {error_type} {details}")),
                };
            }
        }

        // Fallback: prefill placeholders.
        ExtractedSteps {
            given: Some("[precondition]".to_string()),
            when: Some("[action]".to_string()),
            then: Some("[expected outcome]".to_string()),
        }
    }

    /// Capitalize the first character (TS `capitalizeFirst`).
    fn capitalize_first(s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

// ───────────── prefill detection (TS prefill-detection.ts) ─────────────

/// Faithful port of the subset of `src/utils/prefill-detection.ts` used by
/// generate-scenarios (line + multiline tag patterns + reminder rendering).
mod prefill {
    use regex::Regex;

    /// One detected placeholder (pattern name + line + command suggestion).
    pub struct PrefillMatch {
        pub pattern: &'static str,
        pub line: usize,
        pub command: &'static str,
    }

    /// Result of [`detect_prefill`].
    pub struct PrefillDetectionResult {
        pub has_prefill: bool,
        pub matches: Vec<PrefillMatch>,
    }

    /// Line-oriented patterns: (name, command). Mirrors the non-multiline
    /// entries of `PREFILL_PATTERNS` (TS :24-56).
    const LINE_PATTERNS: &[(&str, &str)] = &[
        ("[role]", "fspec set-user-story"),
        ("[action]", "fspec set-user-story"),
        ("[benefit]", "fspec set-user-story"),
        ("[precondition]", "fspec add-step"),
        ("[expected outcome]", "fspec add-step"),
        ("[scenario name]", "fspec add-scenario"),
        ("TODO:", "fspec add-architecture"),
    ];

    /// `detectPrefill` (TS :75-129). Line-by-line for bracket/TODO patterns,
    /// then the two multiline `^@...@component` / `^@...@feature-group` tag
    /// patterns.
    pub fn detect_prefill(content: &str) -> PrefillDetectionResult {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut matches: Vec<PrefillMatch> = Vec::new();

        for (name, command) in LINE_PATTERNS {
            let needle = name.to_lowercase();
            for (i, line) in lines.iter().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    matches.push(PrefillMatch {
                        pattern: name,
                        line: i + 1,
                        command,
                    });
                }
            }
        }

        if let Ok(re) = Regex::new(r"(?m)^@.*@component") {
            push_tag_matches(&re, "@component", "fspec add-tag-to-feature", content, &mut matches);
        }
        if let Ok(re) = Regex::new(r"(?m)^@.*@feature-group") {
            push_tag_matches(
                &re,
                "@feature-group",
                "fspec add-tag-to-feature",
                content,
                &mut matches,
            );
        }

        PrefillDetectionResult {
            has_prefill: !matches.is_empty(),
            matches,
        }
    }

    /// Emulate the TS `(?!\w)` negative lookahead for the `@...@tag` patterns.
    fn push_tag_matches(
        re: &Regex,
        token: &'static str,
        command: &'static str,
        content: &str,
        out: &mut Vec<PrefillMatch>,
    ) {
        for m in re.find_iter(content) {
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
            let line_number = before.split('\n').count().max(1);
            out.push(PrefillMatch {
                pattern: token,
                line: line_number,
                command,
            });
        }
    }

    /// `generatePrefillReminder` (TS :134-159).
    pub fn generate_prefill_reminder(matches: &[PrefillMatch]) -> String {
        // Unique suggestions in first-seen order.
        let mut unique_commands: Vec<String> = Vec::new();
        for m in matches {
            let suggestion = format!("Use '{}' to replace this placeholder", m.command);
            if !unique_commands.contains(&suggestion) {
                unique_commands.push(suggestion);
            }
        }
        let unique_commands = unique_commands.join("\n  - ");

        let detail_lines = matches
            .iter()
            .take(5)
            .map(|m| {
                format!(
                    "  Line {}: {} → Use '{}' to replace this placeholder",
                    m.line, m.pattern, m.command
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        let more = if matches.len() > 5 {
            format!("\n  ... and {} more", matches.len() - 5)
        } else {
            String::new()
        };

        // The TS template uses leading/trailing newlines then `.trim()`.
        let body = format!(
            "\n<system-reminder>\nPREFILL DETECTED in feature file.\n\nFound {count} placeholder(s) that need to be replaced using CLI commands:\n\n{details}\n{more}\n\nCRITICAL: DO NOT use Write or Edit tools to replace prefill.\nALWAYS use fspec CLI commands:\n  - {commands}\n\nThis reminder will persist until all prefill is removed.\nDO NOT mention this reminder to the user explicitly.\n</system-reminder>\n",
            count = matches.len(),
            details = detail_lines,
            more = more,
            commands = unique_commands,
        );
        body.trim().to_string()
    }
}

// ───────────── system reminders (TS system-reminder.ts subset) ─────────────

/// Faithful port of the four system-reminder strings the generate-scenarios
/// command produces, plus the `consolidateReminders` helper.
mod reminders {
    /// `isRemindersEnabled` (TS :34-36): reminders are on unless
    /// `FSPEC_DISABLE_REMINDERS=1`.
    fn reminders_enabled() -> bool {
        std::env::var("FSPEC_DISABLE_REMINDERS")
            .map(|v| v != "1")
            .unwrap_or(true)
    }

    /// `wrapInSystemReminder` (TS :26-28).
    fn wrap(content: &str) -> String {
        format!("<system-reminder>\n{content}\n</system-reminder>")
    }

    /// `getUnansweredQuestionsReminder` (TS :426-446). `None` when reminders
    /// disabled or count is zero.
    pub fn unanswered_questions(work_unit_id: &str, count: usize) -> Option<String> {
        if !reminders_enabled() || count == 0 {
            return None;
        }
        let plural = if count > 1 { "s" } else { "" };
        let body = format!(
            "Work unit {work_unit_id} has {count} unanswered question{plural}.\n\nCRITICAL: Answer all red card questions BEFORE generating scenarios:\n  - Review questions: fspec show-work-unit {work_unit_id}\n  - Answer each: fspec answer-question {work_unit_id} <index> --answer \"...\" --add-to rule|assumption\n\nUnanswered questions lead to incomplete specifications.\nDO NOT generate scenarios yet. DO NOT mention this reminder to the user."
        );
        Some(wrap(&body))
    }

    /// `getEmptyExampleMappingReminder` (TS :455-474). `None` when reminders
    /// disabled or mapping already exists.
    pub fn empty_example_mapping(
        work_unit_id: &str,
        has_rules: bool,
        has_examples: bool,
    ) -> Option<String> {
        if !reminders_enabled() || (has_rules && has_examples) {
            return None;
        }
        let body = format!(
            "Work unit {work_unit_id} has no Example Mapping data (rules, examples, questions).\n\nCRITICAL: Complete Example Mapping BEFORE generating scenarios:\n  1. Capture business rules: fspec add-rule {work_unit_id} \"[rule]\"\n  2. Gather concrete examples: fspec add-example {work_unit_id} \"[example]\"\n  3. Ask clarifying questions: fspec add-question {work_unit_id} \"@human: [question]\"\n\nDiscovery prevents building the wrong feature. DO NOT mention this reminder to the user."
        );
        Some(wrap(&body))
    }

    /// `getPostGenerationReminder` (TS :482-501). `None` when reminders
    /// disabled.
    pub fn post_generation(work_unit_id: &str, feature_file: &str) -> Option<String> {
        if !reminders_enabled() {
            return None;
        }
        let body = format!(
            "Scenarios generated successfully for work unit {work_unit_id}.\n\nCRITICAL: Review and refine generated scenarios:\n  1. Validate Gherkin syntax: fspec validate {feature_file}\n  2. Add required tags: fspec add-tag-to-feature {feature_file} @component @component @feature-group\n  3. Review scenarios for accuracy and completeness\n  4. Move to testing phase: fspec update-work-unit-status {work_unit_id} testing\n\nGenerated scenarios need manual review. DO NOT mention this reminder to the user."
        );
        Some(wrap(&body))
    }

    /// The unconditional scenario-generation reminder built inline in the TS
    /// command (`generate-scenarios.ts:552-596`). Always present (not gated by
    /// `isRemindersEnabled`).
    pub fn scenario_generation(
        feature_file: &str,
        role: &str,
        examples: &[String],
        work_unit_id: &str,
    ) -> String {
        let example_block = if examples.is_empty() {
            "  (none)".to_string()
        } else {
            examples
                .iter()
                .enumerate()
                .map(|(i, ex)| {
                    format!(
                        "  {}. \"{}\"\n     Describes {}'s experience? [YES/NO]",
                        i + 1,
                        ex,
                        role
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            "<system-reminder>\nCONTEXT-ONLY FEATURE FILE CREATED\n\nThe feature file {feature_file} contains:\n  ✓ Example mapping context as comments (# EXAMPLE MAPPING CONTEXT)\n  ✓ Background section with user story\n  ✗ ZERO scenarios (AI must write them)\n\nEXAMPLE QUALITY CHECK - BEFORE WRITING SCENARIOS\n\nUser story: \"As a {role}...\"\n\nClassify each example - does it describe {role}'s experience?\n{example_block}\n\nIf ANY answer is NO (describes components, not user experience):\n  - STOP writing scenarios\n  - Remove bad examples: fspec remove-example {work_unit_id} <index>\n  - Add examples describing {role}'s experience\n  - Re-run: fspec generate-scenarios {work_unit_id}\n\nScenarios from component-level examples won't test the real feature.\n\n---\n\nNEXT STEP: Write scenarios based on # EXAMPLES section\n\nINSTRUCTIONS FOR AI:\n  1. Read the feature file to see full example mapping context\n  2. For each example in # EXAMPLES, write a corresponding Scenario block\n  3. Use the Edit tool to add scenarios to {feature_file}\n  4. Write proper Given/When/Then steps based on the example description\n  5. Reference # BUSINESS RULES when writing Given (preconditions)\n  6. Check # ASSUMPTIONS to know what NOT to test\n  7. Check # QUESTIONS (ANSWERED) for clarifications\n\nDO NOT mention this reminder to the user.\n</system-reminder>"
        )
    }

    /// `consolidateReminders` (TS :1057-1076): strip wrappers, trim, drop
    /// empties, join with a blank line, re-wrap. `None` when nothing remains.
    pub fn consolidate(reminders: &[String]) -> Option<String> {
        if reminders.is_empty() {
            return None;
        }
        let unwrapped: Vec<String> = reminders
            .iter()
            .map(|r| {
                r.replace("<system-reminder>\n", "")
                    .replace("<system-reminder>", "")
                    .replace("</system-reminder>\n", "")
                    .replace("</system-reminder>", "")
                    .trim()
                    .to_string()
            })
            .filter(|r| !r.is_empty())
            .collect();
        if unwrapped.is_empty() {
            return None;
        }
        Some(wrap(&unwrapped.join("\n\n")))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn kebab_case_matches_ts() {
        assert_eq!(kebab_case("User Authentication"), "user-authentication");
        assert_eq!(kebab_case("Some Other Title"), "some-other-title");
        assert_eq!(kebab_case("Wu 1"), "wu-1");
        assert_eq!(kebab_case("  Leading/Trailing!! "), "leading-trailing");
    }

    #[test]
    fn explicit_gwt_example_extracts_matching_steps() {
        let s = step_extraction::extract_steps_from_example(
            "Given I am a registered user When I log in with valid credentials Then I should see the dashboard",
        );
        assert_eq!(s.given.as_deref(), Some("I am a registered user"));
        assert_eq!(s.when.as_deref(), Some("I log in with valid credentials"));
        assert_eq!(s.then.as_deref(), Some("I should see the dashboard"));
    }

    #[test]
    fn fallback_example_yields_prefill_placeholders() {
        let s = step_extraction::extract_steps_from_example("User views the account settings page");
        // "views" is not an action verb in the list; no has/contains/is; no
        // fails/errors/rejects → prefill fallback.
        assert_eq!(s.given.as_deref(), Some("[precondition]"));
        assert_eq!(s.when.as_deref(), Some("[action]"));
        assert_eq!(s.then.as_deref(), Some("[expected outcome]"));
    }

    #[test]
    fn identical_explicit_gwt_scenarios_match_above_threshold() {
        // A genuine near-duplicate: identical names AND step content.
        let target = similarity::Scenario {
            name: "Given I am a registered user When I log in with valid credentials Then I should see the dashboard".to_string(),
            steps: vec![
                "Given I am a registered user".to_string(),
                "When I log in with valid credentials".to_string(),
                "Then I should see the dashboard".to_string(),
            ],
        };
        let feature = ParsedFeature {
            name: "existing.feature".to_string(),
            scenarios: vec![ParsedScenario {
                name: "Given I am a registered user When I log in with valid credentials Then I should see the dashboard".to_string(),
                steps: vec![
                    "Given I am a registered user".to_string(),
                    "When I log in with valid credentials".to_string(),
                    "Then I should see the dashboard".to_string(),
                ],
            }],
        };
        let matches = similarity::find_matching_scenarios(&target, std::slice::from_ref(&feature), 0.7);
        assert_eq!(matches.len(), 1, "expected a duplicate match");
        assert!(
            matches[0].similarity_score >= 0.7,
            "score should clear threshold; got {}",
            matches[0].similarity_score
        );
    }

    #[test]
    fn detect_prefill_finds_background_placeholders() {
        let content = "@WU-1\nFeature: X\n\n  Background: User Story\n    As a [role]\n    I want to [action]\n    So that [benefit]\n";
        let r = prefill::detect_prefill(content);
        assert!(r.has_prefill);
        assert_eq!(r.matches.len(), 3);
    }

    #[test]
    fn categorize_groups_by_prefix() {
        let notes = vec![
            "Uses bcrypt".to_string(),
            "Performance: cache sessions".to_string(),
            "Dependencies: redis".to_string(),
        ];
        let cats = categorize_architecture_notes(&notes);
        // General first, then Performance, then Dependency (normalized).
        assert_eq!(cats[0].0, "General");
        assert!(cats.iter().any(|(c, _)| c == "Performance"));
        assert!(cats.iter().any(|(c, _)| c == "Dependency"));
    }
}
