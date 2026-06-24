//! Shared test fixtures for the `rows` test suite (PROV-107).
//! `pub(crate)` so `rows_tests` and `rows_tests_profile` can share them.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

pub(crate) fn model(id: &str, reasoning: bool, vision: bool, cw: u32, custom: bool) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: cw,
        supports_reasoning: reasoning,
        supports_vision: vision,
        is_custom: custom,
    }
}

pub(crate) fn provider(key: &str, models: Vec<ModelEntry>) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: key.to_string(),
        models,
        profile_name: None,
        is_unreachable: false,
    }
}

/// A local-server profile section: profile_name = Some, profile-qualified
/// display_name, optional unreachable flag. Mirrors the wire shape that
/// RPC-338 adds to `ProviderInfo`.
pub(crate) fn profile_provider(
    key: &str,
    display_name: &str,
    profile_name: &str,
    is_unreachable: bool,
    models: Vec<ModelEntry>,
) -> ProviderInfo {
    ProviderInfo {
        key: key.to_string(),
        display_name: display_name.to_string(),
        models,
        profile_name: Some(profile_name.to_string()),
        is_unreachable,
    }
}

/// Render the full body to a `(symbols, fg-color-per-cell)` grid so tests
/// can assert both glyphs and their foreground colours.
pub(crate) fn render_to_grid(
    rows: &[ModelSelectorRow],
    selected: usize,
) -> (String, Vec<(String, Color)>) {
    let mut term =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
    term.draw(|f| render_body(f.area(), f.buffer_mut(), rows, true, selected, 0, None))
        .expect("draw");
    let buf = term.backend().buffer().clone();
    let mut joined = String::new();
    let mut cells: Vec<(String, Color)> = Vec::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            joined.push_str(cell.symbol());
            cells.push((cell.symbol().to_string(), cell.fg));
        }
        joined.push('\n');
    }
    (joined, cells)
}

pub(crate) fn expanded_set(keys: &[&str]) -> HashSet<String> {
    keys.iter().map(|k| (*k).to_string()).collect()
}
