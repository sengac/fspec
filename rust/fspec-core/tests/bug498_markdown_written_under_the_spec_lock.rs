//! BUG-498 — generated spec markdown is written under the spec lock.
//!
//! Feature: spec/features/generated-spec-markdown-and-the-schedule-log-are-written-under-the-spec-lock.feature

use std::path::Path;
use std::sync::{Mutex, PoisonError};

use codelet_common::spec_lock::{SpecLock, SPEC_LOCK_TIMEOUT_ENV};
use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The repository's own registry, a schema-valid tags.json.
fn tags() -> Result<String, Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .ok_or("repository root")?
        .to_path_buf();
    Ok(std::fs::read_to_string(root.join("spec/tags.json"))?)
}

fn foundation() -> serde_json::Value {
    json!({
        "version": "2.0.0",
        "project": { "name": "Demo", "vision": "V", "projectType": "cli-tool" },
        "problemSpace": { "primaryProblem": { "title": "t", "description": "d", "impact": "medium" } },
        "solutionSpace": { "overview": "o", "capabilities": [{ "name": "C", "description": "d" }] }
    })
}

/// Hold the spec lock of `root` on this thread while `command` runs on
/// another; return whether `output` existed once that locked run had
/// returned, its content after a run with the lock free, and whether that
/// run succeeded.
///
/// TEST-042: no sleep. The locked run gets a lock budget of 0 ms, so it
/// gives up at once instead of waiting, and the output is checked after it
/// has returned: anything written outside the lock would exist by then.
fn run_while_locked(
    root: &Path,
    command: &'static str,
    output: &str,
) -> Result<(bool, String, bool), Box<dyn std::error::Error>> {
    // The budget is process-wide; one locked run at a time.
    static SERIAL: Mutex<()> = Mutex::new(());
    let _serial = SERIAL.lock().unwrap_or_else(PoisonError::into_inner);
    let spec = root.join("spec");
    let lock = SpecLock::acquire(&spec)?;
    let project = root.to_path_buf();
    std::env::set_var(SPEC_LOCK_TIMEOUT_ENV, "0");
    let locked_run = std::thread::spawn(move || run(&project, command)).join();
    std::env::remove_var(SPEC_LOCK_TIMEOUT_ENV);
    let locked_run = locked_run.map_err(|_| "the locked run panicked")?;
    assert!(
        !locked_run.success
            && locked_run
                .error
                .as_deref()
                .is_some_and(|error| error.contains("Failed to acquire file lock")),
        "{command} must not run while the spec lock is held: {locked_run:?}"
    );
    let written_while_locked = spec.join(output).exists();
    drop(lock);
    let succeeded = run(root, command).success;
    Ok((
        written_while_locked,
        std::fs::read_to_string(spec.join(output))?,
        succeeded,
    ))
}

fn run(project: &Path, command: &str) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: json!({}).to_string(),
        project_root: project.to_path_buf(),
    })
}

#[test]
fn generating_tags_md_waits_for_the_spec_lock() -> TestResult {
    // @step Given another process holds the spec lock of a project with registered tags
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("spec"))?;
    std::fs::write(root.path().join("spec/tags.json"), tags()?)?;
    // @step When generate-tags-md runs
    let (early, content, succeeded) = run_while_locked(root.path(), "generate-tags-md", "TAGS.md")?;
    // @step Then TAGS.md is not written until the lock is released
    assert!(!early, "TAGS.md was written while the spec lock was held");
    // @step And afterwards TAGS.md holds the complete generated document
    assert!(succeeded);
    assert!(content.contains("@wip"), "{content}");
    Ok(())
}

#[test]
fn generating_foundation_md_waits_for_the_spec_lock() -> TestResult {
    // @step Given another process holds the spec lock of a project with a foundation
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("spec"))?;
    std::fs::write(
        root.path().join("spec/foundation.json"),
        serde_json::to_string(&foundation())?,
    )?;
    // @step When generate-foundation-md runs
    let (early, content, succeeded) =
        run_while_locked(root.path(), "generate-foundation-md", "FOUNDATION.md")?;
    // @step Then FOUNDATION.md is not written until the lock is released
    assert!(
        !early,
        "FOUNDATION.md was written while the spec lock was held"
    );
    // @step And afterwards FOUNDATION.md holds the complete generated document
    assert!(succeeded);
    assert!(content.contains("Demo"), "{content}");
    Ok(())
}
