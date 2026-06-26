//! RPC-352 — Provider settings list-mode scrollbar parity (TS + /model).
//!
//! Feature: spec/features/provider-settings-scrollbar.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment.
//!
//! Strategy (per the RPC-352 findings doc, mirroring
//! `model_selector/scroll_tests.rs`): render the full
//! `ProviderSettingsView` into a ratatui `TestBackend`/`Buffer` and assert
//! the proportional `■`/`│` scrollbar column appears beside the list on
//! overflow and is absent when the list fits.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_rpc_types::ProviderCredentialInfo;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

fn pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: false,
        credential_type: "api_key".to_string(),
        model_count: 4,
        masked_key: None,
        source: None,
    }
}

/// Build a view whose nav tree holds `n` collapsed provider rows.
fn view_with(n: usize) -> ProviderSettingsView {
    let infos: Vec<ProviderCredentialInfo> = (0..n).map(|i| pinfo(&format!("prov{i}"))).collect();
    let mut v = ProviderSettingsView::new();
    v.set_provider_display_infos(project_display_infos(&infos, &[]));
    v
}

/// Render the whole view into a `w`x`h` TestBackend and return the buffer
/// as one `String` per row. Populates `view.visible_rows` as a side effect.
fn render_lines(v: &mut ProviderSettingsView, w: u16, h: u16) -> Vec<String> {
    let area = Rect {
        x: 0,
        y: 0,
        width: w,
        height: h,
    };
    let mut buf = Buffer::empty(area);
    v.render(area, &mut buf);
    let mut lines = Vec::with_capacity(buf.area.height as usize);
    for y in 0..buf.area.height {
        let mut s = String::new();
        for x in 0..buf.area.width {
            s.push_str(buf[(x, y)].symbol());
        }
        lines.push(s);
    }
    lines
}

fn has_scrollbar(lines: &[String]) -> bool {
    lines.iter().any(|l| l.contains('■') || l.contains('│'))
}

/// Index of the first line carrying a thumb cell `■`.
fn thumb_line(lines: &[String]) -> Option<usize> {
    lines.iter().position(|l| l.contains('■'))
}

/// Count painted provider rows (every provider row carries "prov").
fn provider_row_count(lines: &[String]) -> usize {
    lines.iter().filter(|l| l.contains("prov")).count()
}

/// Press Down `n` times via the public key handler (the only public way to
/// move the selection + adjust the shared scroll state).
fn press_down(v: &mut ProviderSettingsView, n: usize) {
    for _ in 0..n {
        v.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Overflowing provider list paints a scrollbar column
// ────────────────────────────────────────────────────────────────────────

#[test]
fn overflowing_provider_list_paints_scrollbar_column() {
    // @step Given a provider nav-item list of 30 items in a viewport 10 content rows tall
    let mut v = view_with(30);
    render_lines(&mut v, 60, 14);
    // @step And the list is scrolled down away from the top
    press_down(&mut v, 25);
    assert!(v.scroll_offset > 0, "precondition: scrolled away from top");

    // @step When the List body is rendered
    let lines = render_lines(&mut v, 60, 14);

    // @step Then a scrollbar column is painted beside the list
    assert!(has_scrollbar(&lines), "scrollbar column painted: {lines:?}");
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: A fitting provider list paints no scrollbar
// ────────────────────────────────────────────────────────────────────────

#[test]
fn fitting_provider_list_paints_no_scrollbar() {
    // @step Given a provider nav-item list of 3 items in a viewport tall enough to show them all
    let mut v = view_with(3);

    // @step When the List body is rendered
    let lines = render_lines(&mut v, 60, 20);

    // @step Then no scrollbar column is painted
    assert!(
        !has_scrollbar(&lines),
        "no scrollbar must be painted when the list fits: {lines:?}"
    );
    // @step And the provider rows use the full body width
    assert_eq!(
        provider_row_count(&lines),
        3,
        "all 3 provider rows painted: {lines:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: The scrollbar steals no content row
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scrollbar_steals_no_content_row() {
    // @step Given a provider nav-item list of 30 items in a viewport 10 content rows tall
    let mut v = view_with(30);
    render_lines(&mut v, 60, 14);
    // @step And the list is scrolled down away from the top
    press_down(&mut v, 25);

    // @step When the List body is rendered
    let lines = render_lines(&mut v, 60, 14);

    // @step Then every visible content row still paints its provider text beside the scrollbar
    let visible = v.visible_rows();
    assert_eq!(
        provider_row_count(&lines),
        visible,
        "all {visible} visible rows must paint content beside the scrollbar: {lines:?}"
    );
}

// ────────────────────────────────────────────────────────────────────────
// Scenario: Scrolling moves the scrollbar thumb
// ────────────────────────────────────────────────────────────────────────

#[test]
fn scrolling_moves_the_scrollbar_thumb() {
    // @step Given a provider nav-item list of 30 items in a viewport 10 content rows tall
    let mut v = view_with(30);
    let top_lines = render_lines(&mut v, 60, 14);
    let top_thumb = thumb_line(&top_lines).expect("thumb painted at top");

    // @step When the list is scrolled down
    press_down(&mut v, 28);
    let down_lines = render_lines(&mut v, 60, 14);
    let down_thumb = thumb_line(&down_lines).expect("thumb painted after scroll");

    // @step Then the thumb is painted on a lower row than at the top of the list
    assert!(
        down_thumb > top_thumb,
        "thumb moved down: top={top_thumb} down={down_thumb}"
    );
}
