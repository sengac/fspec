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
use ratatui::style::{Color, Style};

use crate::components::model_selector_dialog_rows::ModelSelectorRow;

/// Bottom legend explaining the capability badges + the local-server profile
/// icon (full TS parity with `ModelSelectorView.tsx`).
pub(crate) const LEGEND: &str =
    "[R] Reasoning | [V] Vision | [C] Custom | 📁 Profile (local server)";

/// Footer hint for the browse mode-view (RPC-337 rule [12]; BUG-576 / AK-9:
/// every browse key, Tab and the custom-model keys included).
pub(crate) const FOOTER: &str = "Enter Select | ←→ Expand/Collapse | / Filter | c Copy | r Refresh | Tab Providers | a/e/d Custom model | Esc Close";

/// BUG-576 / AK-9: the footer while a filter is being typed, where Enter
/// applies the filter and Esc clears it.
pub(crate) const FILTER_FOOTER: &str = "Type to filter | Enter Apply | Esc Clear";

/// Placeholder painted once loading has COMPLETED but no models exist
/// (`loaded == true` with an empty projection). PROV-104 rules [9]/[10]:
/// an explicit no-models empty state, distinct from the loading indicator.
pub(crate) const EMPTY_PLACEHOLDER: &str = "No models available";

/// Indicator painted while the provider list has NOT yet finished loading
/// (`loaded == false`). PROV-104 rule [8]: a visible loading state instead
/// of a blank/inert list that is indistinguishable from "no models".
pub(crate) const LOADING_PLACEHOLDER: &str = "Loading models…";

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
                "{arrow} {} {}",
                provider.display_name,
                model_count_label(provider.models.len())
            ),
            badges: String::new(),
            selectable: false,
            provider_key: provider.key.clone(),
            provider_id: provider.provider_id.clone(),
            model_id: String::new(),
            is_profile: provider.profile_name.is_some(),
            is_unreachable: provider.is_unreachable,
            unavailable_reason: provider.unavailable_reason.clone(),
            is_custom: false,
            profile_name: provider.profile_name.clone(),
            context_window: None,
            supports_reasoning: false,
            supports_vision: false,
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
                provider_id: provider.provider_id.clone(),
                model_id: model.id.clone(),
                is_profile: false,
                is_unreachable: false,
                unavailable_reason: None,
                is_custom: model.is_custom,
                profile_name: provider.profile_name.clone(),
                context_window: model.context_window,
                supports_reasoning: model.supports_reasoning,
                supports_vision: model.supports_vision,
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
    // PROV-169: no size badge when the context window is unavailable.
    if let Some(window) = model.context_window {
        let cw = if window >= 1_000 {
            format!("{}k", window / 1_000)
        } else {
            window.to_string()
        };
        badges.push_str(&format!(" [{cw}]"));
    }
    badges
}

/// Index of the first selectable row of the current `(section, model)`.
/// BUG-576: the section and the model must both match, so the same model id
/// in another section is never taken for the current one. `None` when there
/// is no current model or no matching selectable row.
pub(crate) fn index_of_model(
    rows: &[ModelSelectorRow],
    current: Option<(&str, &str)>,
) -> Option<usize> {
    current?;
    rows.iter().position(|r| is_current(r, current))
}

/// Whether `row` is the selectable row of the current `(section, model)`.
pub(crate) fn is_current(row: &ModelSelectorRow, current: Option<(&str, &str)>) -> bool {
    current.is_some_and(|(section, model)| {
        row.selectable
            && row.provider_key == section
            && super::model_id::model_ids_match(&row.model_id, model)
    })
}

/// PROV-127: pluralize a provider header's model count as a parenthetical
/// suffix. Renders `(1 model)` for a single model and `(N models)` otherwise
/// (including `(0 models)`), matching the TS reference selector labels. Shared
/// by the full-screen header rows and the `state.rs` title so the singular /
/// plural rule lives in exactly one place.
pub(crate) fn model_count_label(count: usize) -> String {
    let noun = if count == 1 { "model" } else { "models" };
    format!("({count} {noun})")
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

#[path = "rows_render.rs"]
mod rows_render;
pub(crate) use rows_render::render_body;

#[cfg(test)]
#[path = "rows_test_support.rs"]
mod rows_test_support;

#[cfg(test)]
#[path = "rows_tests.rs"]
mod rows_tests;

#[cfg(test)]
#[path = "rows_tests_header_count.rs"]
mod rows_tests_header_count;

#[cfg(test)]
#[path = "rows_tests_profile.rs"]
mod rows_tests_profile;

#[cfg(test)]
#[path = "rows_tests_selection_style.rs"]
mod rows_tests_selection_style;
