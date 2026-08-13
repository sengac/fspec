// Feature: spec/features/model-selector-filter.feature
//
//! MODEL-007 — Model selector renders the filter input row.
//!
//! This test file validates the acceptance criteria defined in the feature
//! file. Each scenario maps 1:1 to a `#[test]` here, and every Gherkin step
//! carries a matching `// @step` comment.
//!
//! Strategy (mirrors the MODEL-008 title test
//! `model_selector_title_two_span_model008.rs`): render the full
//! `ModelSelectorView` into a ratatui `Buffer` and read cell text. Filter
//! state is driven purely through the PUBLIC `handle_key` API — pressing
//! `/` to enter filter mode then typing chars pushes onto `filter`
//! (`filter_mode == true`), and pressing `Enter` commits the filter
//! (`filter_mode == false`, `filter` retained). No private fields are
//! touched and no new setters are added.
//!
//! RED phase: the browse-list body does NOT yet paint a "Filter: ..." row,
//! so the "top row of the body" assertions and the visible_rows-reservation
//! assertion are expected to FAIL for the RIGHT reason (missing prompt /
//! visible_rows not reduced), not a compile error.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_fspec_tui::views::ModelSelectorView;
use codelet_rpc_types::{ModelEntry, ProviderInfo, SessionId};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

// ────────────────────────────────────────────────────────────────────────
// Helpers
// ────────────────────────────────────────────────────────────────────────

const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

/// The scaffold splits the area into title(1) / spacer(1) / body(Min 0) /
/// footer(1), so the body region starts at y == 2. The "top row of the
/// body" is therefore buffer row 2.
const BODY_TOP_Y: u16 = 2;

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

/// One provider carrying `opus`, `sonnet`, `haiku` so that a filter of
/// "opus" matches exactly one model and the other two are excluded.
fn view_with_models() -> ModelSelectorView {
    let provider = ProviderInfo {
        key: "anthropic".to_string(),
        display_name: "anthropic".to_string(),
        models: vec![model("opus"), model("sonnet"), model("haiku")],
        profile_name: None,
        is_unreachable: false,
    };
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![provider]);
    v
}

/// A single provider whose models are `m0..m{n-1}` (all match a filter of
/// "m"). Enough rows to fill AND overflow a short body so the last visible
/// model row sits exactly on the clipping boundary before the legend.
fn view_with_n_models(n: usize) -> ModelSelectorView {
    let ids: Vec<String> = (0..n).map(|i| format!("m{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let provider = ProviderInfo {
        key: "anthropic".to_string(),
        display_name: "anthropic".to_string(),
        models: refs.iter().map(|i| model(i)).collect(),
        profile_name: None,
        is_unreachable: false,
    };
    let mut v = ModelSelectorView::new();
    v.set_session(Some(SessionId::new("s-1")));
    v.set_providers(vec![provider]);
    v
}

/// Expand every provider header (Right on the top header row) so the model
/// rows are visible in the browse list WITHOUT entering filter mode. Used to
/// establish the no-filter baseline where the body has its FULL height.
fn expand_top_header(v: &mut ModelSelectorView) {
    v.handle_key(key(KeyCode::Right));
}

/// Drive the public key API to set an ACTIVE filter of "opus"
/// (`filter_mode == true`, trailing-cursor prompt): `/` then the chars.
fn type_active_filter(v: &mut ModelSelectorView, text: &str) {
    v.handle_key(key(KeyCode::Char('/')));
    for c in text.chars() {
        v.handle_key(key(KeyCode::Char(c)));
    }
}

/// Drive the public key API to set a COMMITTED filter of "opus"
/// (`filter_mode == false`, filter retained): type it, then press Enter.
fn commit_filter(v: &mut ModelSelectorView, text: &str) {
    type_active_filter(v, text);
    v.handle_key(key(KeyCode::Enter));
}

fn render_to_buffer(view: &mut ModelSelectorView) -> Buffer {
    render_sized(view, WIDTH, HEIGHT)
}

fn render_sized(view: &mut ModelSelectorView, width: u16, height: u16) -> Buffer {
    let mut term = Terminal::new(TestBackend::new(width, height)).expect("Terminal::new");
    term.draw(|f| view.render(f.area(), f.buffer_mut()))
        .expect("draw");
    term.backend().buffer().clone()
}

/// Concatenate the visible symbols of buffer row `y` into a trimmed String.
fn row_string(buf: &Buffer, y: u16) -> String {
    let mut s = String::new();
    for x in 0..buf.area.width {
        s.push_str(buf[(x, y)].symbol());
    }
    s.trim_end().to_string()
}

/// Find the y-index of the first row whose joined text contains `needle`.
fn find_row(buf: &Buffer, needle: &str) -> Option<u16> {
    (0..buf.area.height).find(|&y| row_string(buf, y).contains(needle))
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Active filter renders prompt with trailing cursor
// ════════════════════════════════════════════════════════════════════════
#[test]
fn active_filter_renders_prompt_with_trailing_cursor() {
    // @step Given the model selector is in browse mode with models loaded
    let mut view = view_with_models();
    type_active_filter(&mut view, "opus");

    // @step When the model selector view is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then the top row of the body shows "Filter: opus_"
    let top = row_string(&buf, BODY_TOP_Y);
    assert_eq!(
        top, "Filter: opus_",
        "top body row (y={BODY_TOP_Y}) must show the active-filter prompt with trailing cursor, got {top:?}"
    );

    // @step And filter mode is active and the filter text is "opus"
    // filter_mode is set true by `/` and stays true until Enter/Esc; the
    // typed chars land in `filter`. Both are proven by the rendered prompt:
    // the trailing `_` is emitted ONLY when filter_mode is true, and the
    // "opus" text is the committed filter value.
    assert!(
        top.ends_with('_'),
        "trailing underscore proves filter_mode is active, got {top:?}"
    );
    assert!(
        top.contains("opus"),
        "filter text 'opus' must appear in the prompt, got {top:?}"
    );

    // @step And only models matching "opus" are listed below the prompt
    assert!(
        find_row(&buf, "opus").is_some(),
        "the matching model 'opus' must be listed"
    );
    assert!(
        find_row(&buf, "sonnet").is_none(),
        "non-matching model 'sonnet' must be excluded from the list"
    );
    assert!(
        find_row(&buf, "haiku").is_none(),
        "non-matching model 'haiku' must be excluded from the list"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Committed non-empty filter renders prompt without cursor
// ════════════════════════════════════════════════════════════════════════
#[test]
fn committed_non_empty_filter_renders_prompt_without_cursor() {
    // @step Given the model selector is in browse mode with models loaded
    let mut view = view_with_models();
    commit_filter(&mut view, "opus");

    // @step When the model selector view is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then the top row of the body shows "Filter: opus" without a trailing underscore
    let top = row_string(&buf, BODY_TOP_Y);
    assert_eq!(
        top, "Filter: opus",
        "committed filter prompt must have NO trailing underscore, got {top:?}"
    );

    // @step And filter mode is not active but the filter text is "opus"
    assert!(
        !top.ends_with('_'),
        "absence of a trailing underscore proves filter_mode is inactive, got {top:?}"
    );
    assert!(
        top.contains("opus"),
        "committed filter text 'opus' must still appear, got {top:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: No filter renders no prompt row
// ════════════════════════════════════════════════════════════════════════
#[test]
fn no_filter_renders_no_prompt_row() {
    // @step Given the model selector is in browse mode with models loaded
    let mut view = view_with_models();

    // @step When the model selector view is rendered
    let buf = render_to_buffer(&mut view);

    // @step Then no "Filter:" row is rendered
    assert!(
        find_row(&buf, "Filter:").is_none(),
        "no filter prompt row must be present when there is no filter"
    );

    // @step And filter mode is not active and the filter text is empty
    // Proven observably: no "Filter:" prompt row exists at all, which the
    // render only emits when (filter_mode || !filter.is_empty()).
    let top = row_string(&buf, BODY_TOP_Y);
    assert!(
        !top.starts_with("Filter:"),
        "top body row must NOT be a filter prompt, got {top:?}"
    );

    // @step And the model list starts at the very top of the body
    // The provider header ("anthropic") occupies the very top body row when
    // no filter prompt steals it.
    assert!(
        top.contains("anthropic"),
        "the model list (provider header) must start at the top of the body, got {top:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════
// Scenario: Filter row reserves a line so no model row is hidden
// ════════════════════════════════════════════════════════════════════════
#[test]
fn filter_row_reserves_a_line_so_no_model_row_is_hidden() {
    // @step Given the model selector is in browse mode with models loaded
    // Seed enough models (8) so the list overflows a deliberately SHORT body
    // where the last visible model row sits on the clipping boundary just
    // above the legend. With H=10 the no-filter list paints 6 rows
    // (header + m0..m4) and the boundary content row is m4; adding the filter
    // prompt steals one body line, so without the 1-line reservation that
    // boundary model would be overwritten by the prompt or pushed off-screen.
    const N: usize = 8;
    const HEIGHT_SHORT: u16 = 10;
    let mut no_filter = view_with_n_models(N);
    expand_top_header(&mut no_filter); // browse baseline, filter absent
    let mut with_filter = view_with_n_models(N);
    type_active_filter(&mut with_filter, "m"); // matches every model, auto-expands

    // @step When the model selector view is rendered into a fixed-height buffer
    let baseline = render_sized(&mut no_filter, WIDTH, HEIGHT_SHORT);
    let baseline_visible = no_filter.visible_rows_for_test();
    let buf = render_sized(&mut with_filter, WIDTH, HEIGHT_SHORT);
    let filtered_visible = with_filter.visible_rows_for_test();

    // @step Then visible_rows is reduced by one to reserve the filter line
    assert_eq!(
        filtered_visible,
        baseline_visible.saturating_sub(1),
        "visible_rows must be reduced by one when a filter row is present \
         (baseline={baseline_visible}, filtered={filtered_visible})"
    );
    // The baseline must genuinely be at the clipping boundary: the last model
    // it can show must occupy the FINAL content row directly above the legend
    // (buffer row HEIGHT_SHORT-2). If nothing were ever at risk of clipping
    // this assertion would not hold, so it proves the boundary is exercised.
    let legend_y = HEIGHT_SHORT - 2; // footer at H-1, legend at H-2
    let baseline_boundary_y = legend_y - 1; // last content row
    let baseline_boundary = row_string(&baseline, baseline_boundary_y);
    assert!(
        baseline_boundary.contains("[200k]"),
        "baseline must paint a model row on the last content row (y={baseline_boundary_y}), \
         proving the buffer height puts a model at the clipping boundary, got {baseline_boundary:?}"
    );
    // Identify the model on that baseline boundary row (e.g. "m4").
    let boundary_model = baseline_boundary
        .split_whitespace()
        .find(|t| t.starts_with('m'))
        .expect("baseline boundary row must carry a model id")
        .to_string();

    // @step And a filter row is present because filter mode is active
    let prompt_y =
        find_row(&buf, "Filter: m_").expect("an active filter prompt row must be present");
    assert_eq!(
        prompt_y, BODY_TOP_Y,
        "the filter prompt must occupy the very top body row (y={BODY_TOP_Y}), got y={prompt_y}"
    );

    // @step And no model row is hidden behind the filter prompt
    // Without the 1-line reservation the list would still paint its first
    // content row at BODY_TOP_Y — exactly where the prompt sits — so the
    // provider header (the row the baseline paints at BODY_TOP_Y) would be
    // clobbered by the prompt. The reservation shifts the whole list DOWN by
    // one: the header now lands at BODY_TOP_Y+1 (still on screen, not behind
    // the prompt), and every remaining body row down to the boundary keeps
    // painting real content instead of being overwritten or left blank.
    let baseline_top = row_string(&baseline, BODY_TOP_Y);
    assert!(
        baseline_top.contains("anthropic"),
        "baseline must paint the provider header on the top content row \
         (the row the prompt would otherwise clobber), got {baseline_top:?}"
    );
    let shifted_header_y = BODY_TOP_Y + 1;
    let shifted_header = row_string(&buf, shifted_header_y);
    assert!(
        shifted_header.contains("anthropic"),
        "the provider header must be pushed to y={shifted_header_y} by the reservation, \
         NOT hidden behind the prompt at y={BODY_TOP_Y}, got {shifted_header:?}"
    );
    // And the final content row above the legend still paints a real model row
    // (the reservation did not blank or clip the bottom of the list).
    let filtered_last_y = legend_y - 1;
    let filtered_last = row_string(&buf, filtered_last_y);
    assert!(
        filtered_last.contains("[200k]"),
        "with the reservation the last body row (y={filtered_last_y}) must still paint a \
         model row rather than being blank or clipped, got {filtered_last:?}"
    );
    // Sanity: the boundary model the baseline showed on its LAST row (m4) is
    // the one row the shortened window legitimately drops — its absence
    // confirms exactly ONE line was reserved (not zero, not two).
    assert!(
        find_row(&buf, &format!("{boundary_model} ")).is_none(),
        "reserving exactly one line drops precisely the baseline boundary model \
         {boundary_model:?} from the shorter window (proving a 1-line, not 0- or 2-line, reservation)"
    );
}

// TEMP PROBE — remove before final
