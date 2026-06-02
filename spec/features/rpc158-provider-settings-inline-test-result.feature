@done
@agent-view
@ts-parity
@rust
@provider-settings
@tui
@RPC-158
Feature: Provider settings: inline testResult rendering on focused row in list mode

  """
  Render integration: codelet/fspec-tui/src/views/provider_settings/list_nav_render.rs — `render_nav_items` checks `view.test_result` for each Provider row. If `test_result.provider_id == item.provider_id`, after the existing `render_row` call, paint the test-result span on top of the row's color band. Decoration column = end-of-label column returned from a refactored `render_row` (change render_row return type from `()` to `u16` representing the column AFTER the last painted label cell — 18 existing RPC-104 callers ignore return value, no test changes required). Decoration style: foreground from ProviderTestStatus::span_color (Cyan/Green/Red), background from the row's existing band style (selected ? tint : Reset). Use `buf.set_stringn(end_x, y, &decoration_text, remaining_width, decoration_style)`. Skip painting if `end_x >= row_area.x + row_area.width` (no space left).
  Decoration text + color: introduce `ProviderTestStatus::decoration(&self) -> (String, Color)` returning ('Testing…', Cyan) | (format!('✓ ok ({n}ms)'), Green) | (format!('✗ {m}'), Red). Mirrors the existing `DetailStatus::to_span()` for the Testing/TestOk/TestErr variants (status_text.rs L28-37) bytes-for-bytes. Prefix the decoration with a single ASCII space when painting so the visible row reads 'Label ✓ ok (Nms)'.
  TS reference: src/tui/components/ProviderSettingsPanel.tsx renders testResult inline as `<Text color={color}> {symbol} {text}</Text>` immediately after the provider row label inside the same `<Box>` row. testResult lives on the panel-level state (`useProviderSettingsState.ts` testResult field, set by handleTestProviderConnection). The TS implementation scopes testResult to the row that triggered the test via provider_id matching — the same contract this card ports.
  Test plan: codelet/fspec-tui/tests/provider_settings_test_result_inline_rpc158.rs with one #[test] fn per scenario. Helpers: `view_with_two_providers()` builds a List-mode view containing OpenAI + Anthropic ProviderDisplayInfo; `render_to_buffer(view) -> Buffer` invokes `view.render(area, &mut buf)` against a fixed Rect. Buffer assertion helper `row_contains(buf, row_idx, needle) -> bool` walks one buffer row symbol-by-symbol and looks for substring. Color assertion helper `cell_fg(buf, x, y) -> Color`.
  Downstream wiring (out of scope for RPC-158): the backend test_provider_connection round-trip plumbed in `dispatch.rs` already routes its result to `DetailStatus`. A follow-up card or thin patch under RPC-054 epic will switch that wiring to call `view.set_test_result(...)` instead. RPC-158's exclusive contract is the field + methods + render integration. The legacy Detail::Summary path (status_text.rs DetailStatus) remains untouched so any current callers continue to compile.
  Implementation:
  - codelet/fspec-tui/src/views/provider_settings/mod.rs — add `pub test_result: Option<ProviderTestResult>` field on `ProviderSettingsView` (default None in `new()`); add `pub fn set_test_result(&mut self, provider_id: impl Into<String>, status: ProviderTestStatus)` and `pub fn clear_test_result(&mut self)`. Define `ProviderTestResult { pub provider_id: String, pub status: ProviderTestStatus }` and `pub enum ProviderTestStatus { Testing, Ok { latency_ms: u32 }, Err { message: String } }` — derive Debug/Clone/PartialEq/Eq. Re-export both from `mod.rs`.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. ProviderSettingsView carries a public field test_result: Option<ProviderTestResult> with ProviderTestResult { provider_id: String, status: ProviderTestStatus }
  #   2. ProviderTestStatus has exactly three variants: Testing, Ok { latency_ms: u32 }, Err { message: String } — mirroring the TS testResult discriminated union
  #   3. ProviderSettingsView exposes set_test_result(provider_id, status) and clear_test_result() public methods so backend round-trips and RPC-159 navigation handlers can drive the field
  #   4. test_result is None for newly-constructed (Default) views — no decoration is rendered until set_test_result is called
  #   5. List-mode rendering injects the test result inline ONLY on the Provider header row (RowKind::Provider) whose provider_id matches test_result.provider_id — Profile, ApiKey, OAuthLogin, OAuthStatus and AddProfile rows never render the decoration
  #   6. Inline decoration text follows the TS visual contract: Testing → 'Testing…' (U+2026 ellipsis) in cyan; Ok{latency_ms} → '✓ ok (Nms)' in green; Err{message} → '✗ <message>' in red — text bytes-for-bytes identical to status_text::DetailStatus rendering
  #   7. Decoration foreground (cyan/green/red) is independent of the row's selection band — the selection band still extends across the full row width and the decoration text inherits the band's background while using its own foreground color
  #   8. Decoration is appended after the row's label separated by a single ASCII space — final visible label is e.g. 'OpenAI API ✓ ok (42ms)'
  #   9. If test_result.provider_id does not match any provider currently in the nav-item flat tree, no decoration is rendered anywhere — silent no-op, no panic
  #   10. set_test_result is last-write-wins — a second call with the same or different provider_id overrides the prior status with no merging
  #   11. Both set_test_result and clear_test_result are PURE state mutations — they never touch selected_index, scroll_offset, mode, filter, filter_mode, expanded, nav_items, or status
  #
  # EXAMPLES:
  #   1. View has two providers (OpenAI, Anthropic), focus on Anthropic; set_test_result('anthropic', Ok{latency_ms: 42}) is called; the rendered Anthropic row contains the visible text 'Anthropic ✓ ok (42ms)'
  #   2. View has two providers (OpenAI, Anthropic); set_test_result('anthropic', Err{message: 'dns resolution failed'}) is called; the rendered Anthropic row contains the visible text 'Anthropic ✗ dns resolution failed' with the '✗' painted with red foreground
  #   3. set_test_result('anthropic', Testing); rendered Anthropic row contains 'Anthropic Testing…' with cyan foreground on the 'Testing…' portion
  #   4. test_result is set for 'openai' with Ok{12} while focus is on Anthropic; OpenAI row contains '✓ ok (12ms)' decoration, Anthropic row does NOT contain '✓' anywhere
  #   5. test_result is None (default state); render to a buffer; no buffer cell contains '✓', '✗', or 'Testing' substrings
  #   6. Anthropic is expanded showing an api_key child row; set_test_result('anthropic', Ok{99}); only the Anthropic Provider header row contains the '✓ ok (99ms)' text — the api_key child row does NOT
  #   7. set_test_result('anthropic', Ok{1}) then clear_test_result(); render; no row contains '✓', '✗', or 'Testing' substrings; view.test_result is None
  #   8. set_test_result('completely-unknown-provider-xyz', Ok{5}); render against a 17-provider canonical list; no row contains a '✓' decoration; no panic occurs
  #   9. set_test_result('anthropic', Testing) then immediately set_test_result('anthropic', Ok{77}); render; the Anthropic row contains '✓ ok (77ms)' and does NOT contain 'Testing'
  #   10. Direct unit: ProviderSettingsView::default(); assert view.test_result is None
  #   11. Direct unit: pre-set selected_index=3, scroll_offset=1, filter='ant', filter_mode=true; call set_test_result('groq', Ok{8}); assert all four of those fields are UNCHANGED after the call
  #   12. Direct unit: with test_result already set, pre-set selected_index=2, scroll_offset=1; call clear_test_result(); assert selected_index and scroll_offset are unchanged; assert test_result is None
  #
  # ========================================

  Background: User Story
    As a user testing a provider connection
    I want to see the test result inline next to the provider row I just tested
    So that I see the outcome in the same place I triggered the test, without leaving the list

  @rendering
  Scenario: Successful test result renders as green "✓ ok (Nms)" appended to the focused provider row
    Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    And the selected_index is on the Anthropic row
    When set_test_result is called with provider_id "anthropic" and status Ok with latency_ms 42
    And the view is rendered into a buffer
    Then the Anthropic row contains the substring "✓ ok (42ms)"
    And the "✓ ok (42ms)" foreground color is Green

  @rendering
  Scenario: Failed test result renders as red "✗ <message>" appended to the focused provider row
    Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    And the selected_index is on the Anthropic row
    When set_test_result is called with provider_id "anthropic" and status Err with message "dns resolution failed"
    And the view is rendered into a buffer
    Then the Anthropic row contains the substring "✗ dns resolution failed"
    And the "✗" character foreground color is Red

  @rendering
  Scenario: In-flight test renders as cyan "Testing…" appended to the focused provider row
    Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    And the selected_index is on the Anthropic row
    When set_test_result is called with provider_id "anthropic" and status Testing
    And the view is rendered into a buffer
    Then the Anthropic row contains the substring "Testing…"
    And the "Testing…" foreground color is Cyan

  @rendering
  Scenario: Test result decoration is scoped to the matching provider_id only
    Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    And the selected_index is on the Anthropic row
    When set_test_result is called with provider_id "openai" and status Ok with latency_ms 12
    And the view is rendered into a buffer
    Then the OpenAI row contains the substring "✓ ok (12ms)"
    And the Anthropic row does NOT contain the character "✓"

  @rendering
  Scenario: No decoration is rendered when test_result is None
    Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    And test_result is None
    When the view is rendered into a buffer
    Then no row in the buffer contains the substring "✓"
    And no row in the buffer contains the substring "✗"
    And no row in the buffer contains the substring "Testing"

  @rendering
  Scenario: Decoration renders on the provider header row only, never on its expanded children
    Given a ProviderSettingsView in List mode with the Anthropic provider expanded and exposing an api_key child row
    When set_test_result is called with provider_id "anthropic" and status Ok with latency_ms 99
    And the view is rendered into a buffer
    Then the Anthropic provider header row contains the substring "✓ ok (99ms)"
    And the api_key child row does NOT contain the substring "✓"

  @rendering
  Scenario: clear_test_result removes the inline decoration on the next render
    Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    And set_test_result has been called with provider_id "anthropic" and status Ok with latency_ms 1
    When clear_test_result is called
    And the view is rendered into a buffer
    Then no row in the buffer contains the substring "✓"
    And no row in the buffer contains the substring "✗"
    And no row in the buffer contains the substring "Testing"
    And view.test_result is None

  @rendering
  Scenario: Unknown provider_id is a silent no-op — no decoration anywhere, no panic
    Given a ProviderSettingsView in List mode populated with all 17 canonical provider display infos
    When set_test_result is called with provider_id "completely-unknown-provider-xyz" and status Ok with latency_ms 5
    And the view is rendered into a buffer
    Then no row in the buffer contains the substring "✓"
    And no panic occurs

  @rendering
  Scenario: set_test_result is last-write-wins — a second call overrides the prior status
    Given a ProviderSettingsView in List mode with two providers OpenAI and Anthropic
    And set_test_result has been called with provider_id "anthropic" and status Testing
    When set_test_result is called again with provider_id "anthropic" and status Ok with latency_ms 77
    And the view is rendered into a buffer
    Then the Anthropic row contains the substring "✓ ok (77ms)"
    And the Anthropic row does NOT contain the substring "Testing"

  @api
  Scenario: A newly-constructed ProviderSettingsView has test_result None
    Given a newly-constructed ProviderSettingsView via ProviderSettingsView::default()
    Then view.test_result is None

  @api
  Scenario: set_test_result does not mutate selected_index, scroll_offset, filter, or filter_mode
    Given a ProviderSettingsView with selected_index 3, scroll_offset 1, filter "ant", and filter_mode true
    When set_test_result is called with provider_id "groq" and status Ok with latency_ms 8
    Then view.selected_index is still 3
    And view.scroll_offset is still 1
    And view.filter is still "ant"
    And view.filter_mode is still true

  @api
  Scenario: clear_test_result does not mutate selected_index or scroll_offset
    Given a ProviderSettingsView with test_result already populated and selected_index 2 and scroll_offset 1
    When clear_test_result is called
    Then view.selected_index is still 2
    And view.scroll_offset is still 1
    And view.test_result is None
