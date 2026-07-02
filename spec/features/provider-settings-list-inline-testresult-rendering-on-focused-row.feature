@done
@rpc-150
@RPC-150
@rust
@tui
@provider-settings
@source-shape
@regression
Feature: Provider settings list: inline testResult rendering on focused row
  """
  Regression-shape feature: source-string assertions over
  codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs only
  — no ratatui Buffer / Frame construction needed.

  Pattern mirrors RPC-089/RPC-090/RPC-151/152/153/155: pin canonical
  invocation points so the RPC-072 stub state (test result hidden
  inside the now-removed Detail::Summary view) cannot silently
  re-emerge.

  RPC-158 already covers the behavioural state contract
  (`set_test_result` / `clear_test_result` / `test_result` field
  default). RPC-150 is render-only: pins that the inline decoration
  is wired into `list_nav_render.rs` and not into `detail.rs` /
  `row_render.rs`. Different file, different invariants, no overlap.

  Tests target codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
  via Cargo workspace path traversal — sub-millisecond assertions, no
  async runtime, no ratatui Buffer construction.
  """

  Background: User Story
    As a fspec contributor
    I want to have the inline test_result decoration on focused/matching Provider rows pinned as a regression-shape invariant
    So that the RPC-072 stub state (test result hidden inside the removed Detail::Summary view) cannot silently re-emerge in list-mode rendering

  Scenario: render_nav_items reads view.test_result inside the per-row paint loop
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    When I extract the brace-balanced body of the "fn render_nav_items" function
    Then the body must contain "view.test_result.as_ref()"
    And the body must contain "for (row_idx, item) in nav_items"

  Scenario: Provider row gate guards the test_result decoration paint
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    When I extract the brace-balanced body of the "fn render_nav_items" function
    Then the body must contain "matches!(kind, RowKind::Provider"
    And the body must contain "test_result.provider_id == item.provider_id"
    And the offset of "matches!(kind, RowKind::Provider" must be less than the offset of "test_result.provider_id == item.provider_id"

  Scenario: paint_test_result_decoration has exactly one call site and exactly one fn definition
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    When I scan the file as a string
    Then the file must contain exactly one occurrence of "fn paint_test_result_decoration("
    And the file must contain exactly one call site (total `paint_test_result_decoration(` count minus the one `fn` definition equals 1)

  Scenario: paint_test_result_decoration accepts the canonical six-argument signature
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    When I scan the file as a string
    Then the file must contain "fn paint_test_result_decoration("
    And the source must contain each of the six canonical parameter declarations

  Scenario: paint_test_result_decoration helper is owned exclusively by list_nav_render.rs
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/detail.rs
    And I read the source of codelet/fspec-tui/src/views/provider_settings/row_render.rs
    When I scan each file as a string
    Then detail.rs must contain zero occurrences of "paint_test_result_decoration"
    And row_render.rs must contain zero occurrences of "paint_test_result_decoration"

  Scenario: Decoration foreground comes from status decoration and background comes from row_band_bg
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    When I extract the brace-balanced body of the "fn paint_test_result_decoration" function
    Then the body must contain "status.decoration()"
    And the body must contain "row_band_bg(kind, selected)"
    And the body must contain "Style::default().fg(fg).bg(bg)"

  Scenario: Separator and decoration coordinates respect the row right boundary
    Given I read the source of codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs
    When I extract the brace-balanced body of the "fn paint_test_result_decoration" function
    Then the body must contain "row_area.x.saturating_add(row_area.width)"
    And the body must contain "if end_x >= right_bound"
    And the body must contain "let separator_x = end_x;"
    And the body must contain "let decoration_x = end_x.saturating_add(1);"
    And the body must contain "if decoration_x >= right_bound"
