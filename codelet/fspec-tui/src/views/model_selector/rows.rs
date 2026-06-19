//! RPC-337 — row projection + body rendering for the full-screen
//! ModelSelector mode-view.
//!
//! Feature: spec/features/full-screen-model-selector.feature
//!
//! Reuses the `ModelSelectorRow` projection + header-skipping
//! navigation helpers from `components::model_selector_dialog_rows`
//! (re-scoped to `pub(crate)`), but provides its OWN full-width
//! row→Line builder with a proportional scrollbar, capability badge
//! colouring (TS order `[C] [R] [V] [cw]`), a green `(current)` marker,
//! and a bottom legend — the popup `build_dialog_rows` is NOT reused.

use std::collections::HashSet;

use codelet_rpc_types::{ModelEntry, ProviderInfo};
use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::components::model_selector_dialog_rows::ModelSelectorRow;

/// Bottom legend explaining the capability badges + the local-server profile
/// icon (full TS parity with `ModelSelectorView.tsx`).
pub(crate) const LEGEND: &str =
    "[R] Reasoning | [V] Vision | [C] Custom | 📁 Profile (local server)";

/// Footer hint for the mode-view (RPC-337 rule [12]).
pub(crate) const FOOTER: &str =
    "Enter Select | ←→ Expand/Collapse | / Filter | r Refresh | Esc Close";

/// Placeholder painted while providers have not loaded (or none exist).
pub(crate) const EMPTY_PLACEHOLDER: &str = "No providers available";

/// Build the flat row projection for the full-screen view. Provider
/// headers render with `▼` (expanded) / `▶` (collapsed); model rows are
/// emitted only for expanded providers. A non-empty `filter`
/// (case-insensitive) narrows model rows AND auto-expands every
/// provider so matches are visible; providers with no matching model
/// are dropped entirely.
///
/// Capability badges are appended in TS order `[C] [R] [V] [cw]`
/// (`is_custom` first), distinct from the dialog `build_rows` which
/// omits `[C]`.
pub(crate) fn build_view_rows(
    providers: &[ProviderInfo],
    expanded: &HashSet<String>,
    filter: &str,
) -> Vec<ModelSelectorRow> {
    let lower = filter.to_lowercase();
    let filtering = !lower.is_empty();
    let mut rows = Vec::with_capacity(providers.len() * 4);
    for provider in providers {
        let matching: Vec<&ModelEntry> = provider
            .models
            .iter()
            .filter(|m| {
                !filtering
                    || m.id.to_lowercase().contains(&lower)
                    || m.display_name.to_lowercase().contains(&lower)
            })
            .collect();
        // When filtering, drop providers with no matching models entirely.
        if filtering && matching.is_empty() {
            continue;
        }
        // A non-empty filter auto-expands every (surviving) provider so
        // matches are visible; otherwise honour the expanded set.
        let is_expanded = filtering || expanded.contains(&provider.key);
        let arrow = if is_expanded { '▼' } else { '▶' };
        rows.push(ModelSelectorRow {
            label: format!(
                "{arrow} {} ({} models)",
                provider.display_name,
                provider.models.len()
            ),
            badges: String::new(),
            selectable: false,
            provider_key: provider.key.clone(),
            model_id: String::new(),
            is_profile: provider.profile_name.is_some(),
            is_unreachable: provider.is_unreachable,
        });
        if !is_expanded {
            continue;
        }
        for model in matching {
            rows.push(ModelSelectorRow {
                label: model.display_name.clone(),
                badges: build_badges(model),
                selectable: true,
                provider_key: provider.key.clone(),
                model_id: model.id.clone(),
                is_profile: false,
                is_unreachable: false,
            });
        }
    }
    rows
}

/// Append capability badges in TS order `[C] [R] [V] [cw]`.
fn build_badges(model: &ModelEntry) -> String {
    let mut badges = String::new();
    if model.is_custom {
        badges.push_str(" [C]");
    }
    if model.supports_reasoning {
        badges.push_str(" [R]");
    }
    if model.supports_vision {
        badges.push_str(" [V]");
    }
    if model.context_window > 0 {
        let cw = if model.context_window >= 1_000 {
            format!("{}k", model.context_window / 1_000)
        } else {
            model.context_window.to_string()
        };
        badges.push_str(&format!(" [{cw}]"));
    }
    badges
}

/// First selectable row index, or 0 when none are selectable.
pub(crate) fn first_selectable_or_zero(rows: &[ModelSelectorRow]) -> usize {
    rows.iter().position(|r| r.selectable).unwrap_or(0)
}

/// Per-token badge style: `[C]` yellow, `[R]` magenta, `[V]` blue,
/// everything else (the `[cw]` context-window token) gray.
pub(crate) fn badge_token_style(token: &str) -> Style {
    let color = match token {
        "[C]" => Color::Yellow,
        "[R]" => Color::Magenta,
        "[V]" => Color::Blue,
        _ => Color::Gray,
    };
    Style::default().fg(color)
}

/// Paint the body region: placeholder when empty, otherwise a windowed
/// list with a proportional scrollbar, dim `↑`/`↓` overflow indicators
/// on the first/last visible rows, coloured badges, a green `(current)`
/// marker on the active-session model, and the legend on the bottom row.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_body(
    area: Rect,
    buf: &mut Buffer,
    rows: &[ModelSelectorRow],
    selected_index: usize,
    scroll_offset: usize,
    current_model_id: Option<&str>,
) {
    if area.height == 0 {
        return;
    }
    // Reserve the bottom row for the legend.
    let legend_y = area.y + area.height - 1;
    let legend_row = Rect {
        x: area.x,
        y: legend_y,
        width: area.width,
        height: 1,
    };
    Paragraph::new(Span::styled(
        LEGEND,
        Style::default().add_modifier(Modifier::DIM),
    ))
    .render(legend_row, buf);

    let list_height = area.height.saturating_sub(1);
    if list_height == 0 {
        return;
    }

    if rows.is_empty() {
        let mid_y = area.y.saturating_add(list_height / 2);
        let row = Rect {
            x: area.x,
            y: mid_y,
            width: area.width,
            height: 1,
        };
        Paragraph::new(EMPTY_PLACEHOLDER)
            .alignment(Alignment::Center)
            .render(row, buf);
        return;
    }

    let visible_rows = list_height as usize;
    let total = rows.len();
    let so = scroll_offset.min(total.saturating_sub(1));
    let up_arrow = so > 0;
    let down_arrow = so + visible_rows < total;
    let end = (so + visible_rows).min(total);
    // Scrollbar column (drawn only when the list overflows the viewport).
    let overflow = total > visible_rows;
    let list_width = if overflow {
        area.width.saturating_sub(1)
    } else {
        area.width
    };

    for (rel, abs_i) in (so..end).enumerate() {
        let y = area.y + rel as u16;
        let row_area = Rect {
            x: area.x,
            y,
            width: list_width,
            height: 1,
        };
        let is_first_visible = rel == 0;
        let is_last_visible = rel + 1 == end - so;
        if up_arrow && is_first_visible {
            Paragraph::new(Span::styled(
                "↑",
                Style::default().add_modifier(Modifier::DIM),
            ))
            .render(row_area, buf);
            continue;
        }
        if down_arrow && is_last_visible {
            Paragraph::new(Span::styled(
                "↓",
                Style::default().add_modifier(Modifier::DIM),
            ))
            .render(row_area, buf);
            continue;
        }
        render_row(
            row_area,
            buf,
            &rows[abs_i],
            abs_i == selected_index,
            current_model_id,
        );
    }

    if overflow {
        render_scrollbar(
            Rect {
                x: area.x + list_width,
                y: area.y,
                width: 1,
                height: list_height,
            },
            buf,
            so,
            visible_rows,
            total,
        );
    }
}

/// Paint one row: marker + label + coloured badges + optional green
/// `(current)` marker. Selected rows use the REVERSED highlight.
fn render_row(
    area: Rect,
    buf: &mut Buffer,
    row: &ModelSelectorRow,
    is_selected: bool,
    current_model_id: Option<&str>,
) {
    if !row.selectable {
        // RPC-338: provider header rendering (incl. 📁 / unreachable markers)
        // lives in `header.rs` to keep this file under the source-shape budget.
        super::header::render_header_row(area, buf, row, is_selected);
        return;
    }
    let base = if is_selected {
        Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default()
    };
    let marker = if is_selected { "▸ " } else { "  " };
    let mut spans: Vec<Span<'static>> = vec![
        Span::styled(format!(" {marker}"), base),
        Span::styled(row.label.clone(), base),
    ];
    // Badges — coloured per token (DIM is dropped on the selected row so
    // the colour is legible against the inverse highlight).
    for token in row.badges.split_whitespace() {
        let style = if is_selected {
            base
        } else {
            badge_token_style(token).add_modifier(Modifier::DIM)
        };
        spans.push(Span::styled(format!(" {token}"), style));
    }
    if current_model_id == Some(row.model_id.as_str()) {
        let style = if is_selected {
            base
        } else {
            Style::default().fg(Color::Green)
        };
        spans.push(Span::styled(" (current)".to_string(), style));
    }
    Paragraph::new(Line::from(spans)).render(area, buf);
}

/// Draw a proportional scrollbar thumb `■` over the track `│`.
fn render_scrollbar(
    area: Rect,
    buf: &mut Buffer,
    scroll_offset: usize,
    visible: usize,
    total: usize,
) {
    let h = area.height as usize;
    if h == 0 || total == 0 {
        return;
    }
    let thumb_h = ((visible * h) / total).max(1);
    let thumb_pos = (scroll_offset * h) / total;
    for i in 0..h {
        let is_thumb = i >= thumb_pos && i < thumb_pos + thumb_h;
        let sym = if is_thumb { "■" } else { "│" };
        let row = Rect {
            x: area.x,
            y: area.y + i as u16,
            width: 1,
            height: 1,
        };
        Paragraph::new(Span::styled(
            sym,
            Style::default().add_modifier(Modifier::DIM),
        ))
        .render(row, buf);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn model(id: &str, reasoning: bool, vision: bool, cw: u32, custom: bool) -> ModelEntry {
        ModelEntry {
            id: id.to_string(),
            display_name: id.to_string(),
            context_window: cw,
            supports_reasoning: reasoning,
            supports_vision: vision,
            is_custom: custom,
        }
    }

    fn provider(key: &str, models: Vec<ModelEntry>) -> ProviderInfo {
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
    fn profile_provider(
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
    fn render_to_grid(
        rows: &[ModelSelectorRow],
        selected: usize,
    ) -> (String, Vec<(String, Color)>) {
        let mut term =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
        term.draw(|f| render_body(f.area(), f.buffer_mut(), rows, selected, 0, None))
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

    fn expanded_set(keys: &[&str]) -> HashSet<String> {
        keys.iter().map(|k| (*k).to_string()).collect()
    }

    /// Scenario: Model rows display capability badges
    #[test]
    fn custom_model_row_shows_badges_in_ts_order() {
        // @step Given the model selector lists a custom model supporting reasoning and vision with a 200k context window
        let providers = vec![provider(
            "openai",
            vec![model("gpt", true, true, 200_000, true)],
        )];

        // @step When the row is rendered while unselected
        let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");
        let model_row = rows.iter().find(|r| r.selectable).expect("model row");

        // @step Then it shows the badges "[C]", "[R]", "[V]" and "[200k]" in that order
        assert_eq!(model_row.badges, " [C] [R] [V] [200k]");

        // @step And the "[C]" badge is yellow, "[R]" magenta, "[V]" blue and "[200k]" gray
        assert_eq!(badge_token_style("[C]").fg, Some(Color::Yellow));
        assert_eq!(badge_token_style("[R]").fg, Some(Color::Magenta));
        assert_eq!(badge_token_style("[V]").fg, Some(Color::Blue));
        assert_eq!(badge_token_style("[200k]").fg, Some(Color::Gray));
    }

    /// Scenario: The body renders the capability legend
    #[test]
    fn body_renders_capability_legend_on_bottom_row() {
        // @step Given the model selector is open
        let providers = vec![provider(
            "openai",
            vec![model("gpt", false, false, 8_000, false)],
        )];
        let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");

        // @step When the body is rendered
        let mut term =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
        term.draw(|f| render_body(f.area(), f.buffer_mut(), &rows, 1, 0, None))
            .expect("draw");
        let buf = term.backend().buffer().clone();
        let mut joined = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                joined.push_str(buf[(x, y)].symbol());
            }
            joined.push('\n');
        }

        // @step Then a legend line "[R] Reasoning | [V] Vision | [C] Custom" appears at the bottom of the body
        // (The 📁 segment is a wide-glyph; assert the badge prefix verbatim
        //  and the profile segment text separately — see RPC-338.)
        assert!(
            joined.contains("[R] Reasoning | [V] Vision | [C] Custom"),
            "legend missing: {joined}"
        );
    }

    /// Scenario: Providers still loading shows a placeholder
    #[test]
    fn empty_rows_render_placeholder() {
        // @step Given the model selector has opened but providers have not loaded
        let rows: Vec<ModelSelectorRow> = build_view_rows(&[], &expanded_set(&[]), "");

        // @step When the body is rendered
        let mut term =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
        term.draw(|f| render_body(f.area(), f.buffer_mut(), &rows, 0, 0, None))
            .expect("draw");
        let buf = term.backend().buffer().clone();
        let mut joined = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                joined.push_str(buf[(x, y)].symbol());
            }
            joined.push('\n');
        }

        // @step Then it shows the "No providers available" placeholder
        assert!(
            joined.contains(EMPTY_PLACEHOLDER),
            "placeholder missing: {joined}"
        );
        // @step And the placeholder is replaced once the provider list arrives
        let loaded = build_view_rows(
            &[provider(
                "openai",
                vec![model("gpt", false, false, 8_000, false)],
            )],
            &expanded_set(&["openai"]),
            "",
        );
        assert!(loaded.iter().any(|r| r.selectable));
    }

    /// Scenario: Expanding and collapsing a provider group
    #[test]
    fn collapsed_provider_hides_models_expanded_shows_them() {
        // @step Given the model selector shows an expanded provider group
        let providers = vec![provider(
            "openai",
            vec![model("gpt", false, false, 8_000, false)],
        )];
        let expanded = build_view_rows(&providers, &expanded_set(&["openai"]), "");
        assert!(
            expanded.iter().any(|r| r.selectable),
            "expanded shows models"
        );
        assert!(
            expanded[0].label.starts_with('▼'),
            "expanded header arrow ▼"
        );

        // @step When I press the left arrow on the provider group
        // @step Then the group collapses and hides its model rows
        let collapsed = build_view_rows(&providers, &expanded_set(&[]), "");
        assert!(
            !collapsed.iter().any(|r| r.selectable),
            "collapsed hides models"
        );
        assert!(
            collapsed[0].label.starts_with('▶'),
            "collapsed header arrow ▶"
        );
    }

    /// Scenario: Filtering narrows the model list
    #[test]
    fn filter_narrows_models_and_clearing_restores() {
        // @step Given the model selector is showing all providers and models
        let providers = vec![provider(
            "openai",
            vec![
                model("gpt-4o", false, false, 8_000, false),
                model("o3-mini", true, false, 8_000, false),
            ],
        )];
        let all = build_view_rows(&providers, &expanded_set(&["openai"]), "");
        assert_eq!(all.iter().filter(|r| r.selectable).count(), 2);

        // @step When I press "/" and type filter text
        let filtered = build_view_rows(&providers, &expanded_set(&[]), "o3");

        // @step Then the list narrows to models matching the filter
        let model_rows: Vec<_> = filtered.iter().filter(|r| r.selectable).collect();
        assert_eq!(model_rows.len(), 1);
        assert!(model_rows[0].model_id.contains("o3"));

        // @step And clearing the filter restores the full list
        let restored = build_view_rows(&providers, &expanded_set(&["openai"]), "");
        assert_eq!(restored.iter().filter(|r| r.selectable).count(), 2);
    }

    /// Scenario: The active session model shows a current marker
    #[test]
    fn current_model_row_shows_green_current_marker() {
        // @step Given the model selector lists a model whose id matches the active session model
        let providers = vec![provider(
            "openai",
            vec![model("gpt-4o", false, false, 8_000, false)],
        )];
        let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");

        // @step When the list is rendered
        // (selection rests on the header so the current model row renders
        //  unselected — the green (current) marker only shows when the row
        //  is not inverse-highlighted, matching ModelSelectorView.tsx)
        let mut term =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, 12)).expect("term");
        term.draw(|f| render_body(f.area(), f.buffer_mut(), &rows, 0, 0, Some("gpt-4o")))
            .expect("draw");
        let buf = term.backend().buffer().clone();
        let mut joined = String::new();
        let mut found_green_current = false;
        for y in 0..buf.area.height {
            let mut line = String::new();
            for x in 0..buf.area.width {
                let cell = &buf[(x, y)];
                line.push_str(cell.symbol());
            }
            if line.contains("(current)") {
                // verify at least one cell on this row is green
                for x in 0..buf.area.width {
                    if buf[(x, y)].fg == Color::Green {
                        found_green_current = true;
                    }
                }
            }
            joined.push_str(&line);
            joined.push('\n');
        }

        // @step Then that model row shows a green "(current)" marker
        assert!(
            joined.contains("(current)"),
            "current marker text missing: {joined}"
        );
        assert!(found_green_current, "current marker not green");
    }

    // ========================================================================
    // RPC-338: profile (📁) sections + red (unreachable) markers + legend
    // segment. Feature: spec/features/model-selector-profile-sections.feature
    // ========================================================================

    /// Scenario: A reachable profile section renders the folder icon and qualified label
    #[test]
    fn reachable_profile_header_shows_folder_icon_and_label() {
        // @step Given the model selector is showing a provider list
        // @step And a profile section with profile_name "my-profile", display_name "openai: my-profile", 3 models, and is_unreachable false
        let providers = vec![profile_provider(
            "openai:my-profile",
            "openai: my-profile",
            "my-profile",
            false,
            vec![
                model("a", false, false, 8_000, false),
                model("b", false, false, 8_000, false),
                model("c", false, false, 8_000, false),
            ],
        )];
        let rows = build_view_rows(&providers, &expanded_set(&["openai:my-profile"]), "");
        let header = &rows[0];
        assert!(!header.selectable, "profile header is non-selectable");
        assert!(header.is_profile, "header flagged as profile");
        assert!(!header.is_unreachable, "reachable header");

        // @step When the provider header row is rendered while not selected
        let (joined, cells) = render_to_grid(&rows, first_selectable_or_zero(&rows));

        // @step Then a magenta 📁 icon appears after the expand arrow and before the label
        let folder_is_magenta = cells
            .iter()
            .any(|(sym, fg)| sym == "📁" && *fg == Color::Magenta);
        assert!(folder_is_magenta, "magenta 📁 missing: {joined}");
        let arrow_pos = joined.find('▼').expect("arrow");
        let folder_pos = joined.find('📁').expect("folder");
        let label_pos = joined.find("openai: my-profile").expect("label");
        assert!(
            arrow_pos < folder_pos && folder_pos < label_pos,
            "order arrow<📁<label"
        );

        // @step And the header text includes "openai: my-profile (3 models)"
        assert!(
            joined.contains("openai: my-profile (3 models)"),
            "label: {joined}"
        );

        // @step And the header shows no "(unreachable)" marker
        assert!(
            !joined.contains("(unreachable)"),
            "unexpected unreachable: {joined}"
        );
    }

    /// Scenario: An unreachable profile header renders a red marker and is never hidden
    #[test]
    fn unreachable_profile_header_shows_red_marker_and_is_present() {
        // @step Given the model selector is showing a provider list
        // @step And an unreachable profile section with profile_name "down-profile", display_name "openai: down-profile", 0 models, and is_unreachable true
        let providers = vec![profile_provider(
            "openai:down-profile",
            "openai: down-profile",
            "down-profile",
            true,
            Vec::new(),
        )];
        let rows = build_view_rows(&providers, &expanded_set(&["openai:down-profile"]), "");

        // @step When the provider header row is rendered while not selected
        // (selection parked off the header so its markers render coloured)
        let (joined, cells) = render_to_grid(&rows, 99);

        // @step Then a magenta 📁 icon appears after the expand arrow and before the label
        assert!(
            cells
                .iter()
                .any(|(sym, fg)| sym == "📁" && *fg == Color::Magenta),
            "magenta 📁 missing: {joined}"
        );

        // @step And a red " (unreachable)" marker appears after the "(0 models)" count
        let count_pos = joined.find("(0 models)").expect("count");
        let unreachable_pos = joined.find("(unreachable)").expect("unreachable marker");
        assert!(count_pos < unreachable_pos, "marker after count");
        let red_unreachable = cells.iter().any(|(_, fg)| *fg == Color::Red);
        assert!(
            red_unreachable,
            "no red cell for unreachable marker: {joined}"
        );

        // @step And the header text contains no duplicated "(unreachable)" before the count
        assert_eq!(
            joined.matches("(unreachable)").count(),
            1,
            "exactly one (unreachable) marker: {joined}"
        );

        // @step And the header row remains non-selectable
        assert!(!rows[0].selectable, "header non-selectable");

        // @step And the header row is still present in the list
        assert!(
            rows.iter().any(|r| r.is_profile && r.is_unreachable),
            "header present"
        );
    }

    /// Scenario: A selected profile header renders its markers in the selected style
    #[test]
    fn selected_profile_header_renders_markers_in_selected_style() {
        // @step Given the model selector is showing a provider list
        // @step And a profile section with profile_name set and is_unreachable true
        let providers = vec![profile_provider(
            "openai:down-profile",
            "openai: down-profile",
            "down-profile",
            true,
            Vec::new(),
        )];
        let rows = build_view_rows(&providers, &expanded_set(&["openai:down-profile"]), "");

        // @step When the provider header row is rendered while selected
        let (joined, cells) = render_to_grid(&rows, 0);

        // @step Then the 📁 icon is rendered in the selected highlight style rather than magenta
        let folder_magenta = cells
            .iter()
            .any(|(sym, fg)| sym == "📁" && *fg == Color::Magenta);
        assert!(!folder_magenta, "selected 📁 must not be magenta: {joined}");

        // @step And the " (unreachable)" marker is rendered in the selected highlight style rather than red
        let unreachable_red = cells.iter().any(|(_, fg)| *fg == Color::Red);
        assert!(
            !unreachable_red,
            "selected (unreachable) must not be red: {joined}"
        );
    }

    /// Scenario: A cloud provider header renders without profile or unreachable markers
    #[test]
    fn cloud_provider_header_has_no_profile_or_unreachable_markers() {
        // @step Given the model selector is showing a provider list
        // @step And a cloud provider header with profile_name None and is_unreachable false
        let providers = vec![provider(
            "anthropic",
            vec![model("claude", false, false, 200_000, false)],
        )];
        let rows = build_view_rows(&providers, &expanded_set(&["anthropic"]), "");
        assert!(!rows[0].is_profile, "cloud header not a profile");
        assert!(!rows[0].is_unreachable, "cloud header reachable");

        // @step When the provider header row is rendered
        let (joined, _cells) = render_to_grid(&rows, first_selectable_or_zero(&rows));
        // The body legend row also contains 📁; scope the assertions to the
        // header row (first rendered line) only.
        let header_line = joined.lines().next().unwrap_or("");

        // @step Then the header shows no 📁 prefix
        assert!(
            !header_line.contains('📁'),
            "cloud header must not show 📁: {header_line}"
        );

        // @step And the header shows no "(unreachable)" marker
        assert!(
            !header_line.contains("(unreachable)"),
            "cloud header must not show unreachable: {header_line}"
        );
    }

    /// Scenario: The body legend includes the profile segment
    #[test]
    fn body_legend_includes_profile_segment() {
        // @step Given the model selector body is rendered
        let providers = vec![provider(
            "openai",
            vec![model("gpt", false, false, 8_000, false)],
        )];
        let rows = build_view_rows(&providers, &expanded_set(&["openai"]), "");
        let (joined, _cells) = render_to_grid(&rows, first_selectable_or_zero(&rows));

        // @step Then the legend line reads "[R] Reasoning | [V] Vision | [C] Custom | 📁 Profile (local server)"
        // Const parity is byte-exact; the rendered buffer pads the wide 📁
        // glyph, so assert the textual segments are present on the bottom row.
        assert_eq!(
            LEGEND,
            "[R] Reasoning | [V] Vision | [C] Custom | 📁 Profile (local server)"
        );
        let legend_line = joined.lines().last().unwrap_or("");
        assert!(
            legend_line.contains("[R] Reasoning | [V] Vision | [C] Custom"),
            "badge legend missing: {legend_line}"
        );
        assert!(
            legend_line.contains("📁") && legend_line.contains("Profile (local server)"),
            "profile legend segment missing: {legend_line}"
        );
    }
}
