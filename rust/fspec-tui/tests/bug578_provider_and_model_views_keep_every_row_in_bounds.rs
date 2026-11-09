//! BUG-578: the Provider Settings and /model views keep every row in
//! bounds: the cursor stays painted, a shrinking reload never slices past
//! the tree, the scrollbar drag holds, long labels and short terminals
//! keep what matters on screen, and a /model filter can be cleared.
//!
//! Feature: spec/features/provider-and-model-views-keep-every-row-in-bounds.feature
//!
//! Every test renders the real view into a ratatui buffer and reads the
//! painted rows (deterministic: no runtime, no clock).

use codelet_fspec_tui::views::provider_settings::profile_form::{ProfileForm, PROFILE_FORM_FIELDS};
use codelet_fspec_tui::views::provider_settings::projection::project_display_infos;
use codelet_fspec_tui::views::provider_settings::ProviderDisplayInfo;
use codelet_fspec_tui::views::{ModelSelectorView, ProviderSettingsMode, ProviderSettingsView};
use codelet_rpc_types::{
    ModelEntry, ProfileDefinition, ProviderCredentialInfo, ProviderInfo, ProviderProfileConfig,
    ProviderSettingsConfig, SessionId,
};
use codelet_test_helpers::{OrFail, TestResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

fn lines_of(buf: &Buffer) -> Vec<String> {
    (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

fn render_settings(view: &mut ProviderSettingsView, width: u16, height: u16) -> Vec<String> {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);
    lines_of(&buf)
}

fn render_models(view: &mut ModelSelectorView, width: u16, height: u16) -> Vec<String> {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    view.render(area, &mut buf);
    lines_of(&buf)
}

fn provider(id: &str) -> ProviderCredentialInfo {
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

fn provider_name(index: usize) -> String {
    format!("prov{index:02}")
}

/// A Provider Settings view whose tree holds `count` collapsed providers.
fn settings_with(count: usize) -> ProviderSettingsView {
    let infos: Vec<ProviderCredentialInfo> =
        (0..count).map(|i| provider(&provider_name(i))).collect();
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(project_display_infos(&infos, &[]));
    view
}

/// True iff a painted row carries the selection marker and the selected
/// provider's name.
fn selected_row_painted(lines: &[String], selected: usize) -> bool {
    let name = provider_name(selected);
    lines
        .iter()
        .any(|line| line.contains("> ") && line.contains(&name))
}

/// The (column, row) of the first scrollbar thumb cell.
fn thumb(lines: &[String]) -> Option<(u16, u16)> {
    lines.iter().enumerate().find_map(|(y, line)| {
        line.chars()
            .position(|c| c == '■')
            .map(|x| (x as u16, y as u16))
    })
}

/// The last row of the scrollbar gutter (the lowest row painting a thumb
/// or track cell in `column`).
fn gutter_bottom(lines: &[String], column: u16) -> Option<u16> {
    lines.iter().enumerate().rev().find_map(|(y, line)| {
        matches!(line.chars().nth(usize::from(column)), Some('■' | '│')).then_some(y as u16)
    })
}

fn model(id: &str) -> ModelEntry {
    ModelEntry {
        id: id.to_string(),
        display_name: id.to_string(),
        context_window: Some(200_000),
        supports_reasoning: true,
        supports_vision: true,
        is_custom: false,
    }
}

fn models_with(count: usize) -> ModelSelectorView {
    let provider = ProviderInfo {
        key: "anthropic".to_string(),
        display_name: "anthropic".to_string(),
        models: (0..count).map(|i| model(&format!("m{i:02}"))).collect(),
        profile_name: None,
        is_unreachable: false,
        provider_id: "anthropic".to_string(),
        unavailable_reason: None,
    };
    let mut view = ModelSelectorView::new();
    view.set_session(Some(SessionId::new("s-1")));
    view.set_providers(vec![provider]);
    view
}

#[test]
fn the_selected_row_stays_painted_with_a_committed_filter() -> TestResult {
    // @step Given Provider Settings at 80x12 with more rows than fit and the filter "o" committed
    let mut view = settings_with(30);
    render_settings(&mut view, 80, 12);
    view.handle_key(key(KeyCode::Char('/')));
    view.handle_key(key(KeyCode::Char('o')));
    view.handle_key(key(KeyCode::Enter));
    assert_eq!(view.filter, "o", "the filter is committed");

    // @step When the user moves the cursor to the last row
    for _ in 0..40 {
        view.handle_key(key(KeyCode::Down));
        render_settings(&mut view, 80, 12);
    }
    let lines = render_settings(&mut view, 80, 12);

    // @step Then the selected row is painted on screen
    assert_eq!(view.selected_index, 29, "the cursor is on the last row");
    assert!(
        selected_row_painted(&lines, view.selected_index),
        "the selected row is painted:\n{}",
        lines.join("\n")
    );
    Ok(())
}

#[test]
fn the_selected_row_stays_painted_when_a_status_line_arrives() -> TestResult {
    // @step Given Provider Settings at 80x12 with the cursor on the last painted row
    let mut view = settings_with(30);
    let mut lines = render_settings(&mut view, 80, 12);
    while !lines
        .iter()
        .any(|line| line.contains(&provider_name(view.selected_index + 1)))
        || view.scroll_offset > 0
    {
        let before = view.selected_index;
        view.handle_key(key(KeyCode::Down));
        lines = render_settings(&mut view, 80, 12);
        if view.scroll_offset > 0 {
            view.handle_key(key(KeyCode::Up));
            lines = render_settings(&mut view, 80, 12);
            break;
        }
        assert_ne!(view.selected_index, before, "the cursor moves down");
    }
    assert!(selected_row_painted(&lines, view.selected_index));
    let selected = view.selected_index;

    // @step When the status "✓ saved" arrives
    view.set_status("✓ saved");

    // @step Then the next frame still paints the selected row
    let lines = render_settings(&mut view, 80, 12);
    assert_eq!(view.selected_index, selected, "the cursor did not move");
    assert!(
        lines.iter().any(|line| line.contains("✓ saved")),
        "the status is painted:\n{}",
        lines.join("\n")
    );
    assert!(
        selected_row_painted(&lines, selected),
        "the selected row is painted:\n{}",
        lines.join("\n")
    );
    Ok(())
}

#[test]
fn a_reload_that_shrinks_the_tree_never_slices_past_its_end() -> TestResult {
    // @step Given Provider Settings with 40 rows, the window at offset 30 and the cursor on row 35
    let mut view = settings_with(40);
    render_settings(&mut view, 80, 12);
    view.scroll_offset = 30;
    view.selected_index = 35;

    // @step When a reload leaves 5 rows
    let infos: Vec<ProviderCredentialInfo> = (0..5).map(|i| provider(&provider_name(i))).collect();
    view.set_provider_display_infos(project_display_infos(&infos, &[]));

    // @step Then the next render paints without a panic
    let lines = render_settings(&mut view, 80, 12);

    // @step And the window offset is 0 and the selection is on the last row
    assert_eq!(view.scroll_offset, 0);
    assert_eq!(view.selected_index, 4);
    assert!(selected_row_painted(&lines, 4), "{}", lines.join("\n"));
    Ok(())
}

#[test]
fn dragging_the_provider_settings_scrollbar_thumb_moves_the_window_and_keeps_it_there() -> TestResult
{
    // @step Given Provider Settings with 60 rows and the selection on row 0
    let mut view = settings_with(60);
    let lines = render_settings(&mut view, 80, 24);
    assert_eq!(view.selected_index, 0);
    let (column, row) = thumb(&lines).or_fail("the scrollbar thumb is painted")?;
    let bottom = gutter_bottom(&lines, column).or_fail("the gutter is painted")?;

    // @step When the user presses on the scrollbar thumb and drags it to the bottom of the gutter
    view.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));
    view.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        column,
        bottom,
    ));
    view.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), column, bottom));

    // @step Then after the next render the window shows the last rows
    let lines = render_settings(&mut view, 80, 24);
    assert!(
        lines.iter().any(|line| line.contains(&provider_name(59))),
        "the last row is painted:\n{}",
        lines.join("\n")
    );

    // @step And the selection is inside the window
    assert!(view.scroll_offset > 0, "the window moved");
    assert!(
        selected_row_painted(&lines, view.selected_index),
        "the selected row is painted:\n{}",
        lines.join("\n")
    );
    Ok(())
}

#[test]
fn dragging_the_model_scrollbar_thumb_moves_the_window_and_selects_a_model_row_inside_it(
) -> TestResult {
    // @step Given /model with 60 rows and the selection on the first model
    let mut view = models_with(60);
    view.handle_key(key(KeyCode::Right));
    view.handle_key(key(KeyCode::Down));
    let lines = render_models(&mut view, 120, 24);
    assert!(
        lines.iter().any(|line| line.contains("> m00")),
        "the first model is selected:\n{}",
        lines.join("\n")
    );
    let (column, row) = thumb(&lines).or_fail("the scrollbar thumb is painted")?;
    let bottom = gutter_bottom(&lines, column).or_fail("the gutter is painted")?;

    // @step When the user presses on the scrollbar thumb and drags it to the bottom of the gutter
    view.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));
    view.handle_mouse(mouse(
        MouseEventKind::Drag(MouseButton::Left),
        column,
        bottom,
    ));
    view.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), column, bottom));

    // @step Then after the next render the window shows the last rows
    let lines = render_models(&mut view, 120, 24);
    assert!(
        lines.iter().any(|line| line.contains("m59")),
        "the last model is painted:\n{}",
        lines.join("\n")
    );

    // @step And the selection is a model row inside the window
    assert!(
        lines.iter().any(|line| line.contains("> m")),
        "a painted model row is selected:\n{}",
        lines.join("\n")
    );
    assert!(
        !lines.iter().any(|line| line.contains("> m00")),
        "the selection moved with the window:\n{}",
        lines.join("\n")
    );
    Ok(())
}

#[test]
fn a_drag_that_leaves_the_gutter_column_goes_on_until_the_button_is_released() -> TestResult {
    // @step Given a scrollbar drag started on the Provider Settings thumb
    let mut view = settings_with(60);
    let lines = render_settings(&mut view, 80, 24);
    let (column, row) = thumb(&lines).or_fail("the scrollbar thumb is painted")?;
    let bottom = gutter_bottom(&lines, column).or_fail("the gutter is painted")?;
    view.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), column, row));

    // @step When the pointer moves one column left of the gutter and further down
    let off = column - 1;
    view.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), off, row + 5));
    render_settings(&mut view, 80, 24);
    let midway = view.scroll_offset;
    view.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), off, bottom));
    render_settings(&mut view, 80, 24);

    // @step Then the window still follows the pointer
    assert!(midway > 0, "the drag off the gutter moved the window");
    assert!(
        view.scroll_offset > midway,
        "the window follows further down"
    );

    // @step And releasing the button there ends the drag
    view.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), off, bottom));
    let settled = view.scroll_offset;
    view.handle_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), off, row));
    render_settings(&mut view, 80, 24);
    assert_eq!(
        view.scroll_offset, settled,
        "a drag after the release moves nothing"
    );
    Ok(())
}

#[test]
fn a_long_profile_row_keeps_its_missing_limits_whole() -> TestResult {
    // @step Given the openai profile "local" with base URL "http://192.168.1.50:8080/v1" and no context window or max output tokens
    let mut view = ProviderSettingsView::new();
    view.set_provider_display_infos(vec![ProviderDisplayInfo {
        id: "openai".into(),
        name: "OpenAI".into(),
        configured: true,
        credential_type: "api-key".into(),
        ..ProviderDisplayInfo::default()
    }]);
    view.set_provider_settings_configs(vec![ProviderSettingsConfig {
        provider_id: "openai".into(),
        settings: ProfileDefinition::default(),
        profiles: vec![ProviderProfileConfig {
            name: "local".into(),
            definition: ProfileDefinition {
                base_url: "http://192.168.1.50:8080/v1".into(),
                ..ProfileDefinition::default()
            },
        }],
    }]);
    view.toggle_expansion("openai");

    // @step When Provider Settings renders its row at width 60
    let lines = render_settings(&mut view, 60, 12);

    // @step Then the row shows "— missing context window, max output tokens" whole
    let row = lines
        .iter()
        .find(|line| line.contains("local"))
        .or_fail("the profile row is painted")?;
    assert!(
        row.contains("— missing context window, max output tokens"),
        "the missing limits are whole: {row:?}"
    );

    // @step And the name and URL part ends in "…"
    assert!(
        row.contains("local →") && row.contains("…— missing") && !row.contains("8080/v1"),
        "the name and URL part is shortened with an ellipsis: {row:?}"
    );
    Ok(())
}

#[test]
fn a_short_terminal_shows_the_focused_profile_field_and_the_save_error() -> TestResult {
    // @step Given the profile form at 80x15 with the last field focused
    let mut form = ProfileForm::new_create();
    form.is_editing_name = false;
    form.field_index = PROFILE_FORM_FIELDS.len() - 1;
    let mut view = ProviderSettingsView::new();
    view.mode = ProviderSettingsMode::CreateProfile {
        provider_id: "openai".into(),
        form,
    };

    // @step And the status "Context window must be a number"
    view.set_status("Context window must be a number");

    // @step When the form renders
    let lines = render_settings(&mut view, 80, 15);

    // @step Then the focused field is on screen
    let last = PROFILE_FORM_FIELDS[PROFILE_FORM_FIELDS.len() - 1];
    let focused = lines
        .iter()
        .position(|line| line.contains(&format!("> {last}:")))
        .or_fail(&format!(
            "the focused field is painted:\n{}",
            lines.join("\n")
        ))?;

    // @step And the status is on screen under the fields
    let status = lines
        .iter()
        .position(|line| line.contains("Context window must be a number"))
        .or_fail(&format!("the status is painted:\n{}", lines.join("\n")))?;
    assert!(status > focused, "the status is under the fields");
    Ok(())
}

#[test]
fn esc_in_model_clears_a_committed_filter_before_it_closes() -> TestResult {
    // @step Given /model with the filter "gpt" committed in browse mode
    let mut view = models_with(3);
    view.handle_key(key(KeyCode::Char('/')));
    for c in "gpt".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    view.handle_key(key(KeyCode::Enter));
    let lines = render_models(&mut view, 120, 24);
    assert!(lines.iter().any(|line| line.contains("Filter: gpt")));

    // @step When the user presses Esc
    let first = view.handle_key(key(KeyCode::Esc));

    // @step Then the filter is cleared and /model stays open
    assert!(
        !matches!(
            first,
            codelet_fspec_tui::views::model_selector::ModelSelectorEvent::Close
        ),
        "the first Esc does not close /model"
    );
    let lines = render_models(&mut view, 120, 24);
    assert!(
        !lines.iter().any(|line| line.contains("Filter:")),
        "the filter is cleared:\n{}",
        lines.join("\n")
    );

    // @step And a second Esc closes /model
    let second = view.handle_key(key(KeyCode::Esc));
    assert!(matches!(
        second,
        codelet_fspec_tui::views::model_selector::ModelSelectorEvent::Close
    ));
    Ok(())
}

#[test]
fn a_filter_that_matches_nothing_says_so() -> TestResult {
    // @step Given /model with models loaded
    let mut view = models_with(3);

    // @step When the user filters by "zzz"
    view.handle_key(key(KeyCode::Char('/')));
    for c in "zzz".chars() {
        view.handle_key(key(KeyCode::Char(c)));
    }
    let lines = render_models(&mut view, 120, 24);

    // @step Then the body shows "No models match 'zzz'"
    assert!(
        lines
            .iter()
            .any(|line| line.contains("No models match 'zzz'")),
        "the empty filter result says so:\n{}",
        lines.join("\n")
    );
    Ok(())
}

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> TestResult {
    for entry in std::fs::read_dir(dir).or_fail("read a source directory")? {
        let path = entry.or_fail("read a directory entry")?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

/// The fspec-tui source outside tests: `*_tests.rs` / `tests.rs` files are
/// skipped and comment lines dropped, so only code is scanned.
fn production_code() -> Result<String, codelet_test_helpers::TestError> {
    let mut files = Vec::new();
    rust_files(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    )?;
    let mut code = String::new();
    for path in files {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "tests.rs" || name.ends_with("_tests.rs") {
            continue;
        }
        let text = std::fs::read_to_string(&path).or_fail("read a source file")?;
        for line in text.lines() {
            if !line.trim_start().starts_with("//") {
                code.push_str(line);
                code.push('\n');
            }
        }
    }
    Ok(code)
}

#[test]
fn the_dead_provider_settings_paths_are_gone() -> TestResult {
    // @step Given the fspec-tui source outside tests
    let code = production_code()?;

    // @step When it is scanned for the Provider Settings paths nothing reaches
    let present = |token: &str| code.contains(token);

    // @step Then it has no Summary or OAuthNotice detail view and no DetailStatus
    for gone in [
        "DetailSub::Summary",
        "last_status",
        "OAuthNotice",
        "DetailStatus",
    ] {
        assert!(!present(gone), "{gone} is still in the source");
    }

    // @step And it has no RefreshProviderModels, ProviderModelsRefreshed, TestProviderConnection or ProviderTestComplete action
    for gone in [
        "RefreshProviderModels",
        "ProviderModelsRefreshed",
        "TestProviderConnection",
        "ProviderTestComplete",
    ] {
        assert!(!present(gone), "{gone} is still in the source");
    }

    // @step And it has no inline test_result decoration
    for gone in ["test_result", "ProviderTestStatus", "ProviderTestResult"] {
        assert!(!present(gone), "{gone} is still in the source");
    }

    // @step And the backend still offers test_provider_connection, which the Paseo plugin uses
    assert!(
        present("async fn test_provider_connection"),
        "FspecBackend::test_provider_connection must stay"
    );

    // @step And Enter on Provider Settings with providers but no rows built stays in the list
    let mut view = ProviderSettingsView::new();
    view.set_providers(vec![provider("openai")]);
    assert!(view.nav_items.is_empty(), "no rows are built");
    view.handle_key(key(KeyCode::Enter));
    assert_eq!(view.mode, ProviderSettingsMode::List);
    Ok(())
}
