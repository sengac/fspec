//! Feature: spec/features/fspec-config-writes-are-atomic-and-tests-never-touch-the-real-user-config.feature
//!
//! BUG-640: two processes write one project spec/fspec-config.json at once
//! through `update_config_with_dirs` (one locked read-modify-write each), so
//! the file stays valid JSON and keeps both processes' changes. The child
//! processes are this test binary, re-run with only the ignored
//! `child_writer` test selected.

use std::path::Path;
use std::process::{Command, Stdio};

use codelet_common::fspec_config::{update_config_with_dirs, ConfigScope};
use serde_json::Value;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Updates each child makes: enough that the two processes' writes overlap.
const UPDATES: usize = 40;

/// The child half: `UPDATES` read-modify-writes of the project config named
/// by `BUG640_PROJECT`, each adding the key `<BUG640_KEY>-<n>`.
#[test]
#[ignore = "run only as a child process of concurrent_processes_writing_the_project_config_keep_both_changes"]
fn child_writer() -> TestResult {
    let project = std::env::var("BUG640_PROJECT")?;
    let key = std::env::var("BUG640_KEY")?;
    let data = tempfile::tempdir()?;
    for update in 0..UPDATES {
        update_config_with_dirs(
            ConfigScope::Project,
            data.path(),
            Path::new(&project),
            |root| {
                root[format!("{key}-{update}")] = Value::from(update);
                Ok::<_, String>(())
            },
        )?;
    }
    Ok(())
}

#[test]
fn concurrent_processes_writing_the_project_config_keep_both_changes() -> TestResult {
    // @step Given a project spec/fspec-config.json
    let project = tempfile::tempdir()?;
    std::fs::create_dir_all(project.path().join("spec"))?;
    let config_path = project.path().join("spec").join("fspec-config.json");
    std::fs::write(&config_path, "{\"agent\": \"claude\"}\n")?;

    // @step When two processes each write a different key to it at once
    let children = ["first", "second"]
        .into_iter()
        .map(|key| {
            Command::new(std::env::current_exe()?)
                .args(["--ignored", "--exact", "child_writer", "--test-threads=1"])
                .env("BUG640_PROJECT", project.path())
                .env("BUG640_KEY", key)
                .stdout(Stdio::null())
                .spawn()
        })
        .collect::<Result<Vec<_>, _>>()?;
    for mut child in children {
        assert!(child.wait()?.success(), "a child writer failed");
    }

    // @step Then the file is valid JSON
    let config: Value = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;

    // @step And it holds both keys
    for key in ["first", "second"] {
        for update in 0..UPDATES {
            assert_eq!(
                config[format!("{key}-{update}")],
                Value::from(update),
                "{key}-{update} was lost: {config}"
            );
        }
    }
    assert_eq!(config["agent"], Value::from("claude"));
    Ok(())
}
