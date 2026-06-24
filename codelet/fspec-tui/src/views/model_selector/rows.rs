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

/// Footer hint for the mode-view (RPC-337 rule [12]).
pub(crate) const FOOTER: &str =
    "Enter Select | ←→ Expand/Collapse | / Filter | r Refresh | Esc Close";

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
            is_custom: false,
            profile_name: provider.profile_name.clone(),
            context_window: 0,
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
                model_id: model.id.clone(),
                is_profile: false,
                is_unreachable: false,
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

/// Index of the first selectable row whose `model_id` matches
/// `current_model_id`. Returns `None` when there is no current model or no
/// matching selectable row. The `selectable` guard ensures a non-selectable
/// header row (empty `model_id`) can never be matched.
pub(crate) fn index_of_model(
    rows: &[ModelSelectorRow],
    current_model_id: Option<&str>,
) -> Option<usize> {
    let target = current_model_id?;
    rows.iter()
        .position(|r| r.selectable && super::model_id::model_ids_match(&r.model_id, target))
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
#[path = "rows_tests_profile.rs"]
mod rows_tests_profile;
