#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;

#[test]
fn args_parse_defaults() {
    let a: GenerateCoverageArgs = serde_json::from_str("{}").unwrap();
    assert!(!a.dry_run);
}

#[test]
fn args_parse_dry_run() {
    let a: GenerateCoverageArgs = serde_json::from_str(r#"{"dryRun":true}"#).unwrap();
    assert!(a.dry_run);
}

#[test]
fn render_success_joins_nonzero_parts() {
    let out = render_success(2, 1, 3, 0, &[]);
    assert!(out.starts_with("✓ Created 2, Updated 1, Skipped 3\n"));
    assert!(out.contains("<system-reminder>"));
}

#[test]
fn render_success_no_files_needed() {
    let out = render_success(0, 0, 0, 0, &[]);
    assert!(out.starts_with("No coverage files needed\n"));
}

#[test]
fn render_dry_run_lists_files() {
    let out = render_dry_run(1, 0, 0, &["x.feature.coverage".to_string()], &[], &[]);
    assert!(out.contains("Would create 1 coverage files (DRY RUN)"));
    assert!(out.contains("x.feature.coverage"));
}
