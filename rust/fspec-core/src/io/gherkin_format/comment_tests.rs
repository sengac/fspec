//! Feature: spec/features/preserve-example-mapping-context-as-comments-in-generated-feature-files.feature
//!
//! EXMAP-002 comment preservation in the formatter, moved out of
//! gherkin_format.rs's inline tests (COV-072: one test file per feature).
//! The gherkin parser drops comments, so the formatter re-extracts them from
//! the raw source and writes each one back before the element it preceded.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use gherkin::GherkinEnv;
use serde_json::json;

use crate::{dispatch_command, DispatchRequest};

fn parse(src: &str) -> Feature {
    Feature::parse(src, GherkinEnv::default()).expect("parse")
}

fn position(out: &str, needle: &str) -> usize {
    out.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} missing from:\n{out}"))
}

/// Runs the `format` command through the dispatcher (the entry the CLI
/// subcommand calls) on `spec/features/<name>` and returns the new content.
fn format_file(root: &std::path::Path, rel: &str) -> String {
    let result = dispatch_command(DispatchRequest {
        command: "format".to_string(),
        args_json: json!({ "file": rel }).to_string(),
        project_root: root.to_path_buf(),
    });
    assert!(result.success, "format failed: {result:?}");
    std::fs::read_to_string(root.join(rel)).expect("read formatted")
}

const EXAMPLE_MAPPED: &str = "@EXMAP-002\nFeature: User Login\n\n  # ========================================\n  # EXAMPLE MAPPING CONTEXT\n  # ========================================\n  #\n  # BUSINESS RULES:\n  #   1. Passwords must be 8+ chars\n  #\n  # EXAMPLES:\n  #   1. User logs in with valid creds\n  #\n  # ========================================\n  Background: User Story\n    As a registered user\n    I want to log in securely\n    So that I can reach my account\n\n  Scenario: User logs in with valid credentials\n    Given a registered user\n    When the user logs in\n    Then the dashboard opens\n";

#[test]
fn formatter_preserves_comments_through_format_cycles() {
    // @step Given I have a feature file with example mapping comments
    let tmp = tempfile::tempdir().expect("tempdir");
    let rel = "spec/features/user-login.feature";
    std::fs::create_dir_all(tmp.path().join("spec/features")).expect("mkdir");
    std::fs::write(tmp.path().join(rel), EXAMPLE_MAPPED).expect("write feature");

    // @step And the file contains "# BUSINESS RULES:" comment block
    assert!(EXAMPLE_MAPPED.contains("  # BUSINESS RULES:\n  #   1. Passwords must be 8+ chars\n"));

    // @step When I run "fspec format" on the file
    let once = format_file(tmp.path(), rel);
    let twice = format_file(tmp.path(), rel);

    // @step Then the formatted file should still contain the comment block
    // @step And the comments should be in the same location
    // @step And no comments should be lost
    assert_eq!(once, EXAMPLE_MAPPED, "a canonical file is left as it is");
    assert_eq!(twice, once, "a second format cycle changes nothing");

    let src = "# Header comment\nFeature: One\n\n# Before scenario\n  Scenario: A\n    # Before step\n    Given x\n";
    let first = format_feature(&parse(src), src);
    let second = format_feature(&parse(&first), &first);
    assert_eq!(first, second, "comment preservation is idempotent");
    for comment in ["# Header comment", "# Before scenario", "# Before step"] {
        assert_eq!(second.matches(comment).count(), 1, "{comment}:\n{second}");
    }
}

#[test]
fn formatter_puts_comments_back_at_their_original_positions() {
    // @step Given I have a feature file with comments before the Feature keyword, a Scenario and a step
    let src = "# This is a comment\nFeature: One\n\n# Comment before scenario\n  Scenario: A\n    # Indented comment\n    Given x\n# Comment before When\n    When y\n    Then z\n";

    // @step When the formatter formats it
    let out = format_feature(&parse(src), src);

    // @step Then the formatted output should include all comments
    for comment in [
        "# This is a comment",
        "# Comment before scenario",
        "# Indented comment",
        "# Comment before When",
    ] {
        assert_eq!(out.matches(comment).count(), 1, "{comment}:\n{out}");
    }

    // @step And an indented comment keeps its indentation
    assert!(
        out.contains("\n    # Indented comment\n    Given x\n"),
        "{out}"
    );

    // @step And comments should appear before the elements they precede
    assert!(position(&out, "# This is a comment") < position(&out, "Feature: One"));
    assert!(position(&out, "Feature: One") < position(&out, "# Comment before scenario"));
    assert!(position(&out, "# Comment before scenario") < position(&out, "Scenario: A"));
    assert!(position(&out, "Scenario: A") < position(&out, "# Indented comment"));
    assert!(position(&out, "Given x") < position(&out, "# Comment before When"));
    assert!(position(&out, "# Comment before When") < position(&out, "When y"));

    let mapped = "Feature: Example Mapping\n\n# EXAMPLE MAPPING CONTEXT\n# Rules:\n#   1. Password must be 8+ characters\n# Examples:\n#   1. User enters valid credentials\n\n  Scenario: Login\n    Given x\n";
    let out = format_feature(&parse(mapped), mapped);
    let lines = [
        "# EXAMPLE MAPPING CONTEXT",
        "# Rules:",
        "#   1. Password must be 8+ characters",
        "# Examples:",
        "#   1. User enters valid credentials",
        "Scenario: Login",
    ];
    for pair in lines.windows(2) {
        assert!(
            position(&out, pair[0]) < position(&out, pair[1]),
            "{pair:?}:\n{out}"
        );
    }
}
