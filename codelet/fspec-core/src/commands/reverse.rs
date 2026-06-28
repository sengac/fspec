//! `reverse` — Rust port of `src/commands/reverse.ts` (RPC-294).
//!
//! Interactive reverse-ACDD strategy planning. Analyzes project gaps and guides
//! the agent step-by-step through a reverse-ACDD session persisted in the OS
//! temp directory (see [`crate::types::reverse_session`]).
//!
//! Control-flow priority (FIRST MATCH WINS), mirroring `reverse.ts:29-253`:
//!   1. reset   → delete session, "Session reset".
//!   2. status  → load session or "No active reverse session".
//!   3. complete→ load + validateCompletion; delete on success.
//!   4. continue→ load + incrementStep + save; next-file guidance.
//!   5. strategy→ Strategy-D persona path (dispatcher-only) OR load + setStrategy.
//!   6. existing session present (no flag) → blocked "Existing reverse session detected".
//!   7. initial analysis (no flag, no session) → analyze + detectGaps; dry-run OR create session.
//!
//! All persistence is blocking `std::fs` (no async), so the future resolves on
//! the first poll under `dispatch::poll_sync_future`.
//!
//! Two-front-doors: the dispatcher passes a JSON args object; the standalone
//! binary's clap bridge marshals its six flags into the same JSON shape. The
//! `implementationContext` field (Strategy-D persona path) is dispatcher-only —
//! it is NOT exposed as a clap flag (parity with the TS Commander surface).

use std::path::Path;

use serde::Deserialize;

use crate::error::FspecCoreError;
use crate::types::reverse_session::{
    create_session, delete_session, increment_step, load_session, save_session, session_exists,
    set_strategy, validate_completion, AnalysisResult, CoverageAnalysis, GapAnalysis,
};

/// CLI / dispatcher arguments for `reverse`. Mirrors the TS
/// `ReverseCommandOptions` shape. All flags are optional booleans except
/// `strategy` (a single letter) and `implementationContext` (dispatcher-only).
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct ReverseArgs {
    #[serde(default)]
    strategy: Option<String>,
    #[serde(default, rename = "continue")]
    r#continue: bool,
    #[serde(default)]
    status: bool,
    #[serde(default)]
    reset: bool,
    #[serde(default)]
    complete: bool,
    #[serde(default)]
    dry_run: bool,
    /// Strategy-D persona path — dispatcher-JSON-only, never a clap flag.
    #[serde(default)]
    implementation_context: Option<String>,
}

/// Dispatcher / CLI entry point. Returns the CLI-wrapper-equivalent rendered
/// text on success; escalates failures (existing session, corrupt session,
/// not-all-steps-finished, no active session) via [`FspecCoreError`].
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: ReverseArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "reverse",
            reason: format!("failed to parse args: {e}"),
        })?;

    // ---- 1. reset ----
    if args.reset {
        delete_session(project_root);
        return Ok(render_message("Session reset"));
    }

    // ---- 2. status ----
    if args.status {
        return Ok(render_status(project_root));
    }

    // ---- 3. complete ----
    if args.complete {
        return handle_complete(project_root);
    }

    // ---- 4. continue ----
    if args.r#continue {
        return handle_continue(project_root);
    }

    // ---- 5. strategy ----
    if let Some(strategy) = args.strategy.as_deref() {
        // Strategy D persona path works WITHOUT a session (dispatcher-only).
        if strategy == "D" {
            if let Some(ctx) = args.implementation_context.as_deref() {
                return Ok(handle_strategy_d(project_root, ctx));
            }
        }
        return handle_strategy(project_root, strategy);
    }

    // ---- 6. existing session present ----
    if session_exists(project_root) {
        return handle_existing_session(project_root);
    }

    // ---- 7. initial analysis ----
    handle_initial_analysis(project_root, args.dry_run)
}

// ---------- rendering helpers ----------

/// Wrap content in a `<system-reminder>` block (parity with `wrapSystemReminder`).
fn wrap_system_reminder(content: &str) -> String {
    format!("<system-reminder>\n{content}\n</system-reminder>")
}

/// Render a bare message line (the CLI wrapper prints `message` on its own line).
fn render_message(message: &str) -> String {
    message.to_string()
}

/// Render the CLI-wrapper output order: systemReminder, message, guidance,
/// then suggestions under "Next steps:" (parity with `reverseCommand`,
/// `reverse.ts:629-655`).
fn render_output(
    system_reminder: Option<&str>,
    message: Option<&str>,
    guidance: Option<&str>,
    suggestions: &[&str],
) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(sr) = system_reminder {
        lines.push(sr.to_string());
    }
    if let Some(m) = message {
        lines.push(m.to_string());
    }
    if let Some(g) = guidance {
        lines.push(g.to_string());
    }
    if !suggestions.is_empty() {
        lines.push("\nNext steps:".to_string());
        for s in suggestions {
            lines.push(format!("  - {s}"));
        }
    }
    lines.join("\n")
}

/// Human-readable strategy name (parity with `getStrategyName`).
fn get_strategy_name(strategy: &str) -> &'static str {
    match strategy {
        "A" => "Spec Gap Filling",
        "B" => "Test Gap Filling",
        "C" => "Coverage Mapping",
        "D" => "Full Reverse ACDD",
        _ => "Unknown Strategy",
    }
}

/// First non-zero gap summary string (parity with `formatGaps`).
fn format_gaps(gaps: &GapAnalysis) -> String {
    if gaps.tests_without_features > 0 {
        format!(
            "{} test files without features",
            gaps.tests_without_features
        )
    } else if gaps.features_without_tests > 0 {
        format!(
            "{} feature files without tests",
            gaps.features_without_tests
        )
    } else if gaps.unmapped_scenarios > 0 {
        format!(
            "{} scenarios without coverage mappings",
            gaps.unmapped_scenarios
        )
    } else if gaps.unmapped_implementation > 0 {
        format!(
            "{} implementation files without features",
            gaps.unmapped_implementation
        )
    } else {
        "No gaps detected".to_string()
    }
}

/// Per-strategy static guidance (parity with `generateStrategyGuidance`).
fn generate_strategy_guidance(strategy: &str) -> String {
    match strategy {
        "A" => "Create feature files for existing tests. Reverse engineer acceptance criteria from test assertions.".to_string(),
        "B" => "Create test skeletons for existing feature files. Use --skip-validation when linking coverage.".to_string(),
        "C" => "Quick wins - no new files needed. Link existing tests to scenarios using fspec link-coverage.".to_string(),
        "D" => "Highest effort - analyze code, create features, tests, and work units from scratch.".to_string(),
        _ => String::new(),
    }
}

/// Effort estimate from total gap count (parity with `getEffortEstimate`).
fn get_effort_estimate(strategy: &str, gaps: &GapAnalysis) -> String {
    let total = gaps.tests_without_features
        + gaps.features_without_tests
        + gaps.unmapped_scenarios
        + gaps.unmapped_implementation;
    match strategy {
        "A" => format!("{}-{} points", total * 2, total * 3),
        "B" => format!("{}-{} points", total, total * 2),
        "C" => "1 point total".to_string(),
        "D" => format!("{}-{} points", total * 3, total * 5),
        _ => "Unknown".to_string(),
    }
}

/// Suggest a strategy by gap priority A→B→C→D, default A
/// (parity with `suggestStrategy`).
fn suggest_strategy(gaps: &GapAnalysis) -> &'static str {
    if gaps.tests_without_features > 0 {
        "A"
    } else if gaps.features_without_tests > 0 {
        "B"
    } else if gaps.unmapped_scenarios > 0 {
        "C"
    } else if gaps.unmapped_implementation > 0 {
        "D"
    } else {
        "A"
    }
}

// ---------- branch handlers ----------

/// `--status`: render the no-session sentinel OR (for an active session)
/// the EMPTY CLI body, mirroring the TS `reverseCommand` wrapper exactly.
///
/// IMPORTANT PARITY NOTE (`reverse.ts:38-62` + `629-655`): the TS `--status`
/// branch returns a STRUCTURED result object carrying `phase` / `strategy` /
/// `gapsDetected` / `progress` / `gapList`, but the CLI action
/// (`reverseCommand`) only ever prints `systemReminder`, `message`,
/// `guidance`, and `suggestions`. The active-session status result has NONE
/// of those four fields, so the CLI prints NOTHING (0 bytes, exit 0) — and
/// the Fspec-tool path captures the same empty `output.log` stream. The
/// `Phase:`/`Progress:`/`Files:` lines documented in the help EXAMPLE describe
/// the structured result fields, NOT runtime stdout. Both front doors must
/// therefore emit an empty body for an active session.
fn render_status(project_root: &Path) -> String {
    match load_session(project_root) {
        // No session → the sentinel `message` line (the one field the CLI
        // wrapper does print for status).
        None => render_message("No active reverse session"),
        // Active session → empty body (parity with the TS CLI wrapper, which
        // logs none of the structured status fields).
        Some(_) => String::new(),
    }
}

/// `--complete`: validate completion, delete the session, emit the completion
/// reminder + message (parity with `reverse.ts:65-88`).
fn handle_complete(project_root: &Path) -> Result<String, FspecCoreError> {
    let session = match load_session(project_root) {
        Some(s) => s,
        None => {
            return Err(FspecCoreError::Message(
                "No active reverse session to complete".to_string(),
            ))
        }
    };

    if !validate_completion(&session) {
        return Err(FspecCoreError::Message(
            "Cannot complete: not all steps are finished".to_string(),
        ));
    }

    delete_session(project_root);
    let reminder = wrap_system_reminder("Session completed successfully.\nAll gaps filled.");
    Ok(render_output(
        Some(&reminder),
        Some("✓ Reverse ACDD session complete"),
        None,
        &[],
    ))
}

/// `--continue`: increment step, persist, emit next-file guidance (parity with
/// `reverse.ts:91-114`).
fn handle_continue(project_root: &Path) -> Result<String, FspecCoreError> {
    let session = match load_session(project_root) {
        Some(s) => s,
        None => {
            return Err(FspecCoreError::Message(
                "No active reverse session".to_string(),
            ))
        }
    };

    let updated = increment_step(session);
    save_session(project_root, &updated).map_err(|e| FspecCoreError::Io {
        command: "reverse",
        source: e,
    })?;

    let current = updated.current_step.unwrap_or(1);
    let total = updated.total_steps.unwrap_or(0);
    let is_final = updated.current_step == updated.total_steps;
    let next_file = updated
        .gaps
        .files
        .get((current as usize).saturating_sub(1))
        .cloned()
        .unwrap_or_default();

    let next_cmd = if is_final {
        "After completing this final step, run: fspec reverse --complete"
    } else {
        "After completing this step, run: fspec reverse --continue"
    };
    let reminder = wrap_system_reminder(&format!(
        "Step {current} of {total}\nProcess file: {next_file}\n{next_cmd}"
    ));
    let guidance = format!(
        "Process test file: {next_file}. Read the file, create feature file, then link coverage."
    );
    Ok(render_output(Some(&reminder), None, Some(&guidance), &[]))
}

/// `--strategy=<X>` (non Strategy-D persona path): load session, setStrategy,
/// persist, emit step-1 guidance (parity with `reverse.ts:123-148`).
fn handle_strategy(project_root: &Path, strategy: &str) -> Result<String, FspecCoreError> {
    let session = match load_session(project_root) {
        Some(s) => s,
        None => {
            return Err(FspecCoreError::Message(
                "No active reverse session".to_string(),
            ))
        }
    };

    let strategy_name = get_strategy_name(strategy);
    let total_steps = session.gaps.files.len() as u64;
    let first_file = session.gaps.files.first().cloned().unwrap_or_default();
    let updated = set_strategy(session, strategy, strategy_name, total_steps);
    save_session(project_root, &updated).map_err(|e| FspecCoreError::Io {
        command: "reverse",
        source: e,
    })?;

    let reminder = wrap_system_reminder(&format!(
        "Step 1 of {total_steps}\nStrategy: {strategy} ({strategy_name})\nAfter completing this step, run: fspec reverse --continue"
    ));
    let guidance = format!(
        "Read test file: {first_file}. Then create feature file. Then run fspec link-coverage with --skip-validation."
    );
    Ok(render_output(Some(&reminder), None, Some(&guidance), &[]))
}

/// Existing-session-detected block (no flag, session present). Escalates as an
/// error carrying the rendered suggestion list (parity with `reverse.ts:152-181`).
///
/// PARITY: the TS result object carries `currentPhase` / `currentStrategy` /
/// `currentProgress`, but the CLI wrapper (`reverseCommand`, `reverse.ts:629-655`)
/// only ever logs `systemReminder`, `message`, `guidance`, and `suggestions`.
/// The rendered body is therefore the reminder, the `message`
/// ("Existing reverse session detected"), then the suggestions under
/// "Next steps:" — the `current*` fields are NEVER printed.
fn handle_existing_session(project_root: &Path) -> Result<String, FspecCoreError> {
    // A present-but-unparseable session file → corrupt JSON sentinel
    // (parity with `reverse.ts:154-156`).
    if load_session(project_root).is_none() {
        return Err(FspecCoreError::Message(
            "Session file corrupted".to_string(),
        ));
    }

    let reminder = wrap_system_reminder(
        "Existing session detected. DO NOT start new session.\nEither continue the existing session or reset it first.",
    );
    let suggestions = [
        "fspec reverse --continue",
        "fspec reverse --status",
        "fspec reverse --reset",
        "fspec reverse --complete",
    ];

    let mut body = String::new();
    body.push_str(&reminder);
    body.push('\n');
    body.push_str("Existing reverse session detected");
    body.push_str("\n\nNext steps:");
    for s in suggestions {
        body.push_str(&format!("\n  - {s}"));
    }

    Err(FspecCoreError::Message(body))
}

/// Initial analysis (no flag, no session). Analyzes the project, detects gaps,
/// and either previews (dry-run) or creates a gap-detection session
/// (parity with `reverse.ts:184-253`).
fn handle_initial_analysis(project_root: &Path, dry_run: bool) -> Result<String, FspecCoreError> {
    let analysis = analyze_project(project_root);
    let gaps = detect_gaps(&analysis);
    let suggested = suggest_strategy(&gaps);
    let strategy_name = get_strategy_name(suggested);

    if dry_run {
        let reminder = wrap_system_reminder(&format!(
            "DRY-RUN MODE: Analysis complete, no session created.\nDetected: {}\nSuggested: Strategy {} ({})",
            format_gaps(&gaps),
            suggested,
            strategy_name
        ));
        let guidance = generate_strategy_guidance(suggested);
        return Ok(render_output(
            Some(&reminder),
            Some("Dry-run mode - no session created"),
            Some(&guidance),
            &[],
        ));
    }

    // Create the gap-detection session.
    let session = create_session(
        "gap-detection",
        gaps.clone(),
        Some(suggested.to_string()),
        Some(strategy_name.to_string()),
    );
    save_session(project_root, &session).map_err(|e| FspecCoreError::Io {
        command: "reverse",
        source: e,
    })?;

    let total_gaps = gaps.tests_without_features
        + gaps.features_without_tests
        + gaps.unmapped_scenarios
        + gaps.unmapped_implementation;

    let reminder = wrap_system_reminder(&format!(
        "Gap analysis complete.\nDetected: {}\nSuggested: Strategy {} ({})\nTo choose this strategy, run: fspec reverse --strategy={}",
        format_gaps(&gaps),
        suggested,
        strategy_name,
        suggested
    ));
    let mut guidance = generate_strategy_guidance(suggested);

    // Large projects (100+ gaps): append narrow-scope hint to guidance.
    if total_gaps >= 100 {
        if guidance.is_empty() {
            guidance = format!("Use --strategy={suggested} to narrow scope.");
        } else {
            guidance = format!("{guidance}\n\nUse --strategy={suggested} to narrow scope.");
        }
    }

    // The CLI wrapper does NOT print effortEstimate as a standalone line, but
    // we compute it here for parity with the structured result. It is folded
    // into neither stdout nor the reminder (matching reverseCommand).
    let _ = get_effort_estimate(suggested, &gaps);

    Ok(render_output(Some(&reminder), None, Some(&guidance), &[]))
}

/// Strategy-D persona-driven discovery (dispatcher-only; works WITHOUT a
/// session). Reads `spec/foundation.json` personas and builds the persona
/// guidance reminder (parity with `handleStrategyD`, `reverse.ts:554-624`).
fn handle_strategy_d(project_root: &Path, implementation_context: &str) -> String {
    #[derive(Deserialize)]
    struct Persona {
        #[serde(default)]
        name: String,
        #[serde(default)]
        goals: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Foundation {
        #[serde(default)]
        personas: Vec<Persona>,
    }

    let personas: Vec<Persona> =
        std::fs::read_to_string(project_root.join("spec").join("foundation.json"))
            .ok()
            .and_then(|c| serde_json::from_str::<Foundation>(&c).ok())
            .map(|f| f.personas)
            .unwrap_or_default();

    let mut sr = String::from("REVERSE ACDD - PERSONA-DRIVEN DISCOVERY\n\n");
    sr.push_str(&format!(
        "Implementation context: {implementation_context}\n\n"
    ));

    if !personas.is_empty() {
        sr.push_str("WHO uses this? (Check foundation.json personas)\n");
        for persona in &personas {
            sr.push_str(&format!("  - {}\n", persona.name));
            if !persona.goals.is_empty() {
                sr.push_str(&format!("    Goals: {}\n", persona.goals.join(", ")));
            }
        }
        sr.push('\n');
        let first = &personas[0].name;
        sr.push_str(&format!("What does {first} want to accomplish?\n"));
        sr.push_str("Think outside-in (BDD approach):\n");
        sr.push_str("  Not: \"component has play/pause buttons\"\n");
        sr.push_str(&format!("  Instead: \"{first} controls playback\"\n\n"));
    } else {
        sr.push_str("Foundation.json not found or has no personas.\n");
        sr.push_str("Run: fspec discover-foundation\n\n");
    }

    sr.push_str("What user behavior does this support?\n");
    sr.push_str("not which system calls it, but who BENEFITS?\n");
    sr.push_str("  Not: \"which system calls it\"\n");
    sr.push_str("  Instead: \"who BENEFITS from accurate discounts\" → Shopper\n\n");

    sr.push_str("Transformation templates (implementation → behavior):\n");
    sr.push_str("  • UI Elements → User Actions\n");
    sr.push_str("    button → \"User clicks/taps ACTION\"\n");
    sr.push_str("    input → \"User enters DATA\"\n");
    sr.push_str("  • State → User Expectations\n");
    sr.push_str("    useState → \"User sees STATE\"\n");
    sr.push_str("    loading → \"User waits for PROCESS\"\n");
    sr.push_str("  • API Endpoints → User Needs\n");
    sr.push_str("    POST /orders → \"User completes order\"\n\n");

    sr.push_str("Create user-centric scenarios based on persona goals.\n");

    let mut guidance = String::from("ACDD Workflow:\n");
    guidance.push_str("1. Use example mapping: fspec add-example <work-unit> \"...\"\n");
    guidance.push_str("2. Use example mapping: fspec add-rule <work-unit> \"...\"\n");
    guidance.push_str("3. Generate scenarios: fspec generate-scenarios <work-unit>\n");
    guidance.push_str("4. Create test skeletons based on scenarios\n");
    guidance.push_str("5. Link coverage: fspec link-coverage <feature> --scenario \"...\" --test-file <path> --test-lines <range> --skip-validation\n");

    let reminder = wrap_system_reminder(&sr);
    render_output(Some(&reminder), None, Some(&guidance), &[])
}

// ---------- project analysis ----------

/// Analyze the project: test files, feature files, implementation files, and
/// coverage gaps (parity with `analyzeProject`, `reverse.ts:256-269`).
fn analyze_project(cwd: &Path) -> AnalysisResult {
    let test_files = find_test_files(cwd);
    let feature_files = find_feature_files(cwd);
    let implementation_files = find_implementation_files(cwd);
    let coverage_analysis = analyze_coverage(cwd, &feature_files);

    let summary = format!(
        "Found {} test files, {} feature files, {} implementation files",
        test_files.len(),
        feature_files.len(),
        implementation_files.len()
    );

    AnalysisResult {
        test_files,
        feature_files,
        implementation_files,
        coverage_analysis,
        summary,
    }
}

/// Scan the canonical test directories (non-recursive) for `*.test.{ts,js,tsx,jsx}`
/// files (parity with `findTestFiles`, `reverse.ts:271-290`).
fn find_test_files(cwd: &Path) -> Vec<String> {
    let test_dirs = ["src/__tests__", "test", "tests", "__tests__"];
    let mut out: Vec<String> = Vec::new();
    for dir in test_dirs {
        let full = cwd.join(dir);
        let entries = match std::fs::read_dir(&full) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let mut names: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().into_owned();
                if is_test_file_name(&name) {
                    names.push(name);
                }
            }
        }
        names.sort();
        for name in names {
            out.push(format!("{dir}/{name}"));
        }
    }
    out
}

/// True if `name` matches `/\.test\.(ts|js|tsx|jsx)$/`.
fn is_test_file_name(name: &str) -> bool {
    for ext in [".test.ts", ".test.js", ".test.tsx", ".test.jsx"] {
        if name.ends_with(ext) {
            return true;
        }
    }
    false
}

/// List `spec/features/*.feature` (non-recursive); ENOENT → empty
/// (parity with `findFeatureFiles`, `reverse.ts:292-302`).
fn find_feature_files(cwd: &Path) -> Vec<String> {
    let dir = cwd.join("spec").join("features");
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".feature") {
                names.push(name);
            }
        }
    }
    names.sort();
    names
        .into_iter()
        .map(|n| format!("spec/features/{n}"))
        .collect()
}

/// Recursively walk `src/`, skipping test directories, collecting
/// `.{ts,js,tsx,jsx}` files that are NOT `*.test.ts`, as paths relative to
/// `cwd` (parity with `findImplementationFiles` + `scanDirectory`,
/// `reverse.ts:304-340`).
fn find_implementation_files(cwd: &Path) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    let src = cwd.join("src");
    scan_directory(&src, cwd, &mut files);
    files
}

fn scan_directory(dir: &Path, cwd: &Path, files: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    // Collect + sort entries for deterministic ordering.
    let mut dir_entries: Vec<std::fs::DirEntry> = entries.flatten().collect();
    dir_entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in dir_entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let full = entry.path();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            if !matches!(name.as_str(), "__tests__" | "tests" | "test") {
                scan_directory(&full, cwd, files);
            }
        } else if is_impl_file_name(&name) {
            // Relative to cwd: strip the cwd prefix + separator.
            let rel = full
                .strip_prefix(cwd)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| full.to_string_lossy().into_owned());
            files.push(rel);
        }
    }
}

/// True if `name` matches `/\.(ts|js|tsx|jsx)$/` and does NOT end `.test.ts`.
fn is_impl_file_name(name: &str) -> bool {
    if name.ends_with(".test.ts") {
        return false;
    }
    name.ends_with(".ts")
        || name.ends_with(".js")
        || name.ends_with(".tsx")
        || name.ends_with(".jsx")
}

/// For each feature file, read its `.coverage` sidecar and count scenarios with
/// no `testMappings`. Returns `None` when there are no feature files
/// (parity with `analyzeCoverage`, `reverse.ts:514-549`).
fn analyze_coverage(cwd: &Path, feature_files: &[String]) -> Option<CoverageAnalysis> {
    if feature_files.is_empty() {
        return None;
    }
    let mut unmapped_count: u64 = 0;
    let mut scenarios: Vec<String> = Vec::new();
    for feature_file in feature_files {
        let coverage_path = cwd.join(format!("{feature_file}.coverage"));
        let content = match std::fs::read_to_string(&coverage_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let parsed: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(arr) = parsed.get("scenarios").and_then(|s| s.as_array()) {
            for scenario in arr {
                let mappings = scenario.get("testMappings").and_then(|m| m.as_array());
                let empty = mappings.map(Vec::is_empty).unwrap_or(true);
                if empty {
                    unmapped_count += 1;
                    let name = scenario.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    scenarios.push(format!("{feature_file}:{name}"));
                }
            }
        }
    }
    Some(CoverageAnalysis {
        unmapped_count,
        scenarios,
    })
}

// ---------- gap detection ----------

/// Derive a kebab-case feature name from an implementation path
/// (parity with `deriveFeatureName`, `reverse.ts:349-357`).
fn derive_feature_name(impl_path: &str) -> String {
    let filename = impl_path.rsplit('/').next().unwrap_or(impl_path);
    let mut base = filename;
    for ext in [".tsx", ".jsx", ".ts", ".js"] {
        if let Some(stripped) = base.strip_suffix(ext) {
            base = stripped;
            break;
        }
    }
    // camelCase → kebab: insert '-' between a lower/digit and an upper.
    let chars: Vec<char> = base.chars().collect();
    let mut out = String::with_capacity(chars.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase() && i > 0 {
            let prev = chars[i - 1];
            // lower→Upper boundary (camelCase) OR Upper followed by lower while
            // prev was Upper (PascalCase run end), matching the two TS regexes.
            let next_lower = chars
                .get(i + 1)
                .map(char::is_ascii_lowercase)
                .unwrap_or(false);
            if prev.is_ascii_lowercase() || (prev.is_ascii_uppercase() && next_lower) {
                out.push('-');
            }
        }
        out.push(c);
    }
    out.to_lowercase()
}

/// Whether a feature file exists for an implementation file
/// (parity with `hasFeatureFile`, `reverse.ts:362-373`).
fn has_feature_file(impl_path: &str, feature_files: &[String]) -> bool {
    let feature_name = derive_feature_name(impl_path);
    let expected = format!("spec/features/{feature_name}.feature");
    let suffix = format!("/{feature_name}.feature");
    feature_files
        .iter()
        .any(|f| f == &expected || f.contains(&suffix))
}

/// Implementation files lacking a corresponding feature file
/// (parity with `findUnmappedImplementation`, `reverse.ts:378-385`).
fn find_unmapped_implementation(
    implementation_files: &[String],
    feature_files: &[String],
) -> Vec<String> {
    implementation_files
        .iter()
        .filter(|f| !has_feature_file(f, feature_files))
        .cloned()
        .collect()
}

/// Whether a file is a "pure utility" that should be skipped
/// (parity with `isPureUtility`, `reverse.ts:391-401`).
fn is_pure_utility(impl_path: &str) -> bool {
    let lower = impl_path.to_lowercase();
    lower.contains("utils/format")
        || lower.contains("utils/parse")
        || lower.contains("utils/validate")
        || lower.contains("helpers/")
        || lower.contains("constants/")
}

/// Detect gaps from a project analysis (parity with `detectGaps`,
/// `reverse.ts:403-443`).
fn detect_gaps(analysis: &AnalysisResult) -> GapAnalysis {
    let test_len = analysis.test_files.len();
    let feature_len = analysis.feature_files.len();

    let unmapped_count = analysis
        .coverage_analysis
        .as_ref()
        .map(|c| c.unmapped_count)
        .unwrap_or(0);

    let unmapped_impl_files: Vec<String> =
        find_unmapped_implementation(&analysis.implementation_files, &analysis.feature_files)
            .into_iter()
            .filter(|f| !is_pure_utility(f))
            .collect();

    // Files to process, by gap type (first match wins).
    let files: Vec<String> = if test_len > 0 && feature_len == 0 {
        analysis.test_files.clone()
    } else if feature_len > 0 && test_len == 0 {
        analysis.feature_files.clone()
    } else if unmapped_count > 0 {
        analysis
            .coverage_analysis
            .as_ref()
            .map(|c| c.scenarios.clone())
            .unwrap_or_default()
    } else if !unmapped_impl_files.is_empty() {
        unmapped_impl_files.clone()
    } else {
        Vec::new()
    };

    GapAnalysis {
        tests_without_features: if test_len > 0 && feature_len == 0 {
            test_len as u64
        } else {
            0
        },
        features_without_tests: if feature_len > 0 && test_len == 0 {
            feature_len as u64
        } else {
            0
        },
        unmapped_scenarios: unmapped_count,
        unmapped_implementation: unmapped_impl_files.len() as u64,
        files,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn derive_feature_name_camel_and_pascal() {
        assert_eq!(
            derive_feature_name("src/components/MusicPlayer.tsx"),
            "music-player"
        );
        assert_eq!(
            derive_feature_name("src/hooks/usePlaylistStore.ts"),
            "use-playlist-store"
        );
        assert_eq!(
            derive_feature_name("src/utils/formatTime.js"),
            "format-time"
        );
    }

    #[test]
    fn suggest_strategy_priority_order() {
        let gaps = GapAnalysis {
            tests_without_features: 0,
            features_without_tests: 0,
            unmapped_scenarios: 0,
            unmapped_implementation: 2,
            files: vec![],
        };
        assert_eq!(suggest_strategy(&gaps), "D");
        let gaps_a = GapAnalysis {
            tests_without_features: 3,
            features_without_tests: 1,
            unmapped_scenarios: 5,
            unmapped_implementation: 2,
            files: vec![],
        };
        assert_eq!(suggest_strategy(&gaps_a), "A");
    }

    #[test]
    fn format_gaps_first_non_zero() {
        let gaps = GapAnalysis {
            tests_without_features: 3,
            features_without_tests: 0,
            unmapped_scenarios: 0,
            unmapped_implementation: 0,
            files: vec![],
        };
        assert_eq!(format_gaps(&gaps), "3 test files without features");
    }

    #[test]
    fn get_strategy_name_known_and_unknown() {
        assert_eq!(get_strategy_name("A"), "Spec Gap Filling");
        assert_eq!(get_strategy_name("Z"), "Unknown Strategy");
    }

    #[test]
    fn is_pure_utility_matches_patterns() {
        assert!(is_pure_utility("src/utils/formatDate.ts"));
        assert!(is_pure_utility("src/helpers/foo.ts"));
        assert!(!is_pure_utility("src/commands/reverse.ts"));
    }

    #[test]
    fn effort_estimate_per_strategy() {
        let gaps = GapAnalysis {
            tests_without_features: 3,
            features_without_tests: 0,
            unmapped_scenarios: 0,
            unmapped_implementation: 0,
            files: vec![],
        };
        assert_eq!(get_effort_estimate("A", &gaps), "6-9 points");
        assert_eq!(get_effort_estimate("C", &gaps), "1 point total");
    }
}
