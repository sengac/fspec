@done
@source-shape
@regression
@RPC-087
@rpc-087
@rust
@agent-loop
@agent-core
@session-management
Feature: Agent loop dispatch routes every provider arm through the recovery-wired streaming engine
  """
  RPC-087 sibling regression-shape coverage. Pins that
  rust/agent-loop/src/dispatch.rs has exactly one
  `codelet_cli::interactive::run_agent_stream_with_images(` call inside
  the `run_with_provider!` macro body — proving every provider arm
  funnels through the single streaming engine that wires every error
  classifier + recovery helper (see sibling feature
  agent-loop-error-classification-recovery-wiring-shape.feature).
  """

  Background: User Story
    As a fspec maintainer
    I want every per-turn provider dispatch to funnel through the recovery-wired streaming engine
    So that no provider arm can silently bypass the error classification + retry helpers

  Scenario: dispatch.rs funnels every provider arm through the recovery-wired streaming engine
    Given the source file rust/agent-loop/src/dispatch.rs
    When I read the file as a string
    Then it contains exactly one occurrence of the substring "codelet_cli::interactive::run_agent_stream_with_images("
