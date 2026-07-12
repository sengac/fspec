//! RPC-350 — Provider settings list-mode visual parity regressions vs TypeScript.
//!
//! Feature: spec/features/rpc350-provider-settings-list-mode-parity.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment.
//!
//! Strategy (per the RPC-350 findings doc): render the full
//! `ProviderSettingsView` into a ratatui `TestBackend`/`Buffer` and assert
//! cell-level fg/bg/modifier styling per column range — the same pattern used
//! by `provider_settings_row_render_rpc104.rs` and
//! `provider_settings_test_result_inline_rpc158.rs`. No async, no NAPI.
//!
//! R1 (title two-span), R2 (openai dim profile badge), R3 ("Create new
//! profile" label), R4 (per-span green/dim/gray + black-on-band when selected)
//! and R5 (shared blue title guard for non-provider views).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::agent::ResumeSessionView;
use codelet_fspec_tui::views::provider_settings::nav_item::ProviderDisplayInfo;
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_rpc_types::SessionInfo;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn area() -> Rect {
    Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    }
}

fn render_to_buffer(view: &mut ProviderSettingsView) -> Buffer {
    let a = area();
    let mut buf = Buffer::empty(a);
    view.render(a, &mut buf);
    buf
}

/// Concatenate the visible symbols of row `y` into a String.
fn row_string(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s
}

/// Find the y-index of the first row whose joined text contains `needle`.
fn find_row(buf: &Buffer, needle: &str) -> Option<u16> {
    (0..buf.area.height).find(|&y| row_string(buf, y).contains(needle))
}

/// Display column (x) where the substring `needle` begins on row `y`, in
/// terms of the joined symbol string (treats each cell symbol as one unit,
/// which matches single-width chars used in the asserted segments).
fn col_of(buf: &Buffer, y: u16, needle: &str) -> usize {
    let row = row_string(buf, y);
    row.find(needle)
        .map(|byte_idx| row[..byte_idx].chars().count())
        .unwrap_or_else(|| panic!("substring {needle:?} not found on row {y}: {row:?}"))
}

fn cell_fg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf[(x, y)].fg
}

fn cell_bg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf[(x, y)].bg
}

fn cell_mod(buf: &Buffer, x: u16, y: u16) -> Modifier {
    buf[(x, y)].modifier
}

fn plain_provider(id: &str, name: &str) -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: id.to_string(),
        name: name.to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: false,
        env_var: None,
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
        masked_key: None,
        source: None,
    }
}

fn configured_provider(
    id: &str,
    name: &str,
    masked_key: &str,
    source: &str,
) -> ProviderDisplayInfo {
    let mut p = plain_provider(id, name);
    p.configured = true;
    p.masked_key = Some(masked_key.to_string());
    p.source = Some(source.to_string());
    p
}

/// 19 distinct plain providers — gives a title count of "(19 items)" when all
/// collapsed (matches the RPC-350 reference screenshot tree size).
fn nineteen_providers() -> Vec<ProviderDisplayInfo> {
    (0..19)
        .map(|i| plain_provider(&format!("prov{i}"), &format!("Provider {i}")))
        .collect()
}

// ════════════════════════════════════════════════════════════════════════
// R1 — Scenario: Title renders the name in bold yellow and the item count in
// dim gray
// ════════════════════════════════════════════════════════════════════════
#[test]
fn title_name_bold_yellow_count_dim_gray() {
    // @step Given the provider settings view has 19 nav items
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(nineteen_providers());
    assert_eq!(
        view.nav_items.len(),
        19,
        "fixture should yield 19 nav items"
    );

    // @step When the view is rendered to the terminal buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the title row reads "Provider Settings (19 items)"
    let ty = find_row(&buf, "Provider Settings (19 items)")
        .expect("title row 'Provider Settings (19 items)' should be present");

    // @step And the "Provider Settings" name segment is foreground yellow and bold
    let name_start = col_of(&buf, ty, "Provider Settings") as u16;
    let name_len = "Provider Settings".chars().count() as u16;
    for x in name_start..name_start + name_len {
        assert_eq!(
            cell_fg(&buf, x, ty),
            Color::Yellow,
            "name cell {x} should be Yellow"
        );
        assert!(
            cell_mod(&buf, x, ty).contains(Modifier::BOLD),
            "name cell {x} should be BOLD"
        );
    }

    // @step And the " (19 items)" count segment is foreground dim gray
    let count_start = col_of(&buf, ty, "(19 items)") as u16;
    let count_len = "(19 items)".chars().count() as u16;
    for x in count_start..count_start + count_len {
        let is_dim = cell_fg(&buf, x, ty) == Color::DarkGray
            || cell_mod(&buf, x, ty).contains(Modifier::DIM);
        assert!(
            is_dim,
            "count cell {x} should be dim gray (DarkGray fg or DIM modifier), got fg={:?} mod={:?}",
            cell_fg(&buf, x, ty),
            cell_mod(&buf, x, ty)
        );
        assert_ne!(
            cell_fg(&buf, x, ty),
            Color::Yellow,
            "count cell {x} must NOT be Yellow"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// R2 — Scenario: Expanded OpenAI provider with profiles shows a dim pluralized
// profile badge
// ════════════════════════════════════════════════════════════════════════
#[test]
fn openai_expanded_shows_dim_pluralized_profile_badge() {
    // @step Given the openai provider is expanded with one profile named "qwen"
    let mut view = ProviderSettingsView::new();
    let mut openai = plain_provider("openai", "OpenAI API");
    openai.profiles = vec!["qwen".to_string()];
    view.set_provider_display_infos(vec![openai]);
    view.toggle_expansion("openai");
    // Keep the header row UNSELECTED so the dim badge styling is observable
    // (a selected row would correctly flip every segment to black-on-band).
    view.selected_index = 99;

    // @step When the view is rendered to the terminal buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the openai header row contains the suffix " (1 profile)"
    let hy = find_row(&buf, "(1 profile)")
        .expect("openai header row should contain '(1 profile)' suffix");
    let header = row_string(&buf, hy);
    assert!(
        header.contains("OpenAI API") && header.contains("(1 profile)"),
        "openai header should carry both name and badge, got {header:?}"
    );
    assert!(
        !header.contains("(1 profiles)"),
        "singular profile must not pluralize, got {header:?}"
    );

    // @step And the " (1 profile)" badge segment is rendered dim
    let badge_start = col_of(&buf, hy, "(1 profile)") as u16;
    let badge_len = "(1 profile)".chars().count() as u16;
    for x in badge_start..badge_start + badge_len {
        let is_dim = cell_fg(&buf, x, hy) == Color::DarkGray
            || cell_mod(&buf, x, hy).contains(Modifier::DIM);
        assert!(
            is_dim,
            "badge cell {x} should be dim, got fg={:?} mod={:?}",
            cell_fg(&buf, x, hy),
            cell_mod(&buf, x, hy)
        );
    }

    // @step And a second openai profile changes the badge to " (2 profiles)"
    let mut view2 = ProviderSettingsView::new();
    let mut openai2 = plain_provider("openai", "OpenAI API");
    openai2.profiles = vec!["qwen".to_string(), "fast".to_string()];
    view2.set_provider_display_infos(vec![openai2]);
    view2.toggle_expansion("openai");
    view2.selected_index = 99;
    let buf2 = render_to_buffer(&mut view2);
    let hy2 = find_row(&buf2, "(2 profiles)")
        .expect("two-profile openai header should contain '(2 profiles)'");
    let header2 = row_string(&buf2, hy2);
    assert!(
        header2.contains("(2 profiles)") && !header2.contains("(2 profile)"),
        "two profiles must pluralize, got {header2:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════
// R3 — Scenario: Add-profile row label reads "Create new profile"
// ════════════════════════════════════════════════════════════════════════
#[test]
fn add_profile_row_label_reads_create_new_profile() {
    // @step Given the openai provider is expanded
    let mut view = ProviderSettingsView::new();
    let openai = plain_provider("openai", "OpenAI API");
    view.set_provider_display_infos(vec![openai]);
    view.toggle_expansion("openai");

    // @step And the add-profile row is selected
    // (with no profiles, the nav tree is [provider(0), add-profile(1)])
    view.selected_index = 1;

    // @step When the view is rendered to the terminal buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the add-profile row label text is "Create new profile"
    let ry = find_row(&buf, "Create new profile")
        .expect("add-profile row should read 'Create new profile'");
    let row = row_string(&buf, ry);
    assert!(
        !row.contains("Add Profile"),
        "old 'Add Profile' label must be gone, got {row:?}"
    );

    // @step And the row is prefixed with the "+ " glyph and selection marker
    assert!(
        row.contains("> ") && row.contains("+ Create new profile"),
        "selected add-profile row should show '> ' marker and '+ ' glyph, got {row:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════
// R4 — Scenario: Configured unselected provider row paints per-color status
// segments
// ════════════════════════════════════════════════════════════════════════
#[test]
fn configured_unselected_row_paints_per_color_segments() {
    // @step Given an unselected configured "Google Gemini" provider row with masked key "AIza••••••••H3Ck" and source "env"
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![configured_provider(
        "gemini",
        "Google Gemini",
        "AIza••••••••H3Ck",
        "env",
    )]);
    // Keep selection OFF this row: park the cursor out of range so nothing is
    // selected on the rendered provider row.
    view.selected_index = 99;

    // @step When the view is rendered to the terminal buffer
    let buf = render_to_buffer(&mut view);
    let ry = find_row(&buf, "Google Gemini").expect("gemini provider row should render");

    // @step Then the provider name segment is foreground white
    let name_x = col_of(&buf, ry, "Google Gemini") as u16;
    assert_eq!(
        cell_fg(&buf, name_x, ry),
        Color::White,
        "provider name should be white"
    );
    assert_eq!(
        cell_bg(&buf, name_x, ry),
        Color::Reset,
        "unselected bg reset"
    );

    // @step And the "✓ AIza••••••••H3Ck" masked-key segment is foreground green
    let key_x = col_of(&buf, ry, "AIza") as u16;
    assert_eq!(
        cell_fg(&buf, key_x, ry),
        Color::Green,
        "masked-key segment should be green"
    );
    // The check glyph that precedes it is part of the green run too.
    let check_x = col_of(&buf, ry, "✓") as u16;
    assert_eq!(
        cell_fg(&buf, check_x, ry),
        Color::Green,
        "✓ glyph should be green"
    );

    // @step And the "[env]" source segment is foreground dim gray
    let src_x = col_of(&buf, ry, "[env]") as u16;
    let src_len = "[env]".chars().count() as u16;
    for x in src_x..src_x + src_len {
        let is_dim = cell_fg(&buf, x, ry) == Color::DarkGray
            || cell_mod(&buf, x, ry).contains(Modifier::DIM);
        assert!(
            is_dim,
            "source cell {x} should be dim gray, got fg={:?} mod={:?}",
            cell_fg(&buf, x, ry),
            cell_mod(&buf, x, ry)
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// R4 — Scenario: Unconfigured rows use gray empty-state text with distinct
// provider and api-key wording
// ════════════════════════════════════════════════════════════════════════
#[test]
fn unconfigured_rows_use_gray_empty_state_with_distinct_wording() {
    // @step Given an unselected unconfigured "Cohere" provider row
    let mut view = ProviderSettingsView::new();
    // cohere needs an api-key child on expansion: requires_api_key=true.
    let mut cohere = plain_provider("cohere", "Cohere");
    cohere.requires_api_key = true;
    view.set_provider_display_infos(vec![cohere]);
    view.selected_index = 99; // nothing selected

    // @step When the view is rendered to the terminal buffer
    let buf = render_to_buffer(&mut view);
    let ry = find_row(&buf, "Cohere").expect("cohere provider row should render");

    // @step Then the provider name segment is foreground white
    let name_x = col_of(&buf, ry, "Cohere") as u16;
    assert_eq!(
        cell_fg(&buf, name_x, ry),
        Color::White,
        "provider name should be white"
    );

    // @step And the "(not configured)" segment is foreground gray
    let nc_x = col_of(&buf, ry, "(not configured)") as u16;
    let nc_len = "(not configured)".chars().count() as u16;
    for x in nc_x..nc_x + nc_len {
        let fg = cell_fg(&buf, x, ry);
        assert!(
            fg == Color::Gray || fg == Color::DarkGray,
            "'(not configured)' cell {x} should be gray, got {fg:?}"
        );
    }

    // @step And an unconfigured api-key child row uses "(not set)" in gray instead
    let mut view2 = ProviderSettingsView::new();
    let mut cohere2 = plain_provider("cohere", "Cohere");
    cohere2.requires_api_key = true;
    view2.set_provider_display_infos(vec![cohere2]);
    view2.toggle_expansion("cohere");
    view2.selected_index = 99;
    let buf2 = render_to_buffer(&mut view2);
    let ay = find_row(&buf2, "(not set)").expect("api-key child should read '(not set)'");
    let ns_x = col_of(&buf2, ay, "(not set)") as u16;
    let ns_len = "(not set)".chars().count() as u16;
    for x in ns_x..ns_x + ns_len {
        let fg = cell_fg(&buf2, x, ay);
        assert!(
            fg == Color::Gray || fg == Color::DarkGray,
            "'(not set)' cell {x} should be gray, got {fg:?}"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// R4 — Scenario: Selected configured provider row flips all segments to black
// over the colour band
// ════════════════════════════════════════════════════════════════════════
#[test]
fn selected_configured_row_flips_all_segments_black_over_band() {
    // @step Given a selected configured provider row with a masked key and source
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![configured_provider(
        "gemini",
        "Google Gemini",
        "AIza••••••••H3Ck",
        "env",
    )]);
    view.selected_index = 0; // the provider row IS selected

    // @step When the view is rendered to the terminal buffer
    let buf = render_to_buffer(&mut view);
    let ry = find_row(&buf, "Google Gemini").expect("gemini provider row should render");

    // @step Then the entire row paints a yellow background band
    for x in 0..buf.area.width {
        assert_eq!(
            cell_bg(&buf, x, ry),
            Color::Yellow,
            "selected provider row cell {x} bg should be Yellow band"
        );
    }

    // @step And the name, masked-key and source segments are all foreground black
    for needle in ["Google Gemini", "AIza", "[env]"] {
        let sx = col_of(&buf, ry, needle) as u16;
        let len = needle.chars().count() as u16;
        for x in sx..sx + len {
            assert_eq!(
                cell_fg(&buf, x, ry),
                Color::Black,
                "selected-row segment {needle:?} cell {x} fg should be Black"
            );
        }
    }
}

// ════════════════════════════════════════════════════════════════════════
// R5 — Scenario: Non-provider full-screen views keep their existing title
// styling
// ════════════════════════════════════════════════════════════════════════
fn fake_session(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.to_string(),
        name: id.to_string(),
        status: "idle".to_string(),
        project: String::new(),
        message_count: 0,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: None,
    }
}

#[test]
fn resume_session_view_keeps_shared_blue_title() {
    // @step Given the Resume Session full-screen view with 5 sessions
    let mut view = ResumeSessionView::new();
    view.set_sessions((0..5).map(|i| fake_session(&format!("s{i}"))).collect());

    // @step When that view is rendered to the terminal buffer
    let a = area();
    let mut buf = Buffer::empty(a);
    view.render(a, &mut buf);

    // @step Then its title row "Resume Session (5 available)" keeps the shared blue bold styling
    let ty =
        find_row(&buf, "Resume Session (5 available)").expect("resume title row should render");
    let name_x = col_of(&buf, ty, "Resume Session") as u16;
    assert_eq!(
        cell_fg(&buf, name_x, ty),
        Color::Blue,
        "resume title should stay Blue"
    );
    assert!(
        cell_mod(&buf, name_x, ty).contains(Modifier::BOLD),
        "resume title should stay BOLD"
    );

    // @step And the provider-specific two-span title change does not affect it
    // The count segment of the SHARED title is part of the same blue run — it
    // must NOT be DarkGray/dim (which is the provider-only treatment).
    let count_x = col_of(&buf, ty, "(5 available)") as u16;
    assert_eq!(
        cell_fg(&buf, count_x, ty),
        Color::Blue,
        "shared title count must stay Blue (not dim gray)"
    );
}
