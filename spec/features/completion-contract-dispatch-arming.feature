@done
@cli
@codelet
@completion
@session
@CONT-009
Feature: Completion contract dispatch-site arming across agent-loop twins
  """
  AS-BUILT (CONT-009): the dispatch-site arming block lives in a single shared
  helper, BackgroundSession::sync_completion_contract_for_user_turn
  (codelet/sessions/src/background_session.rs), owned by codelet-sessions so it
  needs zero new dependency edges and cannot violate the no-codelet-agent-loop
  constraint in napi/Cargo.toml. Before each dispatched real user message the
  helper: (1) syncs the /continue chrome state into the inner Session and resets
  the per-turn zero-progress nudge counter (reset_for_new_user_turn); (2) syncs
  the /goal chrome state into the inner Session, applying only on change so an
  unchanged goal never re-calls set_goal (which would reset done_rejections and
  re-inject the CompletionContract reminder) — since CONT-008 this apply is
  additionally generation-gated as the goal-resurrection guard; (3) arms the
  per-session done() registry (codelet_tools::set_continue_armed) iff the
  continue toggle is on OR a goal is set, and syncs the goal spec into the
  registry (codelet_tools::set_session_goal). BOTH agent-loop twins call the
  helper at their dispatch sites, between inner-lock re-acquisition and
  BackgroundOutput creation: codelet/agent-loop/src/agent_loop.rs (standalone
  fspec binary) and codelet/napi/src/agent_loop.rs (production NAPI/TUI
  surface). Twin parity is pinned by a comment-stripped source-shape test
  (rpc082/083 precedent) in
  codelet/sessions/tests/cont009_completion_contract_sync.rs, so the twin
  divergence that caused this bug cannot silently recur.
  Historical note: this card originated because the NAPI twin carried no arming
  block at all (auto-continue and /goal were inert on the production surface);
  the original port-plan research is preserved in
  spec/attachments/CONT-009/napi-arming-gap-research.md.
  """

  Background: User Story
    As a TUI user on the production NAPI surface
    I want to have /continue and /goal actually arm the done() completion contract before each dispatched user message
    So that auto-continue and goal enforcement work identically on the NAPI/TUI surface and the standalone fspec binary

  Scenario: Chrome continue state syncs into the inner session and arms the registry
    Given a BackgroundSession with auto-continue enabled and budget 5 via its chrome state
    When the dispatch-site sync helper runs for a real user message
    Then the inner session has continue_enabled true and continue_budget 5
    And the done() registry reports the session as armed

  Scenario: A goal alone arms the registry and registers the goal spec
    Given a BackgroundSession with the continue toggle off and a chrome goal with text and verify command
    When the dispatch-site sync helper runs for a real user message
    Then the inner session goal matches the chrome goal text and verify command
    And the done() registry reports the session as armed
    And the done() registry returns the goal spec with the same text and verify command

  Scenario: Neither continue nor goal leaves the registry disarmed
    Given a BackgroundSession with the continue toggle off and no chrome goal
    When the dispatch-site sync helper runs for a real user message
    Then the done() registry reports the session as disarmed
    And the done() registry returns no goal spec

  Scenario: A new real user turn resets the zero-progress nudge counter
    Given a BackgroundSession whose inner session consumed 3 zero-progress nudges in the previous turn
    When the dispatch-site sync helper runs for a real user message
    Then the inner session has continue_nudges_used 0

  Scenario: Clearing the chrome goal clears the inner goal and disarms the registry
    Given a BackgroundSession whose inner session and registry carry a previously synced goal
    And the chrome goal has since been cleared with the continue toggle off
    When the dispatch-site sync helper runs for a real user message
    Then the inner session has no goal
    And the done() registry reports the session as disarmed
    And the done() registry returns no goal spec

  Scenario: An unchanged chrome goal is not re-applied
    Given a BackgroundSession whose chrome goal was already synced into the inner session
    And the inner session has recorded 2 done() rejections for that goal
    When the dispatch-site sync helper runs again with the same chrome goal
    Then the inner session still has 2 done() rejections
    And the inner session goal is unchanged

  Scenario: Both agent-loop twins call the shared sync helper at the dispatch site
    Given the production NAPI agent loop source and the standalone agent-loop twin source
    When the dispatch sites between lock re-acquisition and BackgroundOutput creation are inspected
    Then both twins call the shared BackgroundSession sync helper before creating the rig agent
    And neither twin carries a diverged inline copy of the arming block
