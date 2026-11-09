//! BUG-495 — delete-scenario help lists only accepted options.
//!
//! Feature: spec/features/delete-scenario-help-lists-only-accepted-options.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

use common::fspec_bin;

fn fspec(cwd: &Path, args: &[&str]) -> (bool, String) {
    let output = Command::new(fspec_bin())
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("spawn fspec");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.success(), text)
}

#[test]
fn the_help_does_not_offer_a_force_option() {
    let root = tempfile::tempdir().expect("tempdir");
    // @step When I run fspec delete-scenario --help
    let (ok, text) = fspec(root.path(), &["delete-scenario", "--help"]);
    assert!(ok, "{text}");
    // @step Then the output does not mention --force
    assert!(!text.contains("--force"), "{text}");
}

#[test]
fn the_force_option_stays_an_unknown_argument() {
    // @step Given a feature file with scenario "Old"
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path().join("spec/features/x.feature");
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, "Feature: X\n\n  Scenario: Old\n    Given a\n").expect("write");
    // @step When I run fspec delete-scenario with the feature, "Old" and --force
    let (ok, text) = fspec(root.path(), &["delete-scenario", "x", "Old", "--force"]);
    // @step Then the command fails reporting --force as an unknown argument
    assert!(!ok, "{text}");
    assert!(text.contains("--force"), "{text}");
    // @step And scenario "Old" is still in the feature file
    assert!(fs::read_to_string(&path)
        .expect("read")
        .contains("Scenario: Old"));
}
