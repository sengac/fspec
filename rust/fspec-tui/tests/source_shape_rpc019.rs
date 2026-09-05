//! RPC-019 — Source-shape regression tests.
//!
//! Feature: spec/features/rpc019-source-shape.feature

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn workspace_codelet_dir() -> std::path::PathBuf {
    common::workspace_root()
}

fn fspec_tui_src() -> std::path::PathBuf {
    workspace_codelet_dir().join("fspec-tui").join("src")
}

fn read_raw(path: &std::path::Path) -> String {
    common::read_to_string_or_panic(path)
}

fn count_lines_path(path: &std::path::Path) -> usize {
    read_raw(path).lines().count()
}

/// Scenario: codelet workspace declares tui-textarea as a dep
#[test]
fn codelet_workspace_declares_tui_textarea_as_a_dep() {
    // @step Given rust/Cargo.toml after RPC-019 lands
    let body = read_raw(&workspace_codelet_dir().join("Cargo.toml"));
    // @step Then the file contains the substring "tui-textarea ="
    assert!(
        body.contains("tui-textarea ="),
        "rust/Cargo.toml must declare tui-textarea as a workspace dep"
    );
}

/// Scenario: codelet-fspec-tui declares tui-textarea as a dep
#[test]
fn codelet_fspec_tui_declares_tui_textarea_as_a_dep() {
    // @step Given rust/fspec-tui/Cargo.toml after RPC-019 lands
    let body = read_raw(&workspace_codelet_dir().join("fspec-tui").join("Cargo.toml"));
    // @step Then the file contains the substring "tui-textarea"
    assert!(
        body.contains("tui-textarea"),
        "rust/fspec-tui/Cargo.toml must depend on tui-textarea"
    );
}

/// Scenario: New MultiLineInput module exists with the documented surface
#[test]
fn new_multi_line_input_module_exists_with_the_documented_surface() {
    // @step Given the rust/fspec-tui crate after RPC-019 lands
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("multiline_input.rs");
    // @step Then the file rust/fspec-tui/src/views/agent/multiline_input.rs exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And the file contains the substring "pub struct MultiLineInput"
    assert!(body.contains("pub struct MultiLineInput"));
    // @step And the file contains the substring "pub enum InputEventOutcome"
    assert!(body.contains("pub enum InputEventOutcome"));
    // @step And the file contains the substring "Submitted(String)"
    assert!(body.contains("Submitted(String)"));
    // @step And the file contains the substring "Continued"
    assert!(body.contains("Continued"));
    // @step And the file contains the substring "Ignored"
    assert!(body.contains("Ignored"));
}

/// Scenario: New ScrollbackList module exists with the documented surface
#[test]
fn new_scrollback_list_module_exists_with_the_documented_surface() {
    // @step Given the rust/fspec-tui crate after RPC-019 lands
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("scrollback.rs");
    // @step Then the file rust/fspec-tui/src/views/agent/scrollback.rs exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And the file contains the substring "pub struct ScrollbackList"
    assert!(body.contains("pub struct ScrollbackList"));
    // @step And the file contains the substring "pub struct ScrollState"
    assert!(body.contains("pub struct ScrollState"));
    // @step And the file contains the substring "pub fn push"
    assert!(body.contains("pub fn push"));
    // @step And the file contains the substring "stick_to_bottom"
    assert!(body.contains("stick_to_bottom"));
}

/// Scenario: AgentView orchestrator now wires the new widgets
#[test]
fn agent_view_orchestrator_now_wires_the_new_widgets() {
    // @step Given rust/fspec-tui/src/views/agent.rs after RPC-019 lands
    let body = read_raw(&fspec_tui_src().join("views").join("agent.rs"));
    // @step Then the file contains the substring "MultiLineInput"
    assert!(body.contains("MultiLineInput"));
    // @step And the file contains the substring "ScrollbackList"
    assert!(body.contains("ScrollbackList"));
    // @step And the file does NOT contain the substring "tui_input::Input"
    assert!(!body.contains("tui_input::Input"));
}

/// Scenario: Action enum gains four navigation variants
#[test]
fn action_enum_gains_four_navigation_variants() {
    // @step Given rust/fspec-tui/src/components/mod.rs after RPC-019 lands
    let body = read_raw(&fspec_tui_src().join("components").join("mod.rs"));
    // @step Then the file contains the substring "HistoryPrev"
    assert!(body.contains("HistoryPrev"));
    // @step And the file contains the substring "HistoryNext"
    assert!(body.contains("HistoryNext"));
    // @step And the file contains the substring "SessionPrev"
    assert!(body.contains("SessionPrev"));
    // @step And the file contains the substring "SessionNext"
    assert!(body.contains("SessionNext"));
}

/// Scenario: Every file under views/agent/ and views/agent.rs stays under 300 lines
#[test]
fn every_file_under_views_agent_stays_under_300_lines() {
    // @step Given the directory rust/fspec-tui/src/views/agent/ plus the views/agent.rs orchestrator
    let agent_dir = fspec_tui_src().join("views").join("agent");
    let orchestrator = fspec_tui_src().join("views").join("agent.rs");
    // @step When a test counts the line-count of every .rs file
    let mut offenders: Vec<(std::path::PathBuf, usize)> = Vec::new();
    for entry in std::fs::read_dir(&agent_dir).expect("agent dir") {
        let entry = entry.expect("entry");
        let p = entry.path();
        if p.extension().and_then(|s| s.to_str()) == Some("rs") {
            let n = count_lines_path(&p);
            if n >= 300 {
                offenders.push((p.clone(), n));
            }
        }
    }
    // @step Then every file in views/agent/ has fewer than 300 lines
    assert!(offenders.is_empty(), "files >= 300 lines: {offenders:?}");
    // @step And the orchestrator file views/agent.rs has fewer than 300 lines
    let n = count_lines_path(&orchestrator);
    assert!(n < 300, "views/agent.rs has {n} lines, must be < 300");
}

/// Scenario: Views do not directly import codelet_core / napi / tarpc / tokio_tungstenite
#[test]
fn views_do_not_directly_import_forbidden_crates() {
    // @step Given the directory rust/fspec-tui/src/views/ (including views/agent/) after RPC-019 lands
    let views_dir = fspec_tui_src().join("views");

    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).expect("dir") {
            let e = e.expect("entry");
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    walk(&views_dir, &mut files);

    // @step When a test scans every *.rs file
    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let body = read_raw(path);
        // @step Then no file imports `codelet_core::` or `codelet_napi::` or `tarpc::` or `tokio_tungstenite::`
        for needle in [
            "codelet_core::",
            "codelet_napi::",
            "tarpc::",
            "tokio_tungstenite::",
        ] {
            if body.contains(needle) {
                violations.push(format!("{} imports {}", path.display(), needle));
            }
        }
        // @step And no file constructs `tokio::runtime::Builder` or `Runtime::new()`
        if body.contains("tokio::runtime::Builder") || body.contains("Runtime::new(") {
            violations.push(format!("{} constructs a tokio runtime", path.display()));
        }
    }
    assert!(violations.is_empty(), "violations: {violations:?}");
}

