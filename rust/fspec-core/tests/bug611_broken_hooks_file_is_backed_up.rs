//! Feature: spec/features/a-broken-hooks-file-is-backed-up-before-a-new-one-is-written.feature
//!
//! BUG-611: add-hook silently replaced an unparseable or wrong-shape
//! spec/fspec-hooks.json with an empty config, losing every hook, and
//! list-hooks and validate-hooks reported it as "no hooks". Every hook-config
//! writer now moves the broken file to spec/fspec-hooks.json.broken-<timestamp>
//! with an exact notice before writing a new config, never over an existing
//! backup; list-hooks and validate-hooks report the broken file and leave it
//! unchanged (product owner, 2026-11-07).

use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use codelet_fspec_core::commands::add_hook;
use codelet_fspec_core::{dispatch_command, DispatchRequest, DispatchResult};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The JSON parser's message for `{ not json`.
const NOT_JSON_REASON: &str = "key must be a string at line 1 column 3";

const BACKUP_PREFIX: &str = "fspec-hooks.json.broken-";

fn hooks_file(root: &Path) -> PathBuf {
    root.join("spec").join("fspec-hooks.json")
}

/// A project whose spec/fspec-hooks.json holds exactly `raw`.
fn project_with_hooks_file(raw: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let project = tempfile::tempdir()?;
    std::fs::create_dir_all(project.path().join("spec"))?;
    std::fs::write(hooks_file(project.path()), raw)?;
    Ok(project)
}

fn dispatch(root: &Path, command: &str, args: Value) -> DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: root.to_path_buf(),
    })
}

fn add_lint_hook(root: &Path) -> DispatchResult {
    dispatch(
        root,
        "add-hook",
        json!({ "event": "pre-implementing", "name": "lint", "command": "lint.sh", "blocking": false }),
    )
}

/// The names of every backup in spec/, sorted.
fn backups(root: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(root.join("spec"))? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name.starts_with(BACKUP_PREFIX) {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

/// The one backup in spec/, checked to carry a UTC
/// `%Y%m%dT%H%M%S%.3fZ` timestamp.
fn only_backup(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let names = backups(root)?;
    let [name] = names.as_slice() else {
        return Err(format!("expected exactly one backup, found {names:?}").into());
    };
    let stamp = name.trim_start_matches(BACKUP_PREFIX);
    NaiveDateTime::parse_from_str(stamp, "%Y%m%dT%H%M%S%.3fZ")
        .map_err(|e| format!("backup {name} has no %Y%m%dT%H%M%S.%3fZ timestamp: {e}"))?;
    if stamp.len() != "20261108T013000.123Z".len() {
        return Err(format!("backup {name} has a timestamp of the wrong length").into());
    }
    Ok(name.clone())
}

/// BUG-638: add-hook's output, the backup notice then the added hook.
fn added_after(notice: &str) -> String {
    format!("{notice}\n✓ Hook added: pre-implementing/lint")
}

fn notice(reason: &str, backup: &str) -> String {
    format!(
        "spec/fspec-hooks.json could not be parsed ({reason}); it was moved to spec/{backup} \
         and a new hook configuration was written"
    )
}

fn fixed_now() -> Result<DateTime<Utc>, Box<dyn std::error::Error>> {
    let at = Utc
        .with_ymd_and_hms(2026, 11, 8, 1, 30, 0)
        .single()
        .ok_or("no such time")?;
    Ok(at + chrono::Duration::milliseconds(123))
}

#[test]
fn add_hook_backs_up_an_unparseable_hooks_file_and_writes_a_new_one() -> TestResult {
    // @step Given spec/fspec-hooks.json holds "{ not json"
    let project = project_with_hooks_file("{ not json")?;

    // @step When I run "fspec add-hook pre-implementing lint --command lint.sh"
    let result = add_lint_hook(project.path());
    assert!(result.success, "{:?}", result.error);

    // @step Then it prints "spec/fspec-hooks.json could not be parsed (<reason>); it was moved to spec/fspec-hooks.json.broken-<timestamp> and a new hook configuration was written" naming the backup and the JSON parser's message
    let backup = only_backup(project.path())?;
    assert_eq!(result.data, added_after(&notice(NOT_JSON_REASON, &backup)));

    // @step And the backup holds exactly "{ not json"
    assert_eq!(
        std::fs::read(project.path().join("spec").join(&backup))?,
        b"{ not json"
    );

    // @step And spec/fspec-hooks.json holds only the hook "lint" under "pre-implementing"
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(hooks_file(project.path()))?)?;
    assert_eq!(
        written,
        json!({ "hooks": { "pre-implementing": [
            { "name": "lint", "command": "lint.sh", "blocking": false }
        ] } })
    );
    Ok(())
}

#[test]
fn a_hooks_file_of_the_wrong_shape_is_backed_up_like_an_unparseable_one() -> TestResult {
    // @step Given spec/fspec-hooks.json holds {"hooks": []}
    let project = project_with_hooks_file(r#"{"hooks": []}"#)?;

    // @step When I run "fspec add-hook pre-implementing lint --command lint.sh"
    let result = add_lint_hook(project.path());
    assert!(result.success, "{:?}", result.error);

    // @step Then it prints the backup notice with the reason "hooks is not an object"
    let backup = only_backup(project.path())?;
    assert_eq!(
        result.data,
        added_after(&notice("hooks is not an object", &backup))
    );

    // @step And the backup holds exactly {"hooks": []}
    assert_eq!(
        std::fs::read(project.path().join("spec").join(&backup))?,
        br#"{"hooks": []}"#
    );
    Ok(())
}

#[test]
fn remove_hook_backs_up_an_unparseable_hooks_file_and_writes_an_empty_configuration() -> TestResult
{
    // @step Given spec/fspec-hooks.json holds "{ not json"
    let project = project_with_hooks_file("{ not json")?;

    // @step When I run "fspec remove-hook pre-implementing lint"
    let result = dispatch(
        project.path(),
        "remove-hook",
        json!({ "event": "pre-implementing", "name": "lint" }),
    );
    assert!(result.success, "{:?}", result.error);

    // @step Then it prints the backup notice naming the backup
    let backup = only_backup(project.path())?;
    assert_eq!(result.data, notice(NOT_JSON_REASON, &backup));

    // @step And the backup holds exactly "{ not json"
    assert_eq!(
        std::fs::read(project.path().join("spec").join(&backup))?,
        b"{ not json"
    );

    // @step And spec/fspec-hooks.json holds {"hooks": {}}
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(hooks_file(project.path()))?)?;
    assert_eq!(written, json!({ "hooks": {} }));
    Ok(())
}

#[test]
fn an_existing_backup_is_never_overwritten() -> TestResult {
    // @step Given spec/fspec-hooks.json holds "{ not json"
    let project = project_with_hooks_file("{ not json")?;

    // @step And its backup name for the current time already exists
    let now = fixed_now()?;
    let backup = "fspec-hooks.json.broken-20261108T013000.123Z";
    let backup_path = project.path().join("spec").join(backup);
    std::fs::write(&backup_path, "an earlier backup")?;

    // @step When I run "fspec add-hook pre-implementing lint --command lint.sh"
    let args = json!({ "event": "pre-implementing", "name": "lint", "command": "lint.sh", "blocking": false });
    let result = tokio::runtime::Builder::new_current_thread()
        .build()?
        .block_on(add_hook::run_at(&args.to_string(), project.path(), now));

    // @step Then it fails with "spec/fspec-hooks.json could not be parsed (<reason>), and its backup spec/fspec-hooks.json.broken-<timestamp> already exists; nothing was changed"
    let error = result.err().map(|e| e.to_string());
    assert_eq!(
        error.as_deref(),
        Some(
            format!(
                "spec/fspec-hooks.json could not be parsed ({NOT_JSON_REASON}), and its backup \
                 spec/{backup} already exists; nothing was changed"
            )
            .as_str()
        )
    );

    // @step And spec/fspec-hooks.json and the existing backup are unchanged
    assert_eq!(std::fs::read(hooks_file(project.path()))?, b"{ not json");
    assert_eq!(std::fs::read(&backup_path)?, b"an earlier backup");
    assert_eq!(backups(project.path())?, vec![backup.to_string()]);
    Ok(())
}

#[test]
fn list_hooks_and_validate_hooks_report_a_broken_hooks_file_without_changing_it() -> TestResult {
    // @step Given spec/fspec-hooks.json holds "{ not json"
    let project = project_with_hooks_file("{ not json")?;
    let not_changed = format!(
        "spec/fspec-hooks.json could not be parsed ({NOT_JSON_REASON}); the file was not changed"
    );

    // @step When I run "fspec list-hooks"
    let listed = dispatch(project.path(), "list-hooks", json!({}));

    // @step Then it fails with "spec/fspec-hooks.json could not be parsed (<reason>); the file was not changed"
    assert!(!listed.success, "{:?}", listed.data);
    assert_eq!(listed.error.as_deref(), Some(not_changed.as_str()));

    // @step When I run "fspec validate-hooks"
    let validated = dispatch(project.path(), "validate-hooks", json!({}));

    // @step Then it reports valid=false and exitCode 1 with that message
    assert!(validated.success, "{:?}", validated.error);
    let report: Value = serde_json::from_str(&validated.data)?;
    assert_eq!(
        report,
        json!({ "valid": false, "exitCode": 1, "message": not_changed })
    );

    // @step And spec/fspec-hooks.json still holds "{ not json" and no backup exists
    assert_eq!(std::fs::read(hooks_file(project.path()))?, b"{ not json");
    assert_eq!(backups(project.path())?, Vec::<String>::new());
    Ok(())
}
