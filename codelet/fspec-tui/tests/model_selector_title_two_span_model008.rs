// Feature: spec/features/model-selector-title.feature
//
//! MODEL-008 — Model count in title renders in dim two-span style.
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment.
//!
//! Strategy (per the RPC-350 provider-title analog,
//! `provider_settings_list_mode_parity_rpc350.rs`): render the full
//! `ModelSelectorView` into a ratatui `TestBackend`/`Buffer` and assert
//! cell-level fg/modifier styling across the title row's name and count
//! spans. No async, no NAPI — a real view + real `ProviderInfo`/`ModelEntry`
//! rows seeded through the public `set_providers` API.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::ModelSelectorView;
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

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

fn render_to_buffer(view: &mut ModelSelectorView) -> Buffer {
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

fn cell_fg(buf: &Buffer, x: u16, y: u16) -> Color {
    buf[(x, y)].fg
}

fn cell_mod(buf: &Buffer, x: u16, y: u16) -> Modifier {
    buf[(x, y)].modifier
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: 200_000,
        supports_reasoning: true,
        supports_vision: true,
        is_custom: false,
    }
}

/// One provider carrying exactly `n` models → `total_model_count()` == n, so
/// the browse-list title reads "Select Model (n models)".
fn view_with_models(n: usize) -> ModelSelectorView {
    let ids: Vec<String> = (0..n).map(|i| format!("m{i}")).collect();
    let models: Vec<ModelEntry> = ids.iter().map(|id| model(id)).collect();
    let provider = ProviderInfo {
        key: "openai".to_string(),
        display_name: "openai".to_string(),
        models,
        profile_name: None,
        is_unreachable: false,
    };
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![provider]);
    v
}

/// Assert every cell of `needle` on row `ty` is dim DarkGray (the two-span
/// count treatment) and NOT the flat title/name style.
fn assert_count_is_dark_gray(buf: &Buffer, ty: u16, needle: &str) {
    let start = col_of(buf, ty, needle) as u16;
    let len = needle.chars().count() as u16;
    for x in start..start + len {
        assert_eq!(
            cell_fg(buf, x, ty),
            Color::DarkGray,
            "count cell {x} of {needle:?} should be DarkGray, got fg={:?} mod={:?}",
            cell_fg(buf, x, ty),
            cell_mod(buf, x, ty)
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Model browse-list title renders count in dim two-span style
// ════════════════════════════════════════════════════════════════════════
#[test]
fn browse_list_title_renders_count_in_dim_two_span_style() {
    // @step Given the model selector is in browse mode with 12 models available
    let mut view = view_with_models(12);

    // @step When the browse list is rendered
    let buf = render_to_buffer(&mut view);
    let ty = find_row(&buf, "Select Model (12 models)")
        .expect("title row 'Select Model (12 models)' should be present");

    // @step Then the title shows "Select Model" in bold yellow
    let name_start = col_of(&buf, ty, "Select Model") as u16;
    let name_len = "Select Model".chars().count() as u16;
    for x in name_start..name_start + name_len {
        assert_eq!(
            cell_fg(&buf, x, ty),
            Color::Yellow,
            "name cell {x} should be Yellow"
        );
        assert!(
            cell_mod(&buf, x, ty).contains(Modifier::BOLD),
            "name cell {x} should be BOLD"
        );
    }

    // @step And the count " (12 models)" is shown in dim DarkGray
    assert_count_is_dark_gray(&buf, ty, "(12 models)");
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Single model renders singular noun in dim count span
// ════════════════════════════════════════════════════════════════════════
#[test]
fn single_model_renders_singular_noun_in_dim_count_span() {
    // @step Given the model selector is in browse mode with 1 model available
    let mut view = view_with_models(1);

    // @step When the browse list is rendered
    let buf = render_to_buffer(&mut view);
    let ty = find_row(&buf, "Select Model (1 model)")
        .expect("title row 'Select Model (1 model)' should be present");

    // @step Then the count " (1 model)" is shown in dim DarkGray using the singular noun
    let row = row_string(&buf, ty);
    assert!(
        row.contains("(1 model)") && !row.contains("(1 models)"),
        "singular noun must be used, got {row:?}"
    );
    assert_count_is_dark_gray(&buf, ty, "(1 model)");
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Custom-model overlay title is unaffected by the two-span title change
// ════════════════════════════════════════════════════════════════════════
#[test]
fn custom_model_overlay_title_is_unaffected() {
    // @step Given the model selector has the Add Custom Model overlay open
    // A local-server profile section enables the `a` keybind to open the
    // Add Custom Model form; the cursor rests on the profile header.
    let mut view = ModelSelectorView::new();
    view.set_session(Some(SessionId::new("s-1")));
    let provider = ProviderInfo {
        key: "openai".to_string(),
        display_name: "openai: my-profile".to_string(),
        models: vec![model("base")],
        profile_name: Some("my-profile".to_string()),
        is_unreachable: false,
    };
    view.set_providers(vec![provider]);
    view.handle_key(key(KeyCode::Char('a')));

    // @step When the overlay is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then the title shows "Add Custom Model" in its existing overlay style with no dim DarkGray count span
    let ty = find_row(&buf, "Add Custom Model")
        .expect("overlay title row 'Add Custom Model' should be present");
    let row = row_string(&buf, ty);
    assert!(
        !row.contains("models)") && !row.contains("model)"),
        "overlay title must NOT carry a model count span, got {row:?}"
    );
    // The overlay title keeps its existing overlay style: every cell of the
    // "Add Custom Model" name is Cyan + BOLD (the raw-title scaffold style,
    // full_screen_shell.rs:124-127) — NOT the two-span DarkGray count
    // treatment used by the browse-list title.
    let name_start = col_of(&buf, ty, "Add Custom Model") as u16;
    let name_len = "Add Custom Model".chars().count() as u16;
    for x in name_start..name_start + name_len {
        assert_eq!(
            cell_fg(&buf, x, ty),
            Color::Cyan,
            "overlay title cell {x} must be Cyan (raw-title overlay style), got {:?}",
            cell_fg(&buf, x, ty)
        );
        assert!(
            cell_mod(&buf, x, ty).contains(Modifier::BOLD),
            "overlay title cell {x} must be BOLD (raw-title overlay style), got mod={:?}",
            cell_mod(&buf, x, ty)
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Refreshing state renders the refresh suffix in the dim count span
// ════════════════════════════════════════════════════════════════════════
#[test]
fn refreshing_suffix_renders_in_dim_count_span_never_bold_name() {
    // @step Given the model selector is in browse mode with 3 models available
    let mut view = view_with_models(3);

    // @step And a refresh is in flight
    // Pressing "r" emits the refresh action and flips the view into the
    // refreshing state (title gains the "(refreshing...)" suffix).
    view.handle_key(key(KeyCode::Char('r')));

    // @step When the browse list is rendered
    let buf = render_to_buffer(&mut view);
    let ty = find_row(&buf, "(refreshing...)")
        .expect("title row with '(refreshing...)' suffix should be present");

    // @step Then the "(refreshing...)" suffix is shown in the dim DarkGray count span
    assert_count_is_dark_gray(&buf, ty, "(refreshing...)");
    // The count noun span is dim too, so the whole trailing annotation reads
    // as a single dim status region.
    assert_count_is_dark_gray(&buf, ty, "(3 models)");

    // @step And the bold-yellow "Select Model" name span never contains the refresh suffix
    let name_start = col_of(&buf, ty, "Select Model") as u16;
    let name_len = "Select Model".chars().count() as u16;
    let suffix_start = col_of(&buf, ty, "(refreshing...)") as u16;
    assert!(
        suffix_start >= name_start + name_len,
        "refresh suffix must start AFTER the bold name span"
    );
    for x in name_start..name_start + name_len {
        assert_eq!(
            cell_fg(&buf, x, ty),
            Color::Yellow,
            "name cell {x} should stay bold Yellow while refreshing"
        );
        assert!(
            cell_mod(&buf, x, ty).contains(Modifier::BOLD),
            "name cell {x} should stay BOLD while refreshing"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Browse-list title uses the two-span style, not the shared blue-bold title
// ════════════════════════════════════════════════════════════════════════
#[test]
fn browse_title_is_two_span_yellow_not_shared_blue_bold() {
    // @step Given the model selector is in browse mode with 3 models available
    let mut view = view_with_models(3);

    // @step When the browse list is rendered
    let buf = render_to_buffer(&mut view);
    let ty = find_row(&buf, "Select Model (3 models)")
        .expect("title row 'Select Model (3 models)' should be present");

    // @step Then the "Select Model" name span is bold yellow, not the shared blue-bold title style
    // render_title_with_count (ResumeSession/SearchHistory) paints the WHOLE
    // title Blue+BOLD. The two-span model path paints the name Yellow+BOLD and
    // the count DarkGray — so no cell of the title row is Blue.
    let name_start = col_of(&buf, ty, "Select Model") as u16;
    let name_len = "Select Model".chars().count() as u16;
    for x in name_start..name_start + name_len {
        assert_eq!(
            cell_fg(&buf, x, ty),
            Color::Yellow,
            "name cell {x} must be Yellow (two-span), not the shared blue-bold title"
        );
        assert_ne!(
            cell_fg(&buf, x, ty),
            Color::Blue,
            "name cell {x} must NOT be Blue (shared render_title_with_count style)"
        );
    }
    // The count span is DarkGray, also distinct from the all-Blue shared title.
    assert_count_is_dark_gray(&buf, ty, "(3 models)");
}
