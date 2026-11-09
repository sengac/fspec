@done
@MODEL-013
@BUG-455
@session
@thinking-detection
@rust
Feature: Sessions without a chosen thinking level use the model default
  """
  BackgroundSession's base thinking level started at 0 (Off) and session
  creation applied load_default_thinking_level(), which returns Off when
  nothing is saved, so "never chosen" and "explicitly Off" were the same
  value. Since BUG-439 Off is refused on models that cannot turn thinking off
  (Opus 5.5, Fable, Mythos), so every session that never picked a level
  (sub-agents, scheduled sessions) had every turn refused there.
  Decision (user, 2026-11-03): a distinct not-set level.
  - codelet_rpc_types::ThinkingLevel gains ModelDefault (appended); the
  session stores it as THINKING_LEVEL_MODEL_DEFAULT and starts there.
  Creation applies a saved default only when one is saved
  (load_default_thinking_level_opt).
  - effective_turn_thinking_level(base, input) gives None for a ModelDefault
  base unless a keyword asks for a level (or a disable keyword asks for
  Off); None sends no thinking parameter on any provider.
  - get_thinking_level reports ModelDefault; set_thinking_level(ModelDefault)
  clears the level; saving ModelDefault as the default removes the saved
  key. The dialog lists "Model default", /thinking default selects it and
  the header shows "Default".
  Tests: rust/agent-loop/tests/bug455_sessions_without_a_chosen_thinking_level.rs.
  """

  Background: User Story
    As a user (or automation) whose session never chose a thinking level
    I want to have the model use its own default thinking instead of an implied Off
    So that sessions on models that cannot turn thinking off still run, and only an Off I chose is refused

  Scenario: A new session without a saved default has the model-default level
    Given no default thinking level is saved
    When a session is created
    Then its thinking level is ModelDefault

  Scenario: A model-default level sends no thinking parameter whatever the prompt says
    Given a session whose thinking level is ModelDefault
    When turns are prompted with "hello", "ultrathink about this" and "nothink, just answer"
    Then none of the turns sends a thinking parameter

  Scenario: Choosing model default clears an explicit level
    Given a session whose thinking level is High
    When ModelDefault is chosen
    Then its thinking level is ModelDefault again
