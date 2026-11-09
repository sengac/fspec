@done
@BUG-450
@tools
@test-maintenance
@rust
Feature: Schedule tool registration checks read provider module trees
  """
  rust/agent-loop/tests/schedule_tool_test/provider_registration.rs read only
  rust/providers/src/<provider>.rs, but the OpenAI provider was split and its
  create_rig_agent (which registers ScheduleTool::new(session_id)) lives in
  rust/providers/src/openai/agent.rs, so test_schedule_tool_registered_in_openai_provider
  failed although the tool is registered. read_provider_source now composes
  the module tree with codelet_test_helpers::source_composition::read_tree_or_file
  (BUG-418), like the other source-shape pins.
  Tests: rust/agent-loop/tests/bug450_schedule_registration_reads_module_trees.rs.
  """

  Background: User Story
    As a maintainer relying on the schedule tool registration checks
    I want to have the checks read each provider's whole module tree
    So that a provider split into submodules is still checked instead of failing falsely

  Scenario: The OpenAI provider's module tree registers the schedule tool
    Given the OpenAI provider source rust/providers/src/openai.rs and its submodules
    When the module tree is read
    Then it registers ScheduleTool::new(session_id)
