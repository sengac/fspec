//! PROV-135 — Profile form placeholder hints for empty numeric/threshold fields.
//!
//! Feature: spec/features/provider-settings-profile-form-placeholders.feature
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment.
//!
//! Strategy: construct a `ProviderSettingsView` in CreateProfile mode with a
//! chosen `ProfileForm`, render the whole view into a ratatui `Buffer` via the
//! public `render` entry (which routes through `body_render` →
//! `profile_form_render::render_form`), then inspect the rendered field rows
//! for the dim placeholder text. Scenario 5 is a pure `build_definition()`
//! unit check (no rendering) proving placeholders are never persisted.
//!
//! RED PHASE: `field_line` currently renders a generic `(empty)` hint for every
//! empty field, so the per-field placeholder assertions (scenarios 1–3) FAIL
//! until the render-layer fix lands. Scenarios 4 and 5 already pass and act as
//! regression guards. No production code is touched by this file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::provider_settings::profile_form::ProfileForm;
use codelet_fspec_tui::views::{ProviderSettingsMode, ProviderSettingsView};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

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

/// Build a view sitting in CreateProfile mode with the given form.
fn create_view(form: ProfileForm) -> ProviderSettingsView {
    let mut view = ProviderSettingsView::new();
    view.mode = ProviderSettingsMode::CreateProfile {
        provider_id: "openai".to_string(),
        form,
    };
    view
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

/// Display column (x) where the substring `needle` begins on row `y`.
fn col_of(buf: &Buffer, y: u16, needle: &str) -> usize {
    let row = row_string(buf, y);
    row.find(needle)
        .map(|byte_idx| row[..byte_idx].chars().count())
        .unwrap_or_else(|| panic!("substring {needle:?} not found on row {y}: {row:?}"))
}

/// Assert every cell of `needle` on row `y` carries the DIM modifier.
fn assert_dim(buf: &Buffer, y: u16, needle: &str) {
    let start = col_of(buf, y, needle) as u16;
    let len = needle.chars().count() as u16;
    for x in start..start + len {
        assert!(
            buf[(x, y)].modifier.contains(Modifier::DIM),
            "placeholder cell {x} of {needle:?} on row {y} should carry the DIM modifier, got {:?}",
            buf[(x, y)].modifier
        );
    }
}

/// A create form past the name step, focused on `field_index`, with a real
/// name/base_url/api_key so the numeric rows are the only empty ones.
fn form_on_field(field_index: usize) -> ProfileForm {
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = "http://localhost:8888".to_string();
    form.api_key = "sk-test".to_string();
    form.field_index = field_index;
    form
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Empty Context Window field shows a dim placeholder
// ════════════════════════════════════════════════════════════════════════
#[test]
fn empty_context_window_field_shows_dim_placeholder() {
    // @step Given a new profile form is open with the Context Window field empty
    let mut view = create_view(form_on_field(2));

    // @step When the profile form is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then the Context Window row shows the placeholder "128000"
    let ry = find_row(&buf, "Context Window").expect("Context Window row should render");
    let row = row_string(&buf, ry);
    assert!(
        row.contains("128000"),
        "Context Window row should show placeholder '128000', got {row:?}"
    );

    // @step Then the placeholder is rendered with the dim modifier
    assert_dim(&buf, ry, "128000");
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Empty Max Output Tokens field shows a dim placeholder
// ════════════════════════════════════════════════════════════════════════
#[test]
fn empty_max_output_tokens_field_shows_dim_placeholder() {
    // @step Given a new profile form is open with the Max Output Tokens field empty
    let mut view = create_view(form_on_field(3));

    // @step When the profile form is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then the Max Output Tokens row shows the placeholder "16384"
    let ry = find_row(&buf, "Max Output Tokens").expect("Max Output Tokens row should render");
    let row = row_string(&buf, ry);
    assert!(
        row.contains("16384"),
        "Max Output Tokens row should show placeholder '16384', got {row:?}"
    );

    // @step Then the placeholder is rendered with the dim modifier
    assert_dim(&buf, ry, "16384");
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Empty Compaction Threshold field shows a dim placeholder
// ════════════════════════════════════════════════════════════════════════
#[test]
fn empty_compaction_threshold_field_shows_dim_placeholder() {
    // @step Given a new profile form is open with the Compaction Threshold field empty
    let mut view = create_view(form_on_field(4));

    // @step When the profile form is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then the Compaction Threshold row shows the placeholder "80% or 200000"
    let ry =
        find_row(&buf, "Compaction Threshold").expect("Compaction Threshold row should render");
    let row = row_string(&buf, ry);
    assert!(
        row.contains("80% or 200000"),
        "Compaction Threshold row should show placeholder '80% or 200000', got {row:?}"
    );

    // @step Then the placeholder is rendered with the dim modifier
    assert_dim(&buf, ry, "80% or 200000");
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: A field with a typed value shows the value not the placeholder
// ════════════════════════════════════════════════════════════════════════
#[test]
fn typed_value_shows_value_not_placeholder() {
    // @step Given a new profile form is open
    let mut form = form_on_field(3);

    // @step Given the Max Output Tokens field contains the typed value "8192"
    form.max_output_tokens = "8192".to_string();
    let mut view = create_view(form);

    // @step When the profile form is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then the Max Output Tokens row shows "8192"
    let ry = find_row(&buf, "Max Output Tokens").expect("Max Output Tokens row should render");
    let row = row_string(&buf, ry);
    assert!(
        row.contains("8192"),
        "Max Output Tokens row should show typed value '8192', got {row:?}"
    );

    // @step Then the Max Output Tokens row does not show the placeholder "16384"
    assert!(
        !row.contains("16384"),
        "typed row must not show the placeholder '16384', got {row:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Placeholder hints are never persisted into the saved profile
// ════════════════════════════════════════════════════════════════════════
#[test]
fn placeholder_hints_are_never_persisted() {
    // @step Given a new profile form is open with base URL and API key filled in
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.name = "local".to_string();
    form.base_url = "http://localhost:8888".to_string();
    form.api_key = "sk-test".to_string();

    // @step Given the Context Window, Max Output Tokens, and Compaction Threshold fields are left empty
    form.context_window = String::new();
    form.max_output_tokens = String::new();
    form.compaction_threshold = String::new();

    // @step When the profile definition is built from the form
    let def = form
        .build_definition()
        .expect("a form with name + base URL + api key should build a definition")
        .expect("form must build a definition");

    // @step Then the saved profile has no context window value
    assert_eq!(
        def.context_window, None,
        "empty Context Window must not persist a value"
    );

    // @step Then the saved profile has no max output tokens value
    assert_eq!(
        def.max_output_tokens, None,
        "empty Max Output Tokens must not persist a value"
    );

    // @step Then the saved profile has no compaction threshold type or value
    assert_eq!(
        def.compaction_threshold_type, None,
        "empty Compaction Threshold must not persist a type"
    );
    assert_eq!(
        def.compaction_threshold_value, None,
        "empty Compaction Threshold must not persist a value"
    );
}
