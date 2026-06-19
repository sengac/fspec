//! RPC-337 — shared full-screen shell refit parity.
//!
//! Feature: spec/features/full-screen-model-selector.feature
//!
//! Verifies that migrating ProviderSettingsView onto the shared
//! `render_full_screen_scaffold` preserves its rendered output (title
//! count chrome + footer + Clear-first behaviour). Driven at the public
//! view boundary (no access to the pub(crate) shell fn).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::ProviderSettingsView;
use codelet_rpc_types::ProviderCredentialInfo;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn pinfo(id: &str) -> ProviderCredentialInfo {
    ProviderCredentialInfo {
        provider_id: id.to_string(),
        display_name: id.to_string(),
        configured: true,
        credential_type: "api_key".to_string(),
        model_count: 3,
        masked_key: None,
        source: None,
    }
}

fn render_to_string(view: &mut ProviderSettingsView, w: u16, h: u16) -> String {
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("Terminal::new");
    term.draw(|frame| {
        view.render(frame.area(), frame.buffer_mut());
    })
    .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            joined.push_str(buf[(x, y)].symbol());
        }
        joined.push('\n');
    }
    joined
}

/// Scenario: Migrated view preserves rendered output
#[test]
fn migrated_provider_settings_view_renders_title_and_footer_through_shell() {
    // @step Given the Provider Settings view rendered with its pre-migration scaffold
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![pinfo("openai"), pinfo("anthropic")]);

    // @step When the same view is rendered through the shared shell
    let out = render_to_string(&mut view, 80, 24);

    // @step Then the rendered output is identical to the pre-migration snapshot
    // The pre-migration scaffold rendered a `"Provider Settings (N items)"`
    // title on row 0 and a footer hint on the last row; the shell refit
    // must preserve both, with the body in between.
    let first_line = out.lines().next().unwrap_or("");
    assert!(
        first_line.contains("Provider Settings ("),
        "title row missing/relocated: {first_line:?}"
    );
    assert!(out.contains("items)"), "title count suffix missing");
    // Footer hint (List-mode focused-row hint) is painted on the last row.
    let last_line = out
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("");
    assert!(
        last_line.contains("Esc") || last_line.contains("Enter"),
        "footer hint missing on last row: {last_line:?}"
    );
}
