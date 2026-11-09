// Feature: spec/features/git-context-hook-scripts-run-on-exactly-the-changed-files.feature
//
// BUG-589: the script add-virtual-hook --git-context generates runs its
// command on exactly the staged and unstaged files, skips it when there are
// none, and reports a missing jq instead of hiding it. Each test generates
// the script through the dispatcher and runs it directly with the hook
// executor's stdin JSON context (HOOK-018).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};
use tempfile::TempDir;

const RECORDER: &str = "./record-args";
const RECORD: &str = "args.out";
const JQ_MISSING: &str =
    "jq is required to read the changed files of this --git-context hook; install jq";

/// A project with AUTH-001 and a --git-context hook for `command`; returns
/// the generated script's path.
fn generate_script(project: &Path, command: &str) -> PathBuf {
    let spec = project.join("spec");
    fs::create_dir_all(&spec).expect("create spec dir");
    let work_units = json!({
        "version": "0.7.1",
        "meta": { "version": "1.0.0", "lastUpdated": "2026-11-07T00:00:00.000Z" },
        "workUnits": {
            "AUTH-001": {
                "id": "AUTH-001",
                "title": "Login feature",
                "status": "implementing",
                "createdAt": "2026-11-07T00:00:00.000Z",
                "updatedAt": "2026-11-07T00:00:00.000Z"
            }
        },
        "states": {
            "backlog": [], "specifying": [], "testing": [], "implementing": ["AUTH-001"],
            "validating": [], "done": [], "blocked": []
        }
    });
    fs::write(spec.join("work-units.json"), work_units.to_string()).expect("seed work units");
    let result = dispatch_command(DispatchRequest {
        command: "add-virtual-hook".to_string(),
        args_json: json!({
            "workUnitId": "AUTH-001",
            "event": "post-implementing",
            "command": command,
            "blocking": true,
            "gitContext": true
        })
        .to_string(),
        project_root: project.to_path_buf(),
    });
    assert!(result.success, "add-virtual-hook failed: {result:?}");
    let data: Value =
        serde_json::from_str(&fs::read_to_string(spec.join("work-units.json")).unwrap()).unwrap();
    let stored = data["workUnits"]["AUTH-001"]["virtualHooks"][0]["command"]
        .as_str()
        .expect("stored hook command")
        .to_string();
    project.join(stored)
}

/// A project whose hook command is a recorder appending each run's
/// arguments, NUL-separated after a `run` marker, to args.out.
fn recorder_project() -> (TempDir, PathBuf) {
    let project = TempDir::new().expect("tempdir");
    let recorder = project.path().join("record-args");
    fs::write(
        &recorder,
        "#!/bin/bash\n{ printf 'run\\0'; printf '%s\\0' \"$@\"; } >> \"$(dirname \"$0\")/args.out\"\n",
    )
    .expect("write recorder");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&recorder, fs::Permissions::from_mode(0o755)).expect("chmod recorder");
    }
    let script = generate_script(project.path(), RECORDER);
    (project, script)
}

/// Run the script in the project root with the hook context on stdin.
fn run_script(
    project: &Path,
    script: &Path,
    staged: &[&str],
    unstaged: &[&str],
    path: Option<&Path>,
) -> Output {
    let mut command = Command::new(script);
    command
        .current_dir(project)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    let mut child = command.spawn().expect("spawn the generated script");
    let context = json!({
        "workUnitId": "AUTH-001",
        "event": "post-implementing",
        "timestamp": "2026-11-07T00:00:00.000Z",
        "stagedFiles": staged,
        "unstagedFiles": unstaged
    });
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(format!("{context}\n").as_bytes())
        .expect("write context");
    child.wait_with_output().expect("wait for the script")
}

/// Each recorded run's arguments.
fn recorded_runs(project: &Path) -> Vec<Vec<String>> {
    let Ok(text) = fs::read_to_string(project.join(RECORD)) else {
        return Vec::new();
    };
    let mut runs: Vec<Vec<String>> = Vec::new();
    for field in text.split('\0').filter(|field| !field.is_empty()) {
        if field == "run" {
            runs.push(Vec::new());
        } else {
            runs.last_mut()
                .expect("arguments follow a run marker")
                .push(field.to_string());
        }
    }
    runs
}

#[test]
fn no_changed_files_skips_the_command() {
    // @step Given a --git-context hook script generated for a command that records its arguments
    let (project, script) = recorder_project();

    // @step When the script runs with empty stagedFiles and unstagedFiles
    let output = run_script(project.path(), &script, &[], &[], None);

    // @step Then it prints "No changed files to process" and exits 0
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "No changed files to process\n"
    );

    // @step And the command does not run
    assert_eq!(recorded_runs(project.path()), Vec::<Vec<String>>::new());
}

#[test]
fn staged_files_only_reach_the_command_as_separate_arguments() {
    // @step Given a --git-context hook script generated for a command that records its arguments
    let (project, script) = recorder_project();

    // @step When the script runs with stagedFiles "src/auth.ts" and "src/my file.ts" and no unstagedFiles
    let output = run_script(
        project.path(),
        &script,
        &["src/auth.ts", "src/my file.ts"],
        &[],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    // @step Then the command runs once with exactly the arguments "src/auth.ts" and "src/my file.ts"
    assert_eq!(
        recorded_runs(project.path()),
        vec![vec![
            "src/auth.ts".to_string(),
            "src/my file.ts".to_string()
        ]]
    );
}

#[test]
fn unstaged_files_only_reach_the_command() {
    // @step Given a --git-context hook script generated for a command that records its arguments
    let (project, script) = recorder_project();

    // @step When the script runs with no stagedFiles and unstagedFiles "src/hooks/virtual.ts"
    let output = run_script(
        project.path(),
        &script,
        &[],
        &["src/hooks/virtual.ts"],
        None,
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");

    // @step Then the command runs once with exactly the argument "src/hooks/virtual.ts"
    assert_eq!(
        recorded_runs(project.path()),
        vec![vec!["src/hooks/virtual.ts".to_string()]]
    );
}

#[test]
fn a_missing_jq_is_reported_instead_of_hidden() {
    // @step Given a --git-context hook script generated for a command that records its arguments
    let (project, script) = recorder_project();
    let empty_path = TempDir::new().expect("tempdir");

    // @step When the script runs with stagedFiles "src/auth.ts" and no jq on PATH
    let output = run_script(
        project.path(),
        &script,
        &["src/auth.ts"],
        &[],
        Some(empty_path.path()),
    );

    // @step Then it exits 127 with "jq is required to read the changed files of this --git-context hook; install jq" on stderr
    assert_eq!(output.status.code(), Some(127), "{output:?}");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        format!("{JQ_MISSING}\n")
    );

    // @step And the command does not run
    assert_eq!(recorded_runs(project.path()), Vec::<Vec<String>>::new());
}

/// The fenced script after the "Generated Script Example" heading of `doc`.
fn generated_script_example(doc: &str) -> Option<&str> {
    let heading =
        "**Generated Script Example** (`spec/hooks/.virtual/AUTH-001-eslint.sh`):\n\n```bash\n";
    let start = doc.find(heading)? + heading.len();
    let end = doc[start..].find("```\n")? + start;
    Some(&doc[start..end])
}

#[test]
fn the_init_docs_show_the_generated_script() {
    // @step Given the script add-virtual-hook generates for the command "eslint"
    let project = TempDir::new().expect("tempdir");
    let generated =
        fs::read_to_string(generate_script(project.path(), "eslint")).expect("read script");

    // @step When the "Generated Script Example" of every init doc is read
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut docs: Vec<PathBuf> = Vec::new();
    for directory in ["src/commands", "tests/fixtures/init_docs"] {
        for entry in fs::read_dir(crate_root.join(directory)).expect("read docs directory") {
            let path = entry.expect("doc entry").path();
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let is_doc = path.extension().is_some_and(|e| e == "md")
                && (directory != "src/commands" || name.starts_with("init_doc_"));
            if is_doc {
                docs.push(path);
            }
        }
    }
    docs.sort();
    assert_eq!(
        docs.len(),
        9,
        "four init doc sources and five fixtures: {docs:?}"
    );

    // @step Then each example is exactly that script
    for doc in docs {
        let text = fs::read_to_string(&doc).expect("read doc");
        assert_eq!(
            generated_script_example(&text),
            Some(generated.as_str()),
            "{} must show the generated script",
            doc.display()
        );
    }
}
