//! RPC-026 — Source-shape regression for the resume + search mode views.
//!
//! Feature: spec/features/rpc026-resume-and-search-mode-views.feature
//!
//! Verifies the post-refactor file layout: the new mode-view widgets
//! exist under views/agent/ with the required surface and < 300 LoC;
//! the legacy popup widgets are deleted; no view file imports
//! `tui_popup` from the resume/search mode-view files; the dispatch
//! orchestrator + Action enum carry the new RPC-026 surface.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

fn fspec_tui_src() -> std::path::PathBuf {
    common::workspace_root().join("fspec-tui").join("src")
}

fn read_raw(path: &std::path::Path) -> String {
    common::read_to_string_or_panic(path)
}

fn count_lines(path: &std::path::Path) -> usize {
    read_raw(path).lines().count()
}

/// Scenario: New view files exist with the required shape and forbidden imports absent
#[test]
fn new_mode_view_files_exist_with_documented_surface() {
    // @step Given the repository is at the RPC-026 implementing snapshot
    let agent_dir = fspec_tui_src().join("views").join("agent");

    // @step When the source-shape regression test runs
    // (Implicit: this test body IS the regression scan; the assertions below are the result.)

    // @step Then codelet/fspec-tui/src/views/agent/resume_session_view.rs exists with line count < 300
    let resume = agent_dir.join("resume_session_view.rs");
    assert!(resume.is_file(), "{} must exist", resume.display());
    assert!(count_lines(&resume) < 300);

    // @step And codelet/fspec-tui/src/views/agent/search_history_view.rs exists with line count < 300
    let search = agent_dir.join("search_history_view.rs");
    assert!(search.is_file(), "{} must exist", search.display());
    assert!(count_lines(&search) < 300);

    // @step And codelet/fspec-tui/src/views/agent/confirm_dialog.rs exists with line count < 300
    let dialog = agent_dir.join("confirm_dialog.rs");
    assert!(dialog.is_file(), "{} must exist", dialog.display());
    assert!(count_lines(&dialog) < 300);

    let resume_body = read_raw(&resume);
    let search_body = read_raw(&search);

    // @step And resume_session_view.rs contains no occurrences of "tui_popup" or "popup_body"
    assert!(!resume_body.contains("tui_popup"));
    assert!(!resume_body.contains("popup_body"));

    // @step And search_history_view.rs contains no occurrences of "tui_popup" or "popup_body"
    assert!(!search_body.contains("tui_popup"));
    assert!(!search_body.contains("popup_body"));

    // @step And the first non-attribute statement in each mode view's render fn is "Clear.render(area, buf)" or "frame.render_widget(Clear, area)"
    // RPC-337: ResumeSessionView delegates its scaffold (Clear +
    // 4-constraint split + footer + overlay) to the shared
    // `render_full_screen_scaffold` helper. RPC-339: SearchHistoryView is
    // now ALSO refit, delegating to the title-closure variant
    // `render_full_screen_scaffold_with_title` (its editable-query title
    // needs a caller-supplied title renderer). For both views the render
    // fn's first statement is the scaffold call rather than a literal
    // `Clear.render`; the Clear-first invariant is preserved inside the
    // shell (see views/full_screen_shell.rs tests).
    let resume_render_idx = resume_body
        .find("pub fn render(&self, area: Rect, buf: &mut Buffer)")
        .expect("resume render fn");
    let body_after = &resume_body[resume_render_idx..];
    let brace_idx = body_after.find('{').expect("opening brace");
    let after_brace = &body_after[brace_idx + 1..];
    let trimmed = after_brace.trim_start();
    assert!(
        trimmed.starts_with("Clear.render(area, buf)")
            || trimmed.starts_with("crate::views::full_screen_shell::render_full_screen_scaffold"),
        "resume render fn first stmt must paint Clear OR delegate to the shared shell; got: {}",
        &trimmed[..trimmed.len().min(80)]
    );

    let search_render_idx = search_body
        .find("pub fn render(&self, area: Rect, buf: &mut Buffer)")
        .expect("search render fn");
    let s_body_after = &search_body[search_render_idx..];
    let s_brace_idx = s_body_after.find('{').expect("opening brace");
    let s_after_brace = &s_body_after[s_brace_idx + 1..];
    let s_trimmed = s_after_brace.trim_start();
    assert!(
        s_trimmed.starts_with("Clear.render(area, buf)")
            || s_trimmed.starts_with("render_full_screen_scaffold_with_title")
            || s_trimmed
                .starts_with("crate::views::full_screen_shell::render_full_screen_scaffold_with_title"),
        "search render fn first stmt must paint Clear OR delegate to the title-closure shell; got: {}",
        &s_trimmed[..s_trimmed.len().min(80)]
    );

    // @step And codelet/fspec-tui/src/views/agent.rs line count is < 300
    let agent_rs = fspec_tui_src().join("views").join("agent.rs");
    assert!(count_lines(&agent_rs) < 300);

    // @step And codelet/fspec-tui/src/app/dispatch_resume_search_views.rs line count is < 300
    let dispatch026 = fspec_tui_src().join("app").join("dispatch_resume_search_views.rs");
    assert!(count_lines(&dispatch026) < 300);
}

/// Scenario: Old popup files are removed and their identifiers no longer appear
#[test]
fn old_popup_files_removed_and_identifiers_gone() {
    // @step Given the repository is at the RPC-026 implementing snapshot
    let src = fspec_tui_src();

    // @step And codelet/fspec-tui/src/views/agent/resume_picker.rs does NOT exist
    let picker = src.join("views").join("agent").join("resume_picker.rs");
    assert!(!picker.exists(), "{} must NOT exist", picker.display());

    // @step And codelet/fspec-tui/src/views/agent/search_palette.rs does NOT exist
    let palette = src.join("views").join("agent").join("search_palette.rs");
    assert!(!palette.exists(), "{} must NOT exist", palette.display());

    // @step When ripgrep searches codelet/fspec-tui/src/ for "ResumePicker"
    // @step Then zero matches are returned
    // @step When ripgrep searches codelet/fspec-tui/src/ for "SearchPalette"
    // @step Then zero matches are returned
    let mut violations: Vec<String> = Vec::new();
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
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    walk(&src, &mut files);
    for path in &files {
        let body = read_raw(path);
        for needle in ["ResumePicker", "SearchPalette"] {
            if body.contains(needle) {
                violations.push(format!("{} contains {}", path.display(), needle));
            }
        }
    }
    assert!(violations.is_empty(), "violations: {violations:?}");
}

/// Scenario: AgentView gains resume_view/search_view fields
#[test]
fn agent_view_orchestrator_owns_the_new_mode_view_fields() {
    // @step Given codelet/fspec-tui/src/views/agent.rs after RPC-026 lands
    let path = fspec_tui_src().join("views").join("agent.rs");
    let body = read_raw(&path);
    assert!(count_lines(&path) < 300);
    // @step And the file declares the "resume_view" field
    assert!(body.contains("resume_view"));
    // @step And the file declares the "search_view" field
    assert!(body.contains("search_view"));
    // @step And the file early-returns when a mode view is active
    assert!(body.contains("self.resume_view.as_ref()"));
    assert!(body.contains("self.search_view.as_ref()"));
}

/// Scenario: handle_slash_command dispatches the renamed Action variants
#[test]
fn handle_slash_command_dispatches_open_resume_view_and_open_search_view() {
    // @step Given codelet/fspec-tui/src/app/dispatch_slash_commands.rs after RPC-026 lands
    let path = fspec_tui_src().join("app").join("dispatch_slash_commands.rs");
    let body = read_raw(&path);
    assert!(count_lines(&path) < 300);
    // @step And the file routes "SlashCommandAction::Resume" through handle_open_resume_view
    assert!(body.contains("SlashCommandAction::Resume"));
    assert!(body.contains("handle_open_resume_view"));
    // @step And the file routes "SlashCommandAction::Search" through handle_open_search_view
    assert!(body.contains("SlashCommandAction::Search"));
    assert!(body.contains("handle_open_search_view"));
}

/// Scenario: Action enum gains the RPC-026 mode-view variants
#[test]
fn action_enum_gains_mode_view_variants() {
    // @step Given codelet/fspec-tui/src/components/mod.rs after RPC-026 lands
    let body = read_raw(&fspec_tui_src().join("components").join("mod.rs"));
    // @step Then the Action enum declares the "OpenResumeView" variant
    assert!(body.contains("OpenResumeView"));
    // @step And the Action enum declares the "OpenSearchView" variant
    assert!(body.contains("OpenSearchView"));
    // @step And the Action enum declares the "CloseResumeView" variant
    assert!(body.contains("CloseResumeView"));
    // @step And the Action enum declares the "CloseSearchView" variant
    assert!(body.contains("CloseSearchView"));
    // @step And the Action enum declares the "RequestDeleteSession" variant
    assert!(body.contains("RequestDeleteSession"));
    // @step And the Action enum declares the "ConfirmDeleteSession" variant
    assert!(body.contains("ConfirmDeleteSession"));
    // Existing RPC-026 variants must still be present
    assert!(body.contains("SessionListLoaded"));
    assert!(body.contains("AttachToSession"));
    assert!(body.contains("InsertIntoInput"));
    assert!(body.contains("SearchHistory"));
    assert!(body.contains("HistorySearchResults"));
    // Pre-refactor names must NOT appear
    assert!(!body.contains("OpenResumePicker"));
    assert!(!body.contains("OpenSearchPalette"));
}
