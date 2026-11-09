@done
@tui
@agent-view
@navigation
@rust
@BUG-283
Feature: Auxiliary views return to stale or empty agent instead of their navigation origin
  """
  Navigator owns a transient live excursion origin shared by Providers, Models, Blocklist, full-screen Files and Checkpoints. Repeated opens and Providers/Models sibling switches retain that origin; matching final closes consume it, inactive closes do nothing, and missing origin falls back to Board. Existing stores retain their context. Mux pane-local close remains separate. Do not infer origin from persisted mux preferences. Tests drive the real App and MockBackend in isolated synthetic directories; preserve explicit model-selection backend contracts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Closing a full-screen auxiliary view must restore the surface that most recently opened the excursion (Board, Agent, or Mux), preserving its context rather than choosing a hard-coded Agent or Board destination; reopening from another surface must replace the previous origin.
  #   2. Switching between Providers and Models must preserve the original excursion caller across repeated Tab switches. Local Escape behavior for filters, details, forms, confirmations, and Mux pane dismissal must remain unchanged; restoring the caller must not create a session.
  #   3. Files, Checkpoints and Blocklist full-screen dismissal must use the same live-caller routing as Providers and Models. Pane-local Files/Checkpoints Escape in Mux continues to remove only the focused pane. Unrelated dialogs and the board Enter chooser are outside this change.
  #   4. Repeated auxiliary opens cannot overwrite the excursion origin with an auxiliary screen. Close actions for inactive screens are ignored. A missing recorded origin safely returns to Board; explicitly navigating to Board or Agent ends the old excursion. Persisted mux enabled and pre-mux-view preferences are not caller evidence.
  #   5. Navigation restoration itself performs no session creation, resume, assignment or model write. Explicit model commitment retains the existing RPC-022/PROV-118/MODEL-006 contract: with a session update that session; without one persist the default and perform the already-specified explicit creation retry. Agent-origin close/commit still returns to Agent, now with an explicit Agent precondition in existing regressions.
  #
  # EXAMPLES:
  #   1. On Board with no agent sessions, open Settings > Providers (or press P), then Escape from the root list: return to the same Board column/card selection without creating a session, rather than showing an empty Agent.
  #   2. Visit Providers from Agent A and close it, return to Board, then open Providers again and press Escape: return to Board, not the previously visited Agent A.
  #   3. From a Mux layout with a known focused pane, open Providers, Tab to Models and back repeatedly, then Escape: restore the same Mux layout and focused pane instead of a single Agent view.
  #   4. Agent A has a work-unit assignment, draft and scrolled transcript: open and Escape from Providers, Models and Blocklist, retaining identity, assignment, draft and scroll; do not call create, resume, assignment or model-write backends for dismissal.
  #   5. From Board, Agent or Mux, Tab between Providers and Models repeatedly and commit a model for an existing session: return to that original view and update only the explicitly targeted session model. Closing without commit performs no model change. Choosing a default without a session keeps the existing explicit default-and-create retry behavior.
  #   6. Full-screen Files, Checkpoints and Blocklist opened from each supported caller return to that caller; provider filters/details/forms/confirmations and model filtering consume Escape locally, while Files/Checkpoints Escape within Mux still closes only its pane.
  #   7. Repeatedly opening Providers during one excursion still returns to its original caller. Closing a different or already-dismissed screen has no effect. Closing an auxiliary screen with no recorded caller returns safely to Board; leaving an excursion explicitly and starting another uses the new caller, even when the saved mux preference remains enabled.
  #
  # ========================================
  Background: User Story
    As a fspec TUI user
    I want to return from auxiliary screens to the view I most recently left
    So that my board selection, agent context and multiplex layout remain intact without unexpected session changes

  Scenario: Board without sessions returns from Providers
    Given Board has a selected card and no sessions
    When I open Providers using the board key or menu and press Escape
    Then the same Board selection returns without session or model changes

  Scenario: Latest caller replaces a previous Agent visit
    Given I previously visited Providers from Agent A and returned to Board
    When I open Providers again and press Escape
    Then Board returns instead of the retained Agent A

  Scenario: Agent dismissal preserves identity draft assignment and scroll
    Given Agent A has an assignment draft and scrolled transcript
    When I open and dismiss Providers Models and Blocklist
    Then Agent A retains all context without implicit backend writes

  Scenario: Mux dismissal preserves layout panes and focus
    Given Mux has multiple panes a custom split and a focused pane
    When I open Providers from the mux menu and press Escape
    Then the same Mux layout panes window and focus return

  Scenario: Repeated Providers and Models switches retain the excursion caller
    Given Providers is opened from Board Agent or Mux
    When I Tab between Providers and Models repeatedly and Escape from either screen
    Then the original caller returns without changing sessions or models

  Scenario: Model commitment restores the caller and preserves explicit model semantics
    Given Models was reached through Providers from Board Agent or Mux with Agent A open
    When I commit a model for Agent A
    Then the original caller returns and only the targeted session model is updated

  Scenario: Files Checkpoints and Blocklist restore their full-screen caller
    Given Board Agent or Mux is the active caller
    When I open and Escape from full-screen Files Checkpoints and Blocklist
    Then each dismissal restores that caller without session side effects

  Scenario: Local Escape hierarchy does not end the excursion early
    Given Providers or Models has an active filter detail form or confirmation
    When I press Escape through the local hierarchy and finally dismiss the root screen
    Then local cancellation stays in the screen and final dismissal restores Board

  Scenario: Mux pane Escape remains local after an auxiliary excursion
    Given Mux has loaded Files and Checkpoints panes after visiting Providers
    When I press Escape in each focused tool pane
    Then only the focused pane closes and the saved layout remains unchanged

  Scenario: Repeated opens and inactive closes do not corrupt the origin
    Given Board opened an auxiliary screen
    When the same screen is reopened and mismatched or duplicate closes arrive
    Then inactive closes do nothing and the matching close returns to Board once

  Scenario: Missing origin and explicit navigation use a safe fresh caller
    Given an auxiliary screen has no recorded caller or an earlier excursion was left explicitly
    When I dismiss the screen or open and dismiss another excursion with a saved mux preference
    Then missing origin lands on Board and new excursions restore only their live caller
