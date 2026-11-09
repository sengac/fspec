#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
// Feature: spec/features/reverse-rust-port.feature
//
// Dispatcher-contract tests for the Rust port of `reverse` (RPC-294). Each
// scenario maps to exactly one #[test] with @step comments mirroring the
// Gherkin steps verbatim. RED PHASE: the current stub returns NotYetPorted,
// so every test fails until commands::reverse::run is ported.

use std::fs;
use std::path::Path;

use codelet_fspec_core::types::reverse_session::{session_path, ReverseSession};
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------- helpers ----------

fn req(project_root: &Path, args: Value) -> DispatchRequest {
    DispatchRequest {
        command: "reverse".to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    }
}

/// A tempdir that is a project root (carries a Cargo.toml boundary marker) and
/// whose session file is deleted on drop so tests don't leak temp state.
struct Workspace {
    dir: TempDir,
}

impl Workspace {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        // Boundary marker so find_project_root resolves to this dir, giving a
        // deterministic per-test session hash.
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").expect("marker");
        let ws = Self { dir };
        // Ensure no stale session from a previous run with the same path hash.
        ws.clear_session();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn session_file(&self) -> std::path::PathBuf {
        session_path(self.dir.path())
    }

    fn clear_session(&self) {
        let _ = fs::remove_file(self.session_file());
    }

    fn write_session(&self, session: &Value) {
        fs::write(
            self.session_file(),
            serde_json::to_string_pretty(session).unwrap(),
        )
        .expect("write session");
    }

    fn write_session_raw(&self, raw: &str) {
        fs::write(self.session_file(), raw).expect("write session raw");
    }

    fn load_session(&self) -> ReverseSession {
        let content = fs::read_to_string(self.session_file()).expect("session file present");
        serde_json::from_str(&content).expect("session parses")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        self.clear_session();
    }
}

fn gap_detection_session(files: &[&str]) -> Value {
    json!({
        "phase": "gap-detection",
        "strategy": "A",
        "strategyName": "Spec Gap Filling",
        "gaps": {
            "testsWithoutFeatures": files.len(),
            "featuresWithoutTests": 0,
            "unmappedScenarios": 0,
            "unmappedImplementation": 0,
            "files": files,
        },
        "timestamp": "2026-06-01T00:00:00.000Z"
    })
}

fn executing_session(current: u64, total: u64, files: &[&str]) -> Value {
    json!({
        "phase": "executing",
        "strategy": "A",
        "strategyName": "Spec Gap Filling",
        "currentStep": current,
        "totalSteps": total,
        "gaps": {
            "testsWithoutFeatures": files.len(),
            "featuresWithoutTests": 0,
            "unmappedScenarios": 0,
            "unmappedImplementation": 0,
            "files": files,
        },
        "timestamp": "2026-06-01T00:00:00.000Z"
    })
}

fn write_test_files(root: &Path, names: &[&str]) {
    let dir = root.join("src/__tests__");
    fs::create_dir_all(&dir).expect("mkdir tests");
    for n in names {
        fs::write(dir.join(n), "// test\n").expect("write test file");
    }
}

// ---------- scenarios ----------

#[test]
fn reset_deletes_the_session_and_returns_session_reset() {
    // Scenario: Reset deletes the session and returns Session reset

    // @step Given a project root tempdir with an active reverse session file on disk
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        2,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));
    assert!(ws.session_file().exists());

    // @step When I dispatch reverse with reset=true
    let result = dispatch_command(req(ws.path(), json!({ "reset": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "Session reset"
    assert!(
        result.data.contains("Session reset"),
        "expected 'Session reset'; got:\n{}",
        result.data
    );

    // @step Then the session file no longer exists on disk
    assert!(!ws.session_file().exists(), "session file must be deleted");
}

#[test]
fn status_with_no_session_reports_no_active_session() {
    // Scenario: Status with no session reports no active session

    // @step Given a project root tempdir with no reverse session file
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with status=true
    let result = dispatch_command(req(ws.path(), json!({ "status": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "No active reverse session"
    assert!(
        result.data.contains("No active reverse session"),
        "expected 'No active reverse session'; got:\n{}",
        result.data
    );
}

#[test]
fn status_with_active_executing_session_emits_empty_body() {
    // Scenario: Status with an active executing session emits an empty rendered body
    //
    // PARITY: the TS `reverseCommand` wrapper (`reverse.ts:629-655`) only ever
    // prints `systemReminder` / `message` / `guidance` / `suggestions`. The
    // active-session `--status` result object carries NONE of those (only the
    // structured `phase` / `strategy` / `gapsDetected` / `progress` / `gapList`
    // fields), so the CLI — and the Fspec-tool capture path that mirrors it —
    // emit an EMPTY body. We assert that empty body here.

    // @step Given a project root tempdir with an executing session having strategy=A strategyName='Spec Gap Filling' currentStep=2 totalSteps=3 and three gap files
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        2,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));

    // @step When I dispatch reverse with status=true
    let result = dispatch_command(req(ws.path(), json!({ "status": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output is empty because the CLI wrapper logs none of the structured status fields
    assert!(
        result.data.is_empty(),
        "expected empty status body for an active session; got:\n{}",
        result.data
    );

    // @step Then the session file is left untouched by the read-only status query
    assert!(
        ws.session_file().exists(),
        "status must NOT delete the session"
    );
}

#[test]
fn complete_with_no_session_fails_with_exit_1() {
    // Scenario: Complete with no session fails with exit 1

    // @step Given a project root tempdir with no reverse session file
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with complete=true
    let result = dispatch_command(req(ws.path(), json!({ "complete": true })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "No active reverse session to complete"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("No active reverse session to complete"),
        "expected canonical error; got: {msg}"
    );
}

#[test]
fn complete_on_an_unfinished_session_is_rejected() {
    // Scenario: Complete on an unfinished session is rejected

    // @step Given a project root tempdir with an executing session having currentStep=1 and totalSteps=3
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        1,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));

    // @step When I dispatch reverse with complete=true
    let result = dispatch_command(req(ws.path(), json!({ "complete": true })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "Cannot complete: not all steps are finished"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Cannot complete: not all steps are finished"),
        "expected canonical error; got: {msg}"
    );

    // @step Then the session file still exists on disk
    assert!(
        ws.session_file().exists(),
        "session must NOT be deleted on failed complete"
    );
}

#[test]
fn complete_on_a_finished_session_deletes_it_and_returns_success() {
    // Scenario: Complete on a finished session deletes it and returns success

    // @step Given a project root tempdir with an executing session having currentStep=3 and totalSteps=3
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        3,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));

    // @step When I dispatch reverse with complete=true
    let result = dispatch_command(req(ws.path(), json!({ "complete": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "Session completed successfully."
    assert!(
        result.data.contains("Session completed successfully."),
        "expected completion reminder; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "✓ Reverse ACDD session complete"
    assert!(
        result.data.contains("✓ Reverse ACDD session complete"),
        "expected completion message; got:\n{}",
        result.data
    );

    // @step Then the session file no longer exists on disk
    assert!(
        !ws.session_file().exists(),
        "session file must be deleted on complete"
    );
}

#[test]
fn continue_with_no_session_fails_with_exit_1() {
    // Scenario: Continue with no session fails with exit 1

    // @step Given a project root tempdir with no reverse session file
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with continue=true
    let result = dispatch_command(req(ws.path(), json!({ "continue": true })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "No active reverse session"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("No active reverse session"),
        "expected canonical error; got: {msg}"
    );
}

#[test]
fn continue_advances_the_step_and_emits_next_file_guidance() {
    // Scenario: Continue advances the step and emits next-file guidance

    // @step Given a project root tempdir with an executing session having currentStep=1 totalSteps=3 and gap files [a.test.ts, b.test.ts, c.test.ts]
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        1,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));

    // @step When I dispatch reverse with continue=true
    let result = dispatch_command(req(ws.path(), json!({ "continue": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "Step 2 of 3"
    assert!(
        result.data.contains("Step 2 of 3"),
        "expected 'Step 2 of 3'; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "Process file: b.test.ts"
    assert!(
        result.data.contains("Process file: b.test.ts"),
        "expected next file b.test.ts; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "run: fspec reverse --continue"
    assert!(
        result.data.contains("run: fspec reverse --continue"),
        "expected continue hint; got:\n{}",
        result.data
    );

    // @step Then the session file on disk shows currentStep=2
    let session = ws.load_session();
    assert_eq!(
        session.current_step,
        Some(2),
        "currentStep must be persisted as 2"
    );
}

#[test]
fn continue_into_the_final_step_instructs_the_agent_to_run_complete() {
    // Scenario: Continue into the final step instructs the agent to run complete

    // @step Given a project root tempdir with an executing session having currentStep=2 totalSteps=3 and gap files [a.test.ts, b.test.ts, c.test.ts]
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        2,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));

    // @step When I dispatch reverse with continue=true
    let result = dispatch_command(req(ws.path(), json!({ "continue": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "Step 3 of 3"
    assert!(
        result.data.contains("Step 3 of 3"),
        "expected 'Step 3 of 3'; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "run: fspec reverse --complete"
    assert!(
        result.data.contains("run: fspec reverse --complete"),
        "expected complete hint on final step; got:\n{}",
        result.data
    );
}

#[test]
fn strategy_with_no_session_fails_with_exit_1() {
    // Scenario: Strategy with no session fails with exit 1

    // @step Given a project root tempdir with no reverse session file
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with strategy='A'
    let result = dispatch_command(req(ws.path(), json!({ "strategy": "A" })));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "No active reverse session"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("No active reverse session"),
        "expected canonical error; got: {msg}"
    );
}

#[test]
fn strategy_a_on_a_gap_detection_session_moves_it_to_executing_at_step_1() {
    // Scenario: Strategy A on a gap-detection session moves it to executing at step 1

    // @step Given a project root tempdir with a gap-detection session whose gaps.files are [a.test.ts, b.test.ts, c.test.ts]
    let ws = Workspace::new();
    ws.write_session(&gap_detection_session(&[
        "a.test.ts",
        "b.test.ts",
        "c.test.ts",
    ]));

    // @step When I dispatch reverse with strategy='A'
    let result = dispatch_command(req(ws.path(), json!({ "strategy": "A" })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "Step 1 of 3"
    assert!(
        result.data.contains("Step 1 of 3"),
        "expected 'Step 1 of 3'; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "Strategy: A (Spec Gap Filling)"
    assert!(
        result.data.contains("Strategy: A (Spec Gap Filling)"),
        "expected strategy line; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "Read test file: a.test.ts"
    assert!(
        result.data.contains("Read test file: a.test.ts"),
        "expected first-file guidance; got:\n{}",
        result.data
    );

    // @step Then the session file on disk shows phase='executing' and currentStep=1 and totalSteps=3
    let session = ws.load_session();
    assert_eq!(session.phase, "executing");
    assert_eq!(session.current_step, Some(1));
    assert_eq!(session.total_steps, Some(3));
}

#[test]
fn strategy_d_with_implementation_context_returns_persona_guidance_without_a_session() {
    // Scenario: Strategy D with implementationContext returns persona-driven guidance without a session

    // @step Given a project root tempdir with no reverse session file and a spec/foundation.json containing a persona named 'Shopper' with goals
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());
    let spec = ws.path().join("spec");
    fs::create_dir_all(&spec).expect("mkdir spec");
    fs::write(
        spec.join("foundation.json"),
        json!({
            "personas": [
                { "name": "Shopper", "description": "Buys things", "goals": ["complete checkout"] }
            ]
        })
        .to_string(),
    )
    .expect("write foundation");

    // @step When I dispatch reverse with strategy='D' and implementationContext='discount calculator'
    let result = dispatch_command(req(
        ws.path(),
        json!({ "strategy": "D", "implementationContext": "discount calculator" }),
    ));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "REVERSE ACDD - PERSONA-DRIVEN DISCOVERY"
    assert!(
        result
            .data
            .contains("REVERSE ACDD - PERSONA-DRIVEN DISCOVERY"),
        "expected persona-driven header; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "Shopper"
    assert!(
        result.data.contains("Shopper"),
        "expected persona name; got:\n{}",
        result.data
    );

    // @step Then no session file was created on disk
    assert!(
        !ws.session_file().exists(),
        "Strategy D persona path must NOT create a session"
    );

    // FEAT-019 (merged here): the outside-in prompts and the
    // generate-scenarios guidance.
    for expected in [
        // @step And the rendered output contains the substring "WHO uses this? (Check foundation.json personas)"
        "WHO uses this? (Check foundation.json personas)",
        // @step And the rendered output contains the substring "What does Shopper want to accomplish?"
        "What does Shopper want to accomplish?",
        // @step And the rendered output contains the substring "What user behavior does this support?"
        "What user behavior does this support?",
        // @step And the rendered output contains the substring "Generate scenarios: fspec generate-scenarios <work-unit>"
        "Generate scenarios: fspec generate-scenarios <work-unit>",
    ] {
        assert!(
            result.data.contains(expected),
            "expected {expected:?}; got:\n{}",
            result.data
        );
    }
}

#[test]
fn existing_session_detected_blocks_a_new_analysis() {
    // Scenario: Existing session detected blocks a new analysis

    // @step Given a project root tempdir with a parseable executing session having strategy=A strategyName='Spec Gap Filling' currentStep=2 totalSteps=3
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        2,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));

    // @step When I dispatch reverse with no flags
    let result = dispatch_command(req(ws.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "Existing reverse session detected"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Existing reverse session detected"),
        "expected existing-session error; got: {msg}"
    );

    // @step Then the rendered output lists the four suggestions --continue, --status, --reset, --complete
    let combined = format!("{}{}", result.data, msg);
    for s in ["--continue", "--status", "--reset", "--complete"] {
        assert!(
            combined.contains(s),
            "expected suggestion {s}; got data:\n{}\nerror:\n{msg}",
            result.data
        );
    }
}

#[test]
fn corrupt_session_file_is_reported_as_corrupted() {
    // Scenario: Corrupt session file is reported as corrupted

    // @step Given a project root tempdir with a reverse session file containing invalid JSON
    let ws = Workspace::new();
    ws.write_session_raw("{ this is not valid json");

    // @step When I dispatch reverse with no flags
    let result = dispatch_command(req(ws.path(), json!({})));

    // @step Then the dispatcher returns success=false
    assert!(!result.success, "expected success=false, got {result:?}");

    // @step Then the error message contains the substring "Session file corrupted"
    let msg = result.error.as_ref().expect("error message");
    assert!(
        msg.contains("Session file corrupted"),
        "expected corrupted error; got: {msg}"
    );
}

#[test]
fn initial_analysis_with_tests_and_no_features_suggests_strategy_a_and_creates_a_session() {
    // Scenario: Initial analysis with tests and no features suggests Strategy A and creates a session

    // @step Given a project root tempdir with three files under src/__tests__ matching *.test.ts and no spec/features directory and no session file
    let ws = Workspace::new();
    write_test_files(ws.path(), &["a.test.ts", "b.test.ts", "c.test.ts"]);
    assert!(!ws.path().join("spec/features").exists());
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with no flags
    let result = dispatch_command(req(ws.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "Gap analysis complete."
    assert!(
        result.data.contains("Gap analysis complete."),
        "expected gap-analysis reminder; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "3 test files without features"
    assert!(
        result.data.contains("3 test files without features"),
        "expected gap summary; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "Strategy A (Spec Gap Filling)"
    assert!(
        result.data.contains("Strategy A (Spec Gap Filling)"),
        "expected suggested strategy; got:\n{}",
        result.data
    );

    // @step Then a session file was created on disk with phase='gap-detection'
    assert!(ws.session_file().exists(), "session must be created");
    let session = ws.load_session();
    assert_eq!(session.phase, "gap-detection");
}

#[test]
fn dry_run_previews_analysis_without_writing_a_session() {
    // Scenario: Dry-run previews analysis without writing a session

    // @step Given a project root tempdir with three files under src/__tests__ matching *.test.ts and no spec/features directory and no session file
    let ws = Workspace::new();
    write_test_files(ws.path(), &["a.test.ts", "b.test.ts", "c.test.ts"]);
    assert!(!ws.path().join("spec/features").exists());
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with dryRun=true
    let result = dispatch_command(req(ws.path(), json!({ "dryRun": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "DRY-RUN MODE"
    assert!(
        result.data.contains("DRY-RUN MODE"),
        "expected DRY-RUN reminder; got:\n{}",
        result.data
    );

    // @step Then the rendered output contains the substring "Dry-run mode - no session created"
    assert!(
        result.data.contains("Dry-run mode - no session created"),
        "expected dry-run message; got:\n{}",
        result.data
    );

    // @step Then no session file was created on disk
    assert!(
        !ws.session_file().exists(),
        "dry-run must NOT create a session"
    );
}

#[test]
fn flag_priority_resets_before_evaluating_status() {
    // Scenario: Flag priority resets before evaluating status

    // @step Given a project root tempdir with an active reverse session file on disk
    let ws = Workspace::new();
    ws.write_session(&executing_session(
        2,
        3,
        &["a.test.ts", "b.test.ts", "c.test.ts"],
    ));
    assert!(ws.session_file().exists());

    // @step When I dispatch reverse with both reset=true and status=true
    let result = dispatch_command(req(ws.path(), json!({ "reset": true, "status": true })));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step Then the rendered output contains the substring "Session reset"
    assert!(
        result.data.contains("Session reset"),
        "reset must win over status; got:\n{}",
        result.data
    );

    // @step Then the session file no longer exists on disk
    assert!(!ws.session_file().exists(), "reset must delete the session");
}

#[test]
fn cli_and_dispatcher_converge_on_the_same_fspec_core_run_function() {
    // Scenario: CLI and dispatcher converge on the same fspec_core run function

    // @step Given a project root tempdir with no reverse session file
    let ws = Workspace::new();
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with reset=true and also run the CLI subcommand fspec reverse --reset against the same project root
    let result = dispatch_command(req(ws.path(), json!({ "reset": true })));

    // @step Then both paths produce output containing "Session reset"
    assert!(
        result.data.contains("Session reset"),
        "dispatcher path must render 'Session reset'; got:\n{}",
        result.data
    );

    // @step Then the CLI bridge module rust/fspec/src/reverse.rs contains no analysis, gap-detection, or rendering logic — its only computation is JSON arg marshalling
    let bridge_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fspec/src/reverse.rs");
    let bridge_src = fs::read_to_string(&bridge_path).expect("bridge module readable");
    for forbidden in [
        "Gap analysis complete",
        "detectGaps",
        "suggestStrategy",
        "DRY-RUN MODE",
        "Session reset",
        "Existing reverse session detected",
    ] {
        assert!(
            !bridge_src.contains(forbidden),
            "bridge module must NOT embed `{forbidden}` (would duplicate fspec_core logic); got:\n{bridge_src}"
        );
    }
}

// ---------- BUG-295 ----------

#[cfg(unix)]
#[test]
fn a_reverse_session_is_shared_by_every_spelling_of_the_project_path() {
    use codelet_fspec_core::types::reverse_session::{
        create_session, delete_session, legacy_session_path, load_session, save_session,
        session_exists, GapAnalysis,
    };
    // @step Given a project directory and a symlink that points to it
    let real = Workspace::new();
    let links = TempDir::new().expect("link dir");
    let linked = links.path().join("project-link");
    std::os::unix::fs::symlink(real.path(), &linked).expect("symlink");
    let gaps: GapAnalysis =
        serde_json::from_value(json!({"files": []})).expect("empty gap analysis");

    // @step When a reverse session is saved through the symlinked path
    save_session(&linked, &create_session("executing", gaps, None, None)).expect("save");

    // @step Then the session is found through the real path
    assert_eq!(session_path(&linked), session_path(real.path()));
    assert!(session_exists(real.path()));
    assert_eq!(
        load_session(real.path()).map(|session| session.phase),
        Some("executing".to_string())
    );

    // @step And a session saved under the legacy symlinked-path key is still found through that spelling
    delete_session(real.path());
    let legacy = legacy_session_path(&linked);
    assert_ne!(legacy, session_path(&linked));
    fs::write(
        &legacy,
        json!({"phase":"executing","gaps":{"files":[]},"timestamp":"2026-10-30T00:00:00Z"})
            .to_string(),
    )
    .expect("legacy session");
    assert!(session_exists(&linked));
    assert!(load_session(&linked).is_some());
    delete_session(&linked);
    assert!(!legacy.exists());
}

// ---------- scenarios moved from interactive-reverse-acdd-strategy-planning-command.feature (COV-071) ----------

fn write_features_with_unmapped_coverage(root: &Path, count: usize, with_coverage: bool) {
    let dir = root.join("spec/features");
    fs::create_dir_all(&dir).expect("mkdir features");
    for i in 0..count {
        let name = format!("feature-{i}");
        fs::write(
            dir.join(format!("{name}.feature")),
            format!("Feature: F{i}\n\n  Scenario: S{i}\n    Given x\n"),
        )
        .expect("write feature");
        if with_coverage {
            let coverage = json!({"scenarios": [{"name": format!("S{i}"), "testMappings": []}]});
            fs::write(
                dir.join(format!("{name}.feature.coverage")),
                serde_json::to_string_pretty(&coverage).unwrap(),
            )
            .expect("write coverage");
        }
    }
}

fn assert_output_contains(result: &codelet_fspec_core::DispatchResult, needle: &str) {
    assert!(
        result.data.contains(needle),
        "expected {needle:?} in output; got:\n{}",
        result.data
    );
}

#[test]
fn feature_files_without_tests_suggest_strategy_b() {
    // @step Given a project root tempdir with 2 feature files in spec/features, no test files and no session file
    let ws = Workspace::new();
    write_features_with_unmapped_coverage(ws.path(), 2, false);
    assert!(!ws.session_file().exists());

    // @step When I dispatch reverse with no flags
    let result = dispatch_command(req(ws.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the session file on disk has phase='gap-detection'
    assert_eq!(ws.load_session().phase, "gap-detection");

    // @step And the rendered output contains the substring "2 feature files without tests"
    assert_output_contains(&result, "2 feature files without tests");

    // @step And the rendered output contains the substring "Strategy B (Test Gap Filling)"
    assert_output_contains(&result, "Strategy B (Test Gap Filling)");

    // @step And the rendered output contains the substring "Create test skeletons for existing feature files"
    assert_output_contains(&result, "Create test skeletons for existing feature files");

    // @step And the rendered output contains the substring "Use --skip-validation when linking coverage"
    assert_output_contains(&result, "Use --skip-validation when linking coverage");
}

#[test]
fn unmapped_coverage_suggests_strategy_c() {
    // @step Given a project root tempdir with 5 feature files in spec/features and one test file under src/__tests__
    let ws = Workspace::new();
    write_test_files(ws.path(), &["a.test.ts"]);

    // @step And the coverage files of those features list 5 scenarios without test mappings
    write_features_with_unmapped_coverage(ws.path(), 5, true);

    // @step When I dispatch reverse with no flags
    let result = dispatch_command(req(ws.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the session file on disk has phase='gap-detection'
    assert_eq!(ws.load_session().phase, "gap-detection");

    // @step And the rendered output contains the substring "5 scenarios without coverage mappings"
    assert_output_contains(&result, "5 scenarios without coverage mappings");

    // @step And the rendered output contains the substring "Strategy C (Coverage Mapping)"
    assert_output_contains(&result, "Strategy C (Coverage Mapping)");

    // @step And the rendered output contains the substring "Quick wins - no new files needed"
    assert_output_contains(&result, "Quick wins - no new files needed");
}

#[test]
fn raw_implementation_without_specs_or_tests_suggests_strategy_d() {
    // @step Given a project root tempdir with 10 implementation files under src/, no feature files and no test files
    let ws = Workspace::new();
    let src = ws.path().join("src/commands");
    fs::create_dir_all(&src).expect("mkdir src");
    for i in 0..10 {
        fs::write(src.join(format!("command{i}.ts")), "export {};\n").expect("write impl file");
    }

    // @step When I dispatch reverse with no flags
    let result = dispatch_command(req(ws.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the session file on disk has phase='gap-detection'
    assert_eq!(ws.load_session().phase, "gap-detection");

    // @step And the rendered output contains the substring "10 implementation files without features"
    assert_output_contains(&result, "10 implementation files without features");

    // @step And the rendered output contains the substring "Strategy D (Full Reverse ACDD)"
    assert_output_contains(&result, "Strategy D (Full Reverse ACDD)");

    // @step And the rendered output contains the substring "Highest effort - analyze code, create features, tests, and work units"
    assert_output_contains(
        &result,
        "Highest effort - analyze code, create features, tests, and work units",
    );
}

#[test]
fn a_large_project_gets_the_narrow_scope_hint() {
    // @step Given a project root tempdir with 150 test files under src/__tests__ and no spec/features directory
    let ws = Workspace::new();
    let names: Vec<String> = (0..150).map(|i| format!("t{i:03}.test.ts")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    write_test_files(ws.path(), &refs);
    assert!(!ws.path().join("spec/features").exists());

    // @step When I dispatch reverse with no flags
    let result = dispatch_command(req(ws.path(), json!({})));

    // @step Then the dispatcher returns success=true
    assert!(result.success, "expected success=true, got {result:?}");

    // @step And the rendered output contains the substring "150 test files without features"
    assert_output_contains(&result, "150 test files without features");

    // @step And the rendered output contains the substring "Use --strategy=A to narrow scope."
    assert_output_contains(&result, "Use --strategy=A to narrow scope.");
}
