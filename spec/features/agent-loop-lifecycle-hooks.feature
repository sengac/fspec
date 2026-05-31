@wip
@deferred
@session-management
@RPC-090
@rust
@agent-loop
@rpc
@lifecycle-hooks
@hook013
Feature: Agent loop invokes lifecycle hooks at canonical call sites
  """
  RPC-090 (child of RPC-072 family). Lifecycle hooks (session_start,
  user_prompt_submit, post_tool_use, session_end) must be invoked at
  the same points as the NAPI loop — via codelet_core::lifecycle_hooks —
  so hook configs and HOOK-013 keep working.

  Originally scenario "Lifecycle hooks fire at canonical call sites"
  from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want my configured lifecycle hooks to fire at the same call sites the TS Ink frontend uses
    So that user_prompt_submit hook configs still mutate prompts end-to-end in the Rust binary

  Scenario: Lifecycle hooks fire at canonical call sites
    Given a Work Agent session has a configured user_prompt_submit hook that prepends "[hooked] "
    And the stub provider records the prompt it received
    When the user sends "hello"
    Then the stub provider received "[hooked] hello"
    And the session_start hook fired exactly once before the first turn
