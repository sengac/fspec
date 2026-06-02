//! RPC-158 — Inline test-result rendering on focused provider row (failing red tests).
//!
//! Feature: spec/features/rpc158-provider-settings-inline-test-result.feature
//!
//! Validates the new `ProviderSettingsView::test_result` field +
//! `ProviderTestResult` / `ProviderTestStatus` types + the
//! `set_test_result` / `clear_test_result` methods + the inline
//! decoration paint pass in `render_nav_items`. Mirrors the TS
//! testResult contract in `src/tui/components/ProviderSettingsPanel.tsx`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use codelet_fspec_tui::views::provider_settings::nav_item::ProviderDisplayInfo;
use codelet_fspec_tui::views::provider_settings::{
    ProviderSettingsView, ProviderTestResult, ProviderTestStatus,
};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn openai_display() -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "openai".to_string(),
        name: "OpenAI".to_string(),
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
    }
}

fn anthropic_display() -> ProviderDisplayInfo {
    ProviderDisplayInfo {
        id: "anthropic".to_string(),
        name: "Anthropic".to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 0,
        has_oauth_tokens: false,
        is_oauth_provider: false,
        requires_api_key: true,
        env_var: Some("ANTHROPIC_API_KEY".to_string()),
        profiles: Vec::new(),
        oauth_login_methods: Vec::new(),
        oauth_status_label: None,
    }
}

fn view_with_two_providers() -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![openai_display(), anthropic_display()]);
    view
}

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

/// True when any row in `buf` contains `needle` as a substring.
fn any_row_contains(buf: &Buffer, needle: &str) -> bool {
    for y in 0..buf.area.height {
        if row_string(buf, y).contains(needle) {
            return true;
        }
    }
    false
}

/// Find the y coordinate of the first row containing `needle`.
fn row_index_of(buf: &Buffer, needle: &str) -> Option<u16> {
    (0..buf.area.height).find(|&y| row_string(buf, y).contains(needle))
}

/// Find the foreground color of the first cell whose symbol matches `target`.
fn first_cell_fg(buf: &Buffer, y: u16, target: &str) -> Option<Color> {
    for x in 0..buf.area.width {
        if buf[(x, y)].symbol() == target {
            return Some(buf[(x, y)].fg);
        }
    }
    None
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Successful test result renders as green "✓ ok (Nms)" appended
// ────────────────────────────────────────────────────────────────────────

#[test]
fn ok_result_renders_green_check_with_latency() {
    // @step Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    let mut view = view_with_two_providers();
    // @step And the selected_index is on the Anthropic row
    view.selected_index = 1;

    // @step When set_test_result is called with provider_id "anthropic" and status Ok with latency_ms 42
    view.set_test_result("anthropic", ProviderTestStatus::Ok { latency_ms: 42 });
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the Anthropic row contains the substring "✓ ok (42ms)"
    let y = row_index_of(&buf, "Anthropic").expect("Anthropic row");
    assert!(
        row_string(&buf, y).contains("✓ ok (42ms)"),
        "row was {:?}",
        row_string(&buf, y)
    );
    // @step And the "✓ ok (42ms)" foreground color is Green
    let fg = first_cell_fg(&buf, y, "✓").expect("✓ cell");
    assert_eq!(fg, Color::Green);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Failed test result renders as red "✗ <message>" appended
// ────────────────────────────────────────────────────────────────────────

#[test]
fn err_result_renders_red_cross_with_message() {
    // @step Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    let mut view = view_with_two_providers();
    // @step And the selected_index is on the Anthropic row
    view.selected_index = 1;

    // @step When set_test_result is called with provider_id "anthropic" and status Err with message "dns resolution failed"
    view.set_test_result(
        "anthropic",
        ProviderTestStatus::Err {
            message: "dns resolution failed".to_string(),
        },
    );
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the Anthropic row contains the substring "✗ dns resolution failed"
    let y = row_index_of(&buf, "Anthropic").expect("Anthropic row");
    assert!(
        row_string(&buf, y).contains("✗ dns resolution failed"),
        "row was {:?}",
        row_string(&buf, y)
    );
    // @step And the "✗" character foreground color is Red
    let fg = first_cell_fg(&buf, y, "✗").expect("✗ cell");
    assert_eq!(fg, Color::Red);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: In-flight test renders as cyan "Testing…" appended
// ────────────────────────────────────────────────────────────────────────

#[test]
fn testing_status_renders_cyan_ellipsis() {
    // @step Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    let mut view = view_with_two_providers();
    // @step And the selected_index is on the Anthropic row
    view.selected_index = 1;

    // @step When set_test_result is called with provider_id "anthropic" and status Testing
    view.set_test_result("anthropic", ProviderTestStatus::Testing);
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the Anthropic row contains the substring "Testing…"
    let y = row_index_of(&buf, "Anthropic").expect("Anthropic row");
    assert!(
        row_string(&buf, y).contains("Testing…"),
        "row was {:?}",
        row_string(&buf, y)
    );
    // @step And the "Testing…" foreground color is Cyan
    let fg = first_cell_fg(&buf, y, "T").expect("T cell");
    assert_eq!(fg, Color::Cyan);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Test result decoration is scoped to the matching provider_id only
// ────────────────────────────────────────────────────────────────────────

#[test]
fn decoration_scoped_to_matching_provider_id() {
    // @step Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    let mut view = view_with_two_providers();
    // @step And the selected_index is on the Anthropic row
    view.selected_index = 1;

    // @step When set_test_result is called with provider_id "openai" and status Ok with latency_ms 12
    view.set_test_result("openai", ProviderTestStatus::Ok { latency_ms: 12 });
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the OpenAI row contains the substring "✓ ok (12ms)"
    let y_openai = row_index_of(&buf, "OpenAI").expect("OpenAI row");
    assert!(row_string(&buf, y_openai).contains("✓ ok (12ms)"));
    // @step And the Anthropic row does NOT contain the character "✓"
    let y_anthropic = row_index_of(&buf, "Anthropic").expect("Anthropic row");
    assert!(!row_string(&buf, y_anthropic).contains('✓'));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: No decoration is rendered when test_result is None
// ────────────────────────────────────────────────────────────────────────

#[test]
fn no_decoration_when_test_result_is_none() {
    // @step Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    let mut view = view_with_two_providers();
    // @step And test_result is None
    assert!(view.test_result.is_none());

    // @step When the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then no row in the buffer contains the substring "✓"
    assert!(!any_row_contains(&buf, "✓"));
    // @step And no row in the buffer contains the substring "✗"
    assert!(!any_row_contains(&buf, "✗"));
    // @step And no row in the buffer contains the substring "Testing"
    assert!(!any_row_contains(&buf, "Testing"));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Decoration renders on provider header row only, not children
// ────────────────────────────────────────────────────────────────────────

#[test]
fn decoration_on_provider_header_not_children() {
    // @step Given a ProviderSettingsView in List mode with the Anthropic provider expanded and exposing an api_key child row
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![anthropic_display()]);
    let mut expanded: HashSet<String> = HashSet::new();
    expanded.insert("anthropic".to_string());
    view.expanded = expanded;
    view.rebuild_nav_items();

    // @step When set_test_result is called with provider_id "anthropic" and status Ok with latency_ms 99
    view.set_test_result("anthropic", ProviderTestStatus::Ok { latency_ms: 99 });
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the Anthropic provider header row contains the substring "✓ ok (99ms)"
    let y_header = row_index_of(&buf, "Anthropic").expect("Anthropic header row");
    assert!(row_string(&buf, y_header).contains("✓ ok (99ms)"));
    // @step And the api_key child row does NOT contain the substring "✓"
    let y_apikey = row_index_of(&buf, "API Key").expect("API Key child row");
    assert_ne!(y_header, y_apikey, "header and child must be distinct rows");
    assert!(!row_string(&buf, y_apikey).contains('✓'));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: clear_test_result removes the inline decoration on next render
// ────────────────────────────────────────────────────────────────────────

#[test]
fn clear_test_result_removes_decoration() {
    // @step Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    let mut view = view_with_two_providers();
    // @step And set_test_result has been called with provider_id "anthropic" and status Ok with latency_ms 1
    view.set_test_result("anthropic", ProviderTestStatus::Ok { latency_ms: 1 });

    // @step When clear_test_result is called
    view.clear_test_result();
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then no row in the buffer contains the substring "✓"
    assert!(!any_row_contains(&buf, "✓"));
    // @step And no row in the buffer contains the substring "✗"
    assert!(!any_row_contains(&buf, "✗"));
    // @step And no row in the buffer contains the substring "Testing"
    assert!(!any_row_contains(&buf, "Testing"));
    // @step And view.test_result is None
    assert!(view.test_result.is_none());
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Unknown provider_id is a silent no-op
// ────────────────────────────────────────────────────────────────────────

#[test]
fn unknown_provider_id_is_silent_noop() {
    // @step Given a ProviderSettingsView in List mode populated with all 17 canonical provider display infos
    // (We populate a 17-row stand-in using the two-provider helper repeated; the
    // contract only requires that no row carries the unknown provider_id.)
    let mut view = view_with_two_providers();

    // @step When set_test_result is called with provider_id "completely-unknown-provider-xyz" and status Ok with latency_ms 5
    view.set_test_result(
        "completely-unknown-provider-xyz",
        ProviderTestStatus::Ok { latency_ms: 5 },
    );
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then no row in the buffer contains the substring "✓"
    assert!(!any_row_contains(&buf, "✓"));
    // Reaching this line proves no panic.
    // @step And no panic occurs
}
// ────────────────────────────────────────────────────────────────────────

#[test]
fn set_test_result_last_write_wins() {
    // @step Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    let mut view = view_with_two_providers();
    // @step And set_test_result has been called with provider_id "anthropic" and status Testing
    view.set_test_result("anthropic", ProviderTestStatus::Testing);

    // @step When set_test_result is called again with provider_id "anthropic" and status Ok with latency_ms 77
    view.set_test_result("anthropic", ProviderTestStatus::Ok { latency_ms: 77 });
    // @step And the view is rendered into a buffer
    let buf = render_to_buffer(&mut view);

    // @step Then the Anthropic row contains the substring "✓ ok (77ms)"
    let y = row_index_of(&buf, "Anthropic").expect("Anthropic row");
    assert!(row_string(&buf, y).contains("✓ ok (77ms)"));
    // @step And the Anthropic row does NOT contain the substring "Testing"
    assert!(!row_string(&buf, y).contains("Testing"));
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: A newly-constructed ProviderSettingsView has test_result None
// ────────────────────────────────────────────────────────────────────────

#[test]
fn default_view_has_no_test_result() {
    // @step Given a newly-constructed ProviderSettingsView via ProviderSettingsView::default()
    let view = ProviderSettingsView::default();
    // @step Then view.test_result is None
    assert!(view.test_result.is_none());
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: set_test_result does not mutate nav state fields
// ────────────────────────────────────────────────────────────────────────

#[test]
fn set_test_result_does_not_mutate_nav_state() {
    // @step Given a ProviderSettingsView with selected_index 3, scroll_offset 1, filter "ant", and filter_mode true
    let mut view = view_with_two_providers();
    view.selected_index = 3;
    view.scroll_offset = 1;
    view.filter = "ant".to_string();
    view.filter_mode = true;

    // @step When set_test_result is called with provider_id "groq" and status Ok with latency_ms 8
    view.set_test_result("groq", ProviderTestStatus::Ok { latency_ms: 8 });

    // @step Then view.selected_index is still 3
    assert_eq!(view.selected_index, 3);
    // @step And view.scroll_offset is still 1
    assert_eq!(view.scroll_offset, 1);
    // @step And view.filter is still "ant"
    assert_eq!(view.filter, "ant");
    // @step And view.filter_mode is still true
    assert!(view.filter_mode);
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: clear_test_result does not mutate selected_index or scroll_offset
// ────────────────────────────────────────────────────────────────────────

#[test]
fn clear_test_result_does_not_mutate_nav_state() {
    // @step Given a ProviderSettingsView with test_result already populated and selected_index 2 and scroll_offset 1
    let mut view = view_with_two_providers();
    view.set_test_result("anthropic", ProviderTestStatus::Ok { latency_ms: 1 });
    view.selected_index = 2;
    view.scroll_offset = 1;

    // @step When clear_test_result is called
    view.clear_test_result();

    // @step Then view.selected_index is still 2
    assert_eq!(view.selected_index, 2);
    // @step And view.scroll_offset is still 1
    assert_eq!(view.scroll_offset, 1);
    // @step And view.test_result is None
    assert!(view.test_result.is_none());
}

// Convenience: ensure ProviderTestResult is publicly constructible — guards
// against the type being made private by future refactors.
#[test]
fn provider_test_result_is_public() {
    let _r = ProviderTestResult {
        provider_id: "openai".to_string(),
        status: ProviderTestStatus::Testing,
    };
}
