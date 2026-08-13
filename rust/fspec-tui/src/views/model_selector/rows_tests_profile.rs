//! PROV-107 — RPC-338 profile-section render tests
//! (folder icon, unreachable marker, selected style, legend segment).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::rows_test_support::*;
use super::*;

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
    let (joined, cells) = render_to_grid(
        &rows,
        crate::components::model_selector_dialog_rows::first_selectable(&rows),
    );

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
    let (joined, _cells) = render_to_grid(
        &rows,
        crate::components::model_selector_dialog_rows::first_selectable(&rows),
    );
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
    let (joined, _cells) = render_to_grid(
        &rows,
        crate::components::model_selector_dialog_rows::first_selectable(&rows),
    );

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
