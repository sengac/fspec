//! BUG-160 — richer-match types + snippet logic for WorkUnitSearchDialog.
//!
//! Feature: spec/features/board-search-dialog-result-snippet.feature
//!
//! Extracted from `work_unit_search_dialog.rs` so the parent file stays
//! under the 300-LoC source-shape budget. `SearchMatch` pairs the
//! work-unit id with the mode-aware snippet text (title in Id/Title
//! mode, description in Description mode, title fallback when the unit
//! has no description) that `filter_work_units` threads into the row
//! builder. Pure; no state.

use codelet_rpc_types::WorkUnitInfo;

/// The three search modes cycled by Tab (BOARD-022).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    /// Match the work-unit id (default).
    #[default]
    Id,
    /// Match the work-unit title.
    Title,
    /// Match the work-unit description (units without one never match).
    Description,
}

impl SearchMode {
    /// Next mode in the Tab cycle (id → title → description → id).
    pub fn next(self) -> SearchMode {
        match self {
            SearchMode::Id => SearchMode::Title,
            SearchMode::Title => SearchMode::Description,
            SearchMode::Description => SearchMode::Id,
        }
    }

    /// Short label shown in the dialog title row.
    pub fn label(self) -> &'static str {
        match self {
            SearchMode::Id => "id",
            SearchMode::Title => "title",
            SearchMode::Description => "description",
        }
    }
}

/// One richer match: the work-unit `id` plus the mode-aware `snippet`
/// text the dialog paints next to it (BUG-160).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// The matched work-unit id (selection / scroll math key).
    pub id: String,
    /// The dimmed snippet text shown after the id (title or description).
    pub snippet: String,
}

/// Case-insensitive substring filter over a work-units snapshot. Pure +
/// unit-tested (proptest in `mod tests`). Returns matching
/// [`SearchMatch`] pairs in board order. An empty query matches every
/// unit in Id/Title mode; in Description mode only units that HAVE a
/// description match.
pub fn filter_work_units(
    units: &[WorkUnitInfo],
    mode: SearchMode,
    query: &str,
) -> Vec<SearchMatch> {
    let q = query.to_lowercase();
    units
        .iter()
        .filter(|u| match mode {
            SearchMode::Id => u.id.to_lowercase().contains(&q),
            SearchMode::Title => u.title.to_lowercase().contains(&q),
            SearchMode::Description => u
                .description
                .as_deref()
                .is_some_and(|d| d.to_lowercase().contains(&q)),
        })
        .map(|u| SearchMatch {
            id: u.id.clone(),
            snippet: snippet_for(u, mode),
        })
        .collect()
}

/// BUG-160: the mode-aware snippet for one unit — the description in
/// Description mode, otherwise the title (which doubles as the fallback
/// when a unit has no description, so the snippet is never empty).
pub fn snippet_for(unit: &WorkUnitInfo, mode: SearchMode) -> String {
    match mode {
        SearchMode::Description => unit
            .description
            .clone()
            .unwrap_or_else(|| unit.title.clone()),
        SearchMode::Id | SearchMode::Title => unit.title.clone(),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;
    use proptest::proptest;

    fn wu(id: &str, title: &str, description: Option<&str>) -> WorkUnitInfo {
        WorkUnitInfo {
            id: id.to_string(),
            title: title.to_string(),
            work_type: "story".to_string(),
            status: "backlog".to_string(),
            description: description.map(str::to_string),
            estimate: None,
            epic: None,
            attachments: Vec::new(),
            last_state_change_at: None,
        }
    }

    proptest! {
        // BOARD-022 / BUG-160: filter_work_units invariants — every
        // returned id exists in the input, the empty query is a superset
        // of any filtered result, and matching is case-insensitive.
        #[test]
        fn filter_work_units_invariants(
            id in proptest::string::string_regex("[A-Z]{1,4}-[0-9]{1,3}").unwrap(),
            title in proptest::string::string_regex("[a-z]{1,8}").unwrap(),
            query in proptest::string::string_regex("[a-z]{0,4}").unwrap(),
            as_title in proptest::arbitrary::any::<bool>(),
        ) {
            let mode = if as_title {
                SearchMode::Title
            } else {
                SearchMode::Id
            };
            let units = vec![wu(&id, &title, Some(&title))];
            let matches = filter_work_units(&units, mode, &query);
            // Every returned id exists in the input.
            for m in &matches {
                assert!(units.iter().any(|u| u.id == m.id));
            }
            // Case-insensitivity: uppercasing the query gives the same set.
            let upper = filter_work_units(&units, mode, query.to_uppercase().as_str());
            assert_eq!(matches, upper);
            // Empty query is a superset of the filtered result.
            let all = filter_work_units(&units, mode, "");
            for m in &matches {
                assert!(all.iter().any(|a| a.id == m.id));
            }
        }
    }

    #[test]
    fn description_mode_never_matches_units_without_a_description() {
        let units = vec![wu("NO-DESC-1", "some title", None)];
        assert_eq!(
            filter_work_units(&units, SearchMode::Description, "some"),
            Vec::<SearchMatch>::new()
        );
        assert_eq!(
            filter_work_units(&units, SearchMode::Description, ""),
            Vec::<SearchMatch>::new()
        );
    }

    #[test]
    fn mode_next_cycles_id_title_description_id() {
        assert_eq!(SearchMode::Id.next(), SearchMode::Title);
        assert_eq!(SearchMode::Title.next(), SearchMode::Description);
        assert_eq!(SearchMode::Description.next(), SearchMode::Id);
    }
}
