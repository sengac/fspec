//! BUG-479 — feature files match the formatter.
//!
//! fspec audit reports a feature file whose bytes the formatter would change.
//! The generators now write the canonical layout, the formatter keeps
//! content ('#' lines inside a description, trailing comments, an unnamed
//! Background), and every committed feature file is formatted.
//!
//! Formatting only ever runs in scratch projects here; `fspec format` is
//! never run on the repository (BUG-152).
//!
//! Feature: spec/features/feature-files-match-the-formatter.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use codelet_fspec_core::io::gherkin::parse_feature_lenient;
use codelet_fspec_core::io::gherkin_format::format_feature;
use codelet_fspec_core::{dispatch_command, DispatchRequest};
use serde_json::{json, Value};

fn dispatch(project_root: &Path, command: &str, args: Value) -> codelet_fspec_core::DispatchResult {
    dispatch_command(DispatchRequest {
        command: command.to_string(),
        args_json: args.to_string(),
        project_root: project_root.to_path_buf(),
    })
}

/// fspec audit's format verdict ("PASS" without format problems, else
/// "FAIL") and its format problem lines.
fn format_check(project_root: &Path) -> (String, Vec<String>) {
    let result = dispatch(project_root, "audit", json!({"area": ["format"]}));
    let data: Value = serde_json::from_str(&result.data).expect("audit data is JSON");
    let problems: Vec<String> = data["problems"]
        .as_array()
        .map(|problems| {
            problems
                .iter()
                .filter(|p| p["code"] == "needs-formatting")
                .map(|p| format!("{} needs formatting", p["location"].as_str().unwrap_or("")))
                .collect()
        })
        .unwrap_or_default();
    let status = if problems.is_empty() { "PASS" } else { "FAIL" };
    (status.to_string(), problems)
}

/// Write `content` as spec/features/sample.feature in a scratch project, run
/// fspec audit there, then format the scratch copy. Returns the check verdict
/// and the file after formatting.
fn check_then_format(content: &str) -> (String, String) {
    let root = tempfile::tempdir().expect("tempdir");
    let file = root.path().join("spec/features/sample.feature");
    fs::create_dir_all(file.parent().expect("features dir")).expect("mkdir features");
    fs::write(&file, content).expect("write feature");
    let (status, _) = format_check(root.path());
    let formatted = dispatch(
        root.path(),
        "format",
        json!({"file": "spec/features/sample.feature"}),
    );
    assert!(formatted.success, "format failed: {formatted:?}");
    (status, fs::read_to_string(&file).expect("read formatted"))
}

#[test]
fn a_description_line_starting_with_a_hash_stays_in_the_description() {
    // @step Given a feature whose description docstring contains the line "#[path] fixtures."
    let content = "@cli\nFeature: Sample\n  \"\"\"\n  BUG-424 removed the dead env! redirect for its\n  #[path] fixtures.\n  \"\"\"\n\n  Scenario: One\n    Given a step\n";

    // @step When fspec audit runs and the file is formatted in a scratch project
    let (status, formatted) = check_then_format(content);

    // @step Then the audit reports the file as formatted
    assert_eq!(status, "PASS");

    // @step And formatting leaves the file unchanged
    assert_eq!(formatted, content);
}

#[test]
fn comments_after_the_last_scenario_are_kept() {
    // @step Given a feature that ends with a comment block after its last scenario
    let content = "@cli\nFeature: Sample\n\n  Scenario: One\n    Given a step\n\n  # BUG-444 superseded \"Old scenario\":\n  # a broken coverage file is reported, not skipped.\n";

    // @step When fspec audit runs and the file is formatted in a scratch project
    let (status, formatted) = check_then_format(content);

    // @step Then the audit reports the file as formatted
    assert_eq!(status, "PASS");

    // @step And formatting leaves the file unchanged
    assert_eq!(formatted, content);
}

#[test]
fn a_background_without_a_name_has_no_trailing_space() {
    // @step Given a feature with a line "  Background:" and no Background name
    let content = "@cli\nFeature: Sample\n\n  Background:\n    Given a shared step\n\n  Scenario: One\n    Given a step\n";

    // @step When fspec audit runs and the file is formatted in a scratch project
    let (status, formatted) = check_then_format(content);

    // @step Then the audit reports the file as formatted
    assert_eq!(status, "PASS");

    // @step And formatting leaves the file unchanged
    assert_eq!(formatted, content);
}

#[test]
fn create_feature_writes_a_formatted_feature_file() {
    // @step Given an empty project
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("spec/features")).expect("mkdir features");

    // @step When fspec create-feature "Sample Thing" runs
    let created = dispatch(
        root.path(),
        "create-feature",
        json!({"name": "Sample Thing"}),
    );
    assert!(created.success, "create-feature failed: {created:?}");
    assert!(root
        .path()
        .join("spec/features/sample-thing.feature")
        .exists());

    // @step Then fspec audit reports spec/features/sample-thing.feature as formatted
    let (status, errors) = format_check(root.path());
    assert_eq!(status, "PASS", "{errors:?}");
}

#[test]
fn generate_scenarios_writes_a_formatted_feature_file() {
    // @step Given a work unit with a user story, rules, examples and an answered question
    let root = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(root.path().join("spec/features")).expect("mkdir features");
    let board = json!({
        "version": "0.7.1",
        "workUnits": {"WU-1": {
            "id": "WU-1", "title": "User Authentication", "type": "story", "status": "specifying",
            "createdAt": "2026-06-01T00:00:00.000Z", "updatedAt": "2026-06-01T00:00:00.000Z",
            "userStory": {"role": "registered user", "action": "log in securely", "benefit": "access my account"},
            "rules": [{"id": 0, "text": "Password must be 8+ characters", "deleted": false}],
            "examples": [{"id": 0, "text": "User logs in with a valid password", "deleted": false}],
            "questions": [{"id": 0, "text": "@human: Support OAuth?", "deleted": false,
                "selected": true, "answer": "Not yet"}]
        }},
        "states": {"backlog": [], "specifying": ["WU-1"], "testing": [], "implementing": [],
            "validating": [], "done": [], "blocked": []}
    });
    fs::write(
        root.path().join("spec/work-units.json"),
        serde_json::to_string_pretty(&board).expect("serialize board"),
    )
    .expect("write work-units.json");

    // @step When fspec generate-scenarios writes its feature file
    let generated = dispatch(
        root.path(),
        "generate-scenarios",
        json!({"workUnitId": "WU-1"}),
    );
    assert!(
        generated.success,
        "generate-scenarios failed: {generated:?}"
    );
    assert!(root
        .path()
        .join("spec/features/user-authentication.feature")
        .exists());

    // @step Then fspec audit reports that feature file as formatted
    let (status, errors) = format_check(root.path());
    assert_eq!(status, "PASS", "{errors:?}");
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repo root")
}

fn git(args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root())
        .output()
        .expect("run git");
    assert!(out.status.success(), "git {args:?} failed: {out:?}");
    String::from_utf8(out.stdout).expect("git output is UTF-8")
}

#[test]
fn every_committed_feature_file_is_formatted() {
    // @step Given the repository's committed feature files
    // (read from HEAD, so other cards' uncommitted edits do not count)
    let listing = git(&["ls-tree", "-r", "HEAD", "--", "spec/features"]);
    let blobs: Vec<(String, String)> = listing
        .lines()
        .filter_map(|line| {
            let (meta, path) = line.split_once('\t')?;
            let id = meta.split_whitespace().nth(2)?;
            path.ends_with(".feature")
                .then(|| (id.to_string(), path.to_string()))
        })
        .collect();
    assert!(blobs.len() > 100, "expected the committed feature files");
    let mut cat = Command::new("git")
        .args(["cat-file", "--batch"])
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn git cat-file");
    let mut stdin = cat.stdin.take().expect("cat-file stdin");
    let ids: String = blobs.iter().map(|(id, _)| format!("{id}\n")).collect();
    let writer = std::thread::spawn(move || stdin.write_all(ids.as_bytes()));
    let out = cat.wait_with_output().expect("git cat-file output");
    writer.join().expect("writer thread").expect("write ids");
    let mut rest = &out.stdout[..];

    // @step When fspec audit runs on them
    let mut unformatted = Vec::new();
    for (_, path) in &blobs {
        let header_end = rest.iter().position(|b| *b == b'\n').expect("header");
        let header = String::from_utf8_lossy(&rest[..header_end]).into_owned();
        let size: usize = header
            .split_whitespace()
            .nth(2)
            .and_then(|s| s.parse().ok())
            .expect("blob size");
        let body = &rest[header_end + 1..header_end + 1 + size];
        rest = &rest[header_end + 1 + size + 1..];
        let content = String::from_utf8_lossy(body).into_owned();
        // Unparseable files are the gherkin area's concern, as in fspec audit.
        if let Ok(feature) = parse_feature_lenient(&content) {
            if format_feature(&feature, &content) != content {
                unformatted.push(path.clone());
            }
        }
    }

    // @step Then none of them is reported as needing formatting
    assert!(
        unformatted.is_empty(),
        "committed feature files needing formatting:\n{}",
        unformatted.join("\n")
    );
}
