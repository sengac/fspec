@done
@scroll
@e2e
@navigation
@model-selector
@tui
@PROV-104
Feature: Model view scroll/viewport parity with TypeScript

  """
  End-to-end tier (tui-test): launches the real fspec binary in a PTY and drives the /model view with real keystrokes, complementing the unit-level scroll geometry tests in model-view-scroll-viewport.feature. Data is made deterministic by pointing FSPEC_USER_DIR at a temp dir containing an fspec-config.json with a local-server (openai) profile carrying several customModels, so build_local_profile_sections() yields selectable rows offline without credentials or network. Key path under test: crossterm EventStream -> App::handle_event -> Navigator::handle_event(ViewMode::ModelSelector) -> handle_model_selector_event -> ModelSelectorView::handle_key (Up/Down -> move_up/move_down -> adjust_scroll). Rows are populated asynchronously via backend.list_providers() returning Action::ListProvidersLoaded; tests must wait for rows to render before asserting movement.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The scroll indicator (up/down/scrollbar) renders in a dedicated column beside the list and never overwrites a content row
  #   2. The body slices the full visible window so every visible row paints content; the selected row is always painted within the viewport
  #   3. Navigating to the bottom edge of a list longer than the viewport keeps the selected row painted on the last visible content row
  #   4. Navigating to the top edge keeps the selected row painted on the first visible content row
  #   5. PageDown and PageUp move the selection by one viewport height and keep the selected row painted within the viewport
  #   6. The /model view must be verified end-to-end with tui-test against the real fspec binary, not only with unit tests that inject pre-populated rows
  #   7. On a freshly built binary, pressing Up/Down in the /model view must move the highlighted model row when selectable model rows exist
  #   8. The /model view populates rows asynchronously (backend.list_providers); the view must surface a loading/empty/error state so the user is never left with a silently inert, blank list
  #   9. When the row projection contains only non-selectable provider headers (no models loaded), Up/Down has nothing to land on; this state must be distinguishable from a populated list
  #   10. A failed list_providers() call must not be silently swallowed; it must produce a user-visible error/empty state
  #   11. Option B: seed a local-server profile with a custom model in a temp HOME (~/.fspec/fspec-config.json) so build_local_profile_sections() yields selectable rows offline and deterministically, independent of network/credentials.
  #
  # EXAMPLES:
  #   1. With a 30-model list in a 10-row viewport, pressing Down past the bottom paints the selected model row as the last visible content row
  #   2. After scrolling down then pressing Up back to the first model, the selected model row is painted on the first content row and offset returns to 0
  #   3. Pressing End on a tall list paints the last model row at the bottom edge with no inline arrow overwriting it
  #   4. A mid-list selection is painted within the viewport and not stolen by an overflow indicator
  #   5. Pressing PageDown advances the selection by one viewport height and the new selected row is painted within the viewport
  #   6. When the list overflows the viewport, a scrollbar column is painted beside the list and the rightmost content column still shows model text
  #   7. e2e: launch the real fspec binary, open a Work Agent, type /model, wait for model rows to render, press Down — the highlighted row moves to the next selectable model
  #   8. e2e: with a list taller than the viewport, press Down repeatedly to the bottom edge — the selected model row stays painted within the viewport (RPC-340/PROV-104 scroll-follow)
  #   9. e2e: open /model before providers finish loading — the view shows a loading indicator, not a blank inert list
  #   10. e2e: open /model with no models available — the view shows an explicit empty/'no models' state rather than appearing to ignore arrow keys
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should the e2e tui-test get deterministic, selectable model rows? (A) --features test-stub-provider, (B) seed a local-server profile/custom model in a temp ~/.fspec config, or (C) set a provider API key env var relying on models.dev catalog?
  #   A: Option B: seed a local-server profile with a custom model in a temp HOME (~/.fspec/fspec-config.json) so build_local_profile_sections() yields selectable rows offline and deterministically, independent of network/credentials.
  #
  # ========================================

  Background: User Story
    As a fspec TUI user selecting a model
    I want to move the highlight with Up/Down in the /model view and have the list follow my cursor
    So that I can actually pick a model in the real running binary, not just in unit tests

  Scenario: Pressing Down in the live /model view moves the highlight to the next model
    Given the fspec binary is launched with FSPEC_USER_DIR pointing at a temp config containing a local-server profile with several custom models
    When I press the Down arrow
    Then the highlighted model row moves to the next selectable model
    And I open a Work Agent and submit "/model"
    And the model rows have rendered with at least two selectable models


  Scenario: Down to the bottom of a tall list keeps the selected model painted in the viewport
    Given the fspec binary is launched with FSPEC_USER_DIR pointing at a temp config whose local-server profile has more custom models than fit the viewport
    When I press the Down arrow repeatedly to the last model
    Then the last model row is visible in the rendered viewport and remains highlighted
    And I open a Work Agent and submit "/model" and the model rows have rendered


  Scenario: Fresh open with all sections collapsed lets Down reach a section to expand
    Given the fspec binary is launched with a temp config whose local-server profile carries custom models and no current model is set so every section opens collapsed
    When I press the Down arrow to move the cursor onto the custom-model profile header and press the Right arrow to expand it
    Then the profile's custom models become visible in the list
    And I open the /model view and only collapsed provider headers are shown

