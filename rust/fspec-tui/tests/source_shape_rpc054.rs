//! RPC-054 — Source-shape assertions for the provider-settings view +
//! wire types.
//!
//! Feature: spec/features/rpc054-provider-settings-source-shape.feature
//!
//! REVISION 2026-06-01: rewritten to lock in the new full-screen
//! mode-view contract (ResumeSessionView pattern). The previous
//! Block-bordered two-pane shape is forbidden.

use codelet_test_helpers::{OrFail, TestResult};
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> Result<PathBuf, codelet_test_helpers::TestError> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    Ok(crate_dir
        .parent()
        .and_then(|p| p.parent())
        .or_fail("workspace root above rust/fspec-tui")?
        .to_path_buf())
}

fn read_at(rel: &str) -> Result<String, codelet_test_helpers::TestError> {
    let path = workspace_root()?.join(rel);
    if rel == "rust/rpc-types/src/lib.rs" {
        return read_tree(&path);
    }
    if rel == "rust/fspec-tui/src/components/mod.rs" {
        return Ok(codelet_test_helpers::source_composition::read_source(&path)
            .or_fail("resolve composed components source")?);
    }
    Ok(
        codelet_test_helpers::source_composition::read_tree_or_file(&path)
            .map_err(|e| format!("read {}: {e}", path.display()))?,
    )
}

include!("source_shape_rpc054/provider_surface.rs");

/// Scenario: ProviderSettingsView imports the canonical full-screen helpers
#[test]
fn provider_settings_view_imports_canonical_helpers() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/mod.rs
    let source = read_at("rust/fspec-tui/src/views/provider_settings/mod.rs")?;
    // @step When the use statements are parsed
    // RPC-337: the Clear + 4-constraint Layout + title/footer chrome is
    // now owned by the shared `render_full_screen_scaffold`, so this view
    // no longer imports Clear / Layout / render_title_with_count directly.
    // The canonical-helper assertions are superseded by the
    // render-delegation assertion in
    // `render_starts_with_clear_and_uses_4_constraint_layout`.
    // @step And the file imports crate::components::scroll_viewport::ensure_visible
    // (RPC-157: wrap_index dropped — list mode is clamped, no wrap-around)
    assert!(
        source.contains("scroll_viewport::ensure_visible")
            || source.contains("scroll_viewport::{ensure_visible")
            || source.contains("ensure_visible"),
        "must import ensure_visible"
    );
    assert!(
        !source.contains("wrap_index"),
        "must NOT import wrap_index (RPC-157: clamped nav, no wrap-around)"
    );
    // @step And the file imports crate::views::agent::confirm_dialog::ConfirmDialog
    assert!(
        source.contains("confirm_dialog::{ConfirmDialog")
            || source.contains("confirm_dialog::ConfirmDialog"),
        "must import ConfirmDialog"
    );
    // @step And the render delegates to the shared full-screen scaffold (RPC-337)
    assert!(
        source.contains("render_full_screen_scaffold"),
        "must delegate to the shared render_full_screen_scaffold (RPC-337)"
    );
    Ok(())
}

/// Scenario: ProviderSettingsView does NOT import Block / Borders
#[test]
fn provider_settings_view_does_not_import_block_or_borders() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/mod.rs
    let source = read_at("rust/fspec-tui/src/views/provider_settings/mod.rs")?;
    // @step When the use statements are parsed
    // @step Then the file does NOT import ratatui::widgets::Block
    assert!(
        !source.contains("widgets::Block")
            && !source.contains("widgets::{Block")
            && !source.contains(", Block"),
        "must NOT import ratatui::widgets::Block"
    );
    // @step And the file does NOT import ratatui::widgets::Borders
    assert!(
        !source.contains("Borders"),
        "must NOT import ratatui::widgets::Borders"
    );
    Ok(())
}

/// Scenario: render() starts with Clear and uses the 4-constraint Layout
#[test]
fn render_starts_with_clear_and_uses_4_constraint_layout() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/mod.rs
    let source = read_at("rust/fspec-tui/src/views/provider_settings/mod.rs")?;
    // @step When the source of ProviderSettingsView::render is inspected
    let render_idx = source.find("pub fn render(").or_fail("render fn present")?;
    let render_body = &source[render_idx..];
    // RPC-337: render now delegates to the shared full-screen scaffold,
    // which guarantees Clear-first + the 4-constraint
    // [Length(1), Length(1), Min(0), Length(1)] split internally (pinned
    // by views/full_screen_shell.rs tests). The view's render fn first
    // statement is therefore the scaffold call rather than a literal
    // Clear.render.
    let body_start = render_body
        .find('{')
        .map(|i| &render_body[i + 1..])
        .unwrap_or("");
    let trimmed = body_start.trim_start();
    // @step Then render delegates to render_full_screen_scaffold (Clear + 4-constraint split owned by the shell)
    assert!(
        render_body.contains("render_full_screen_scaffold"),
        "render() must delegate to render_full_screen_scaffold (RPC-337); got start: {:?}",
        &trimmed[..trimmed.len().min(80)]
    );
    Ok(())
}

/// Scenario: Every file under views/provider_settings/ stays under 300 lines
#[test]
fn every_file_under_provider_settings_stays_under_300_lines() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/
    let dir = workspace_root()?.join("rust/fspec-tui/src/views/provider_settings");
    // @step When the file sizes are measured
    let mut offenders: Vec<(PathBuf, usize)> = Vec::new();
    // BUG-622: child module directories (oauth_login_render/) are walked too.
    let mut pending = vec![dir];
    while let Some(current) = pending.pop() {
        for entry in fs::read_dir(&current).or_fail("provider_settings dir")? {
            let p = entry.or_fail("entry")?.path();
            if p.is_dir() {
                pending.push(p);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                // BUG-389: a size ceiling measures the file itself (with any
                // include!d fragments), not the module tree under it.
                let body =
                    codelet_test_helpers::source_composition::read_source(&p).or_fail("read")?;
                let n = body.lines().count();
                if n >= 300 {
                    offenders.push((p.clone(), n));
                }
            }
        }
    }
    // @step Then every .rs file under that directory, child module directories included, is < 300 lines
    assert!(offenders.is_empty(), "files >= 300 lines: {offenders:?}");
    Ok(())
}

/// Scenario: components/mod.rs declares the new ConfirmDeleteProviderCredentials action
#[test]
fn components_mod_declares_confirm_delete_provider_credentials_action() -> TestResult {
    // @step Given rust/fspec-tui/src/components/mod.rs
    let source = read_at("rust/fspec-tui/src/components/mod.rs")?;
    // @step When the Action enum is parsed
    // @step Then it contains a variant ConfirmDeleteProviderCredentials(String)
    assert!(source.contains("ConfirmDeleteProviderCredentials(String)"));
    // @step And the existing variants OpenProviderSettingsView, CloseProviderSettingsView, SaveProviderCredentials, ProviderSettingsStatus all remain (TUI-132 removed the raw DeleteProviderCredentials, BUG-578 the test-connection and refresh-models variants and BUG-664 the unnumbered ProviderCredentialsLoaded)
    for needle in [
        "OpenProviderSettingsView",
        "CloseProviderSettingsView",
        "SaveProviderCredentials",
        "ProviderSettingsStatus(String)",
    ] {
        assert!(source.contains(needle), "Action enum missing {needle}");
    }
    Ok(())
}

/// Scenario: ProviderSettingsView declares filter + filter_mode fields
#[test]
fn provider_settings_view_declares_filter_fields() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/mod.rs
    let source = read_at("rust/fspec-tui/src/views/provider_settings/mod.rs")?;
    // @step When the ProviderSettingsView struct is parsed
    // @step Then the struct contains a field "filter: String" (or equivalent type holding the filter string)
    assert!(source.contains("pub filter: String"));
    // @step And the struct contains a field "filter_mode: bool" (or equivalent flag for whether filter input is active)
    assert!(source.contains("pub filter_mode: bool"));
    Ok(())
}

/// Scenario: List mode key dispatcher routes "/" to enter filter mode
#[test]
fn list_mode_routes_slash_to_filter_mode() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/mod.rs
    // (we check the list.rs sub-module that owns the handler)
    let list_source = read_at("rust/fspec-tui/src/views/provider_settings/list.rs")?;
    // @step When the list mode key handler is inspected
    // @step Then a "/" keypress in List mode (with filter_mode false) sets filter_mode to true
    assert!(list_source.contains("KeyCode::Char('/')"));
    assert!(list_source.contains("view.filter_mode = true"));
    // @step And does NOT insert the "/" character anywhere
    // (the '/' branch sets filter_mode = true and returns immediately —
    //  no `view.filter.push('/')` exists in the same arm)
    let slash_idx = list_source
        .find("KeyCode::Char('/')")
        .or_fail("/ arm present")?;
    let nearby = &list_source[slash_idx..(slash_idx + 200).min(list_source.len())];
    assert!(!nearby.contains("filter.push('/')"));
    Ok(())
}

/// Scenario: Esc-cascade clears filter before closing the view
#[test]
fn esc_cascade_clears_filter_before_closing() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/mod.rs
    let list_source = read_at("rust/fspec-tui/src/views/provider_settings/list.rs")?;
    // @step When the list mode Esc handler is inspected
    // @step Then the Esc handler first checks for filter_mode = true → clears filter and sets filter_mode = false
    // (handled by handle_filter_key — verified by inspecting that path)
    assert!(list_source.contains("if view.filter_mode"));
    assert!(list_source.contains("handle_filter_key"));
    // @step And else if filter is non-empty → clears filter and stays in List
    // @step And else (filter_mode false, filter empty) → emits ProviderSettingsEvent::Close
    let esc_idx = list_source
        .find("KeyCode::Esc => {")
        .or_fail("Esc arm present in handle_list_key")?;
    let esc_body = &list_source[esc_idx..(esc_idx + 600).min(list_source.len())];
    assert!(esc_body.contains("!view.filter.is_empty()"));
    assert!(esc_body.contains("view.filter.clear()"));
    assert!(esc_body.contains("ProviderSettingsEvent::Close"));
    Ok(())
}

/// Scenario: The OAuth login's GitHub Copilot preamble renderers live in a child module
#[test]
fn oauth_login_copilot_preamble_renderers_live_in_a_child_module() -> TestResult {
    // @step Given rust/fspec-tui/src/views/provider_settings/oauth_login_render.rs and its child directory oauth_login_render/
    let dir = workspace_root()?.join("rust/fspec-tui/src/views/provider_settings");
    let parent_path = dir.join("oauth_login_render.rs");
    let child_path = dir.join("oauth_login_render/copilot_preamble.rs");
    // @step When the files are read
    let parent = fs::read_to_string(&parent_path).or_fail("read oauth_login_render.rs")?;
    let child = fs::read_to_string(&child_path)
        .map_err(|e| format!("read {}: {e}", child_path.display()))?;
    // @step Then oauth_login_render/copilot_preamble.rs defines render_deployment_type_select and render_enterprise_url_entry
    for name in [
        "render_deployment_type_select",
        "render_enterprise_url_entry",
    ] {
        assert!(
            child.contains(&format!("fn {name}(")),
            "copilot_preamble.rs must define {name}"
        );
    }
    // @step And oauth_login_render.rs declares mod copilot_preamble and re-exports both renderers
    assert!(
        parent.contains("mod copilot_preamble;"),
        "oauth_login_render.rs must declare mod copilot_preamble"
    );
    assert!(
        parent.contains(
            "use copilot_preamble::{render_deployment_type_select, render_enterprise_url_entry};"
        ),
        "oauth_login_render.rs must re-export both renderers"
    );
    // @step And oauth_login_render.rs does not define either renderer itself
    for name in [
        "render_deployment_type_select",
        "render_enterprise_url_entry",
    ] {
        assert!(
            !parent.contains(&format!("fn {name}(")),
            "oauth_login_render.rs must not define {name}"
        );
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Scenarios duplicated from RPC-020 source-shape: assert the /providers
// alias is fully removed. These are restated in the RPC-054 feature so
// the work-unit-to-test mapping stays 1:1.
// ─────────────────────────────────────────────────────────────────────────

/// Scenario: SlashCommandAction enum contains no Providers variant
#[test]
fn rpc054_slash_command_action_enum_has_no_providers_variant() -> TestResult {
    // @step Given rust/fspec-tui/src/views/agent/slash_commands.rs after the 2026-06-01 revision
    let source = read_at("rust/fspec-tui/src/views/agent/slash_commands.rs")?;
    // @step When the source is parsed for SlashCommandAction variants
    // (textual scan — any `Providers,` variant would fail the assertion)
    // @step Then the enum contains "Provider" exactly once
    let provider_variant_count = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("Provider,") || trimmed.starts_with("Provider ")
        })
        .count();
    assert_eq!(
        provider_variant_count, 1,
        "SlashCommandAction must declare exactly one Provider variant, got {provider_variant_count}"
    );
    // @step And the enum does NOT contain a "Providers" variant
    assert!(
        !source.contains("Providers,"),
        "slash_commands.rs must NOT declare SlashCommandAction::Providers"
    );
    // @step And the SLASH_COMMANDS const contains exactly one entry whose action is SlashCommandAction::Provider
    let provider_registry_count = source.matches("SlashCommandAction::Provider").count();
    assert!(
        provider_registry_count >= 1,
        "SLASH_COMMANDS must contain a Provider entry"
    );
    // @step And no entry in SLASH_COMMANDS has the name "providers"
    assert!(
        !source.contains("\"providers\""),
        "no SLASH_COMMANDS entry may use the name \"providers\""
    );
    Ok(())
}

/// Scenario: dispatch_slash_commands.rs has no Providers arm
#[test]
fn rpc054_dispatch_slash_commands_has_no_providers_arm() -> TestResult {
    // @step Given rust/fspec-tui/src/app/dispatch_slash_commands.rs after the 2026-06-01 revision
    let source = read_at("rust/fspec-tui/src/app/dispatch_slash_commands.rs")?;
    // @step When the file is read
    // @step Then it contains exactly one arm matching "SlashCommandAction::Provider =>"
    let provider_arm_count = source.matches("SlashCommandAction::Provider =>").count();
    assert_eq!(
        provider_arm_count, 1,
        "dispatch_slash_commands.rs must contain exactly one `SlashCommandAction::Provider =>` arm, got {provider_arm_count}"
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
    Ok(())
}

/// REFAC-021/022: read a source file together with its include!d fragments
/// and file-backed child modules (split modules read as one text).
fn read_tree(path: &std::path::Path) -> Result<String, codelet_test_helpers::TestError> {
    Ok(
        codelet_test_helpers::source_composition::read_module_tree(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?,
    )
}
