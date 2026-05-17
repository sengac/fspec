//! RPC-026 — Source-shape regression tests.
//!
//! Feature: spec/features/rpc026-source-shape.feature

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

/// Scenario: The resume picker widget lives under views/agent with the right file shape
#[test]
fn resume_picker_widget_has_documented_surface() {
    // @step Given the codelet/fspec-tui crate
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("resume_picker.rs");
    // @step Then a file exists at codelet/fspec-tui/src/views/agent/resume_picker.rs
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And that file is under 300 lines
    assert!(count_lines_path(&path) < 300);
    // @step And the file declares "pub struct ResumePicker"
    assert!(body.contains("pub struct ResumePicker"));
    // @step And the file declares "pub enum ResumePickerOutcome"
    assert!(body.contains("pub enum ResumePickerOutcome"));
    // @step And the file declares "pub fn set_sessions"
    assert!(body.contains("pub fn set_sessions"));
    // @step And the file declares "pub fn handle_key"
    assert!(body.contains("pub fn handle_key"));
    // @step And the file declares "pub fn render"
    assert!(body.contains("pub fn render"));
}

/// Scenario: The search palette widget lives under views/agent with the right file shape
#[test]
fn search_palette_widget_has_documented_surface() {
    // @step Given the codelet/fspec-tui crate
    let path = fspec_tui_src()
        .join("views")
        .join("agent")
        .join("search_palette.rs");
    // @step Then a file exists at codelet/fspec-tui/src/views/agent/search_palette.rs
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And that file is under 300 lines
    assert!(count_lines_path(&path) < 300);
    // @step And the file declares "pub struct SearchPalette"
    assert!(body.contains("pub struct SearchPalette"));
    // @step And the file declares "pub enum SearchPaletteOutcome"
    assert!(body.contains("pub enum SearchPaletteOutcome"));
    // @step And the file declares "pub fn set_query"
    assert!(body.contains("pub fn set_query"));
    // @step And the file declares "pub fn set_matches"
    assert!(body.contains("pub fn set_matches"));
    // @step And the file declares "pub fn handle_key"
    assert!(body.contains("pub fn handle_key"));
    // @step And the file declares "pub fn render"
    assert!(body.contains("pub fn render"));
}

/// Scenario: The new dispatch helpers live in their own dispatch_rpc026.rs file under 300 lines
#[test]
fn dispatch_rpc026_module_has_documented_surface() {
    // @step Given the codelet/fspec-tui crate
    let path = fspec_tui_src().join("app").join("dispatch_rpc026.rs");
    // @step Then a file exists at codelet/fspec-tui/src/app/dispatch_rpc026.rs
    assert!(path.is_file(), "{} must exist", path.display());
    let body = read_raw(&path);
    // @step And that file is under 300 lines
    assert!(count_lines_path(&path) < 300);
    // @step And the file declares "fn handle_open_resume_picker"
    assert!(body.contains("fn handle_open_resume_picker"));
    // @step And the file declares "fn handle_session_list_loaded"
    assert!(body.contains("fn handle_session_list_loaded"));
    // @step And the file declares "fn handle_attach_to_session"
    assert!(body.contains("fn handle_attach_to_session"));
    // @step And the file declares "fn handle_open_search_palette"
    assert!(body.contains("fn handle_open_search_palette"));
    // @step And the file declares "fn handle_search_history"
    assert!(body.contains("fn handle_search_history"));
    // @step And the file declares "fn handle_history_search_results"
    assert!(body.contains("fn handle_history_search_results"));
    // @step And the file declares "fn handle_insert_into_input"
    assert!(body.contains("fn handle_insert_into_input"));
}

/// Scenario: AgentView stays under 300 LoC after the two new Option fields land
#[test]
fn agent_view_orchestrator_owns_the_new_popup_fields() {
    // @step Given codelet/fspec-tui/src/views/agent.rs after RPC-026 lands
    let path = fspec_tui_src().join("views").join("agent.rs");
    let body = read_raw(&path);
    // @step Then the file is under 300 lines
    assert!(count_lines_path(&path) < 300);
    // @step And the file declares the "resume_popup" field
    assert!(body.contains("resume_popup"));
    // @step And the file declares the "search_popup" field
    assert!(body.contains("search_popup"));
}

/// Scenario: App dispatch orchestrator stays under 300 LoC after the five new match arms land
#[test]
fn app_dispatch_orchestrator_routes_new_actions() {
    // @step Given codelet/fspec-tui/src/app/dispatch.rs after RPC-026 lands
    let path = fspec_tui_src().join("app").join("dispatch.rs");
    let body = read_raw(&path);
    // @step Then the file is under 300 lines
    assert!(count_lines_path(&path) < 300);
    // @step And the file routes "Action::OpenResumePicker" through handle_open_resume_picker
    assert!(body.contains("Action::OpenResumePicker"));
    assert!(body.contains("handle_open_resume_picker"));
    // @step And the file routes "Action::SessionListLoaded" through handle_session_list_loaded
    assert!(body.contains("Action::SessionListLoaded"));
    assert!(body.contains("handle_session_list_loaded"));
    // @step And the file routes "Action::AttachToSession" through handle_attach_to_session
    assert!(body.contains("Action::AttachToSession"));
    assert!(body.contains("handle_attach_to_session"));
    // @step And the file routes "Action::OpenSearchPalette" through handle_open_search_palette
    assert!(body.contains("Action::OpenSearchPalette"));
    assert!(body.contains("handle_open_search_palette"));
    // @step And the file routes "Action::SearchHistory" through handle_search_history
    assert!(body.contains("Action::SearchHistory"));
    assert!(body.contains("handle_search_history"));
    // @step And the file routes "Action::HistorySearchResults" through handle_history_search_results
    assert!(body.contains("Action::HistorySearchResults"));
    assert!(body.contains("handle_history_search_results"));
    // @step And the file routes "Action::InsertIntoInput" through handle_insert_into_input
    assert!(body.contains("Action::InsertIntoInput"));
    assert!(body.contains("handle_insert_into_input"));
}

/// Scenario: handle_slash_command in dispatch_rpc020.rs is amended to dispatch the new actions for Resume/Search
#[test]
fn handle_slash_command_dispatches_open_resume_and_open_search() {
    // @step Given codelet/fspec-tui/src/app/dispatch_rpc020.rs after RPC-026 lands
    let path = fspec_tui_src().join("app").join("dispatch_rpc020.rs");
    let body = read_raw(&path);
    // @step Then the file is under 300 lines
    assert!(count_lines_path(&path) < 300);
    // @step And the file routes "SlashCommandAction::Resume" through Action::OpenResumePicker
    assert!(body.contains("SlashCommandAction::Resume"));
    assert!(body.contains("Action::OpenResumePicker"));
    // @step And the file routes "SlashCommandAction::Search" through Action::OpenSearchPalette
    assert!(body.contains("SlashCommandAction::Search"));
    assert!(body.contains("Action::OpenSearchPalette"));
}

/// Scenario: Action enum gains the seven new variants required by RPC-026
#[test]
fn action_enum_gains_seven_new_variants() {
    // @step Given codelet/fspec-tui/src/components/mod.rs after RPC-026 lands
    let body = read_raw(&fspec_tui_src().join("components").join("mod.rs"));
    // @step Then the Action enum declares the "OpenResumePicker" variant
    assert!(body.contains("OpenResumePicker"));
    // @step And the Action enum declares the "OpenSearchPalette" variant
    assert!(body.contains("OpenSearchPalette"));
    // @step And the Action enum declares the "SessionListLoaded" variant
    assert!(body.contains("SessionListLoaded"));
    // @step And the Action enum declares the "AttachToSession" variant
    assert!(body.contains("AttachToSession"));
    // @step And the Action enum declares the "InsertIntoInput" variant
    assert!(body.contains("InsertIntoInput"));
    // @step And the Action enum declares the "SearchHistory" variant
    assert!(body.contains("SearchHistory"));
    // @step And the Action enum declares the "HistorySearchResults" variant
    assert!(body.contains("HistorySearchResults"));
}

/// Scenario: No view file imports forbidden crates
#[test]
fn views_do_not_directly_import_forbidden_crates() {
    // @step Given the codelet/fspec-tui crate
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
    let mut violations: Vec<String> = Vec::new();
    for path in &files {
        let body = read_raw(path);
        // @step Then no file under codelet/fspec-tui/src/views/ imports codelet_core
        // @step And no file under codelet/fspec-tui/src/views/ imports codelet_napi
        // @step And no file under codelet/fspec-tui/src/views/ imports tarpc
        // @step And no file under codelet/fspec-tui/src/views/ imports tokio_tungstenite
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
    }
    assert!(violations.is_empty(), "violations: {violations:?}");
}
