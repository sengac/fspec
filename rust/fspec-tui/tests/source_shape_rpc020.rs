//! RPC-020 — Source-shape regression tests.
//!
//! Feature: spec/features/rpc020-source-shape.feature

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

/// Scenario: codelet_core::file_search helper module exists with the documented surface
#[test]
fn codelet_core_file_search_helper_exists_with_documented_surface() {
    // @step Given rust/core/src/file_search.rs after RPC-020 lands
    let path = workspace_codelet_dir()
        .join("core")
        .join("src")
        .join("file_search.rs");
    // @step Then the file exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And the file contains the substring "pub fn search"
    assert!(body.contains("pub fn search"));
}

/// Scenario: codelet_core::lib re-exports the new file_search module
#[test]
fn codelet_core_lib_re_exports_file_search() {
    // @step Given rust/core/src/lib.rs after RPC-020 lands
    let body = read_raw(
        &workspace_codelet_dir()
            .join("core")
            .join("src")
            .join("lib.rs"),
    );
    // @step Then the file contains the substring "pub mod file_search"
    assert!(body.contains("pub mod file_search"));
}

/// Scenario: FspecService trait gains the search_files RPC method
#[test]
fn fspec_service_trait_gains_search_files_rpc_method() {
    // @step Given rust/rpc/src/lib.rs after RPC-020 lands
    let body = read_raw(
        &workspace_codelet_dir()
            .join("rpc")
            .join("src")
            .join("lib.rs"),
    );
    // @step Then the file contains the substring "async fn search_files"
    assert!(body.contains("async fn search_files"));
    // @step And the file contains the substring "prefix: String"
    assert!(body.contains("prefix: String"));
    // @step And the file contains the substring "limit: u32"
    assert!(body.contains("limit: u32"));
}

/// Scenario: FspecBackend trait declares the search_files method
#[test]
fn fspec_backend_trait_declares_search_files() {
    // @step Given rust/fspec-tui/src/transport/mod.rs after RPC-020 lands
    let body = read_raw(&fspec_tui_src().join("transport").join("mod.rs"));
    // @step Then the file contains the substring "async fn search_files"
    assert!(body.contains("async fn search_files"));
}

/// Scenario: Both transports implement the search_files FspecBackend method
#[test]
fn both_transports_implement_search_files() {
    // @step Given the rust/fspec-tui crate after RPC-020 lands
    let embedded = read_raw(&fspec_tui_src().join("transport").join("embedded.rs"));
    let websocket = read_raw(&fspec_tui_src().join("transport").join("websocket.rs"));
    // @step Then rust/fspec-tui/src/transport/embedded.rs contains the substring "async fn search_files"
    assert!(embedded.contains("async fn search_files"));
    // @step And rust/fspec-tui/src/transport/websocket.rs contains the substring "async fn search_files"
    assert!(websocket.contains("async fn search_files"));
}

/// Scenario: New slash_commands module exists with the documented surface
#[test]
fn slash_commands_module_exists_with_documented_surface() {
    // @step Given the rust/fspec-tui crate after RPC-020 lands
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("slash_commands.rs");
    // @step Then the file rust/fspec-tui/src/views/agent/slash_commands.rs exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And the file contains the substring "pub struct SlashCommand"
    assert!(body.contains("pub struct SlashCommand"));
    // @step And the file contains the substring "pub enum SlashCommandAction"
    assert!(body.contains("pub enum SlashCommandAction"));
    // @step And the file contains the substring "pub const SLASH_COMMANDS"
    assert!(body.contains("pub const SLASH_COMMANDS"));
    // @step And the file contains the substring "pub fn filter_commands"
    assert!(body.contains("pub fn filter_commands"));
}

/// Scenario: New SlashCommandPopup module exists with the documented surface
#[test]
fn slash_command_popup_module_exists_with_documented_surface() {
    // @step Given the rust/fspec-tui crate after RPC-020 lands
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("slash_command_popup.rs");
    // @step Then the file rust/fspec-tui/src/views/agent/slash_command_popup.rs exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And the file contains the substring "pub struct SlashCommandPopup"
    assert!(body.contains("pub struct SlashCommandPopup"));
}

/// Scenario: New FileSearchPopup module exists with the documented surface
#[test]
fn file_search_popup_module_exists_with_documented_surface() {
    // @step Given the rust/fspec-tui crate after RPC-020 lands
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("file_search_popup.rs");
    // @step Then the file rust/fspec-tui/src/views/agent/file_search_popup.rs exists
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And the file contains the substring "pub struct FileSearchPopup"
    assert!(body.contains("pub struct FileSearchPopup"));
}

/// Scenario: AgentView orchestrator owns the new popup fields
#[test]
fn agent_view_orchestrator_owns_the_new_popup_fields() {
    // @step Given rust/fspec-tui/src/views/agent.rs after RPC-020 lands
    let body = read_raw(&fspec_tui_src().join("views").join("agent.rs"));
    // @step Then the file contains the substring "slash_popup"
    assert!(body.contains("slash_popup"));
    // @step And the file contains the substring "file_popup"
    assert!(body.contains("file_popup"));
    // @step And the file contains the substring "SlashCommandPopup"
    assert!(body.contains("SlashCommandPopup"));
    // @step And the file contains the substring "FileSearchPopup"
    assert!(body.contains("FileSearchPopup"));
}

/// Scenario: Action enum gains three new variants
#[test]
fn action_enum_gains_three_new_variants() {
    // @step Given rust/fspec-tui/src/components/mod.rs after RPC-020 lands
    let body = read_raw(&fspec_tui_src().join("components").join("mod.rs"));
    // @step Then the file contains the substring "SlashCommandSelected"
    assert!(body.contains("SlashCommandSelected"));
    // @step And the file contains the substring "SearchFiles"
    assert!(body.contains("SearchFiles"));
    // @step And the file contains the substring "FileSearchResults"
    assert!(body.contains("FileSearchResults"));
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
    // @step Given the directory rust/fspec-tui/src/views/ (including views/agent/) after RPC-020 lands
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

/// Scenario: SlashCommandAction enum contains no Providers variant
#[test]
fn slash_command_action_enum_has_no_providers_variant() {
    // @step Given rust/fspec-tui/src/views/agent/slash_commands.rs after the 2026-06-01 revision
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("slash_commands.rs");
    // @step When the source is parsed for SlashCommandAction variants
    let source = read_raw(&path);

    // @step Then the enum contains "Provider" exactly once
    // (We check that "Provider," — i.e. the singular variant declaration
    //  — exists, and the plural "Providers," does NOT exist.)
    assert!(
        source.contains("Provider,"),
        "slash_commands.rs must declare SlashCommandAction::Provider"
    );
    // @step And the enum does NOT contain a "Providers" variant
    assert!(
        !source.contains("Providers,"),
        "slash_commands.rs must NOT declare SlashCommandAction::Providers (no /providers in TS reference)"
    );
    // @step And the SLASH_COMMANDS const contains exactly one entry whose action is SlashCommandAction::Provider
    // Count `action: SlashCommandAction::Provider,` (registry entry) — must be exactly 1
    let registry_count = source
        .matches("action: SlashCommandAction::Provider,")
        .count();
    assert_eq!(
        registry_count, 1,
        "SLASH_COMMANDS must contain exactly one entry whose action is SlashCommandAction::Provider; got {registry_count}"
    );
    // @step And no entry in SLASH_COMMANDS has the name "providers"
    assert!(
        !source.contains("SlashCommandAction::Providers"),
        "slash_commands.rs must NOT reference SlashCommandAction::Providers anywhere"
    );
    assert!(
        !source.contains("\"providers\""),
        "slash_commands.rs must NOT contain the string literal \"providers\""
    );
}

/// Scenario: dispatch_slash_commands.rs has no Providers arm
#[test]
fn dispatch_slash_commands_has_no_providers_arm() {
    // @step Given rust/fspec-tui/src/app/dispatch_slash_commands.rs after the 2026-06-01 revision
    let path = fspec_tui_src()
        .join("app")
        .join("dispatch_slash_commands.rs");
    // @step When the file is read
    let source = read_raw(&path);

    // @step Then it contains exactly one arm matching "SlashCommandAction::Provider =>"
    assert!(
        source.contains("SlashCommandAction::Provider =>"),
        "dispatch_slash_commands.rs must have a single `SlashCommandAction::Provider =>` arm"
    );
    // @step And it does NOT contain "SlashCommandAction::Providers"
    assert!(
        !source.contains("SlashCommandAction::Providers"),
        "dispatch_slash_commands.rs must NOT reference SlashCommandAction::Providers"
    );
    // @step And it does NOT contain "| SlashCommandAction::Providers"
    assert!(
        !source.contains("| SlashCommandAction::Providers"),
        "dispatch_slash_commands.rs must NOT have a `| SlashCommandAction::Providers` arm"
    );
}
