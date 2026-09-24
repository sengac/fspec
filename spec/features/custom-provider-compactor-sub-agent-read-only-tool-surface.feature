@done
@agent-core
@context-management
@compaction
@CMPCT-044
@critical @component @feature-group
Feature: Custom-provider compactor sub-agent read-only tool surface

  """
  CMPCT-044/045 review follow-up: when the compactor sub-agent is built on a Rhai custom provider, CustomProvider::create_rig_agent must attach ONLY the seven read-only infrastructure tools (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) — the `sub_agent: true` gate on the builder's custom-provider dispatch arm strips the full provider surface (GenerateCompaction, InjectSummary, DeepSearch, AgentManager, write tools, Rhai/Fspec/Bridge facades) so the ephemeral compactor cannot recurse into compaction or mutate state. Every parent-agent call site passes `false` and keeps the full surface as before.
  
  Source-shape guard: the contract is pinned by source markers — (1) CustomProvider::create_rig_agent accepts a `sub_agent: bool` gate, (2) the compactor spawner passes `true` on the custom-provider dispatch arm, (3) the sub-agent branch attaches exactly the seven read-only tools and returns early.
  
  Test: rust/cli/tests/cmpct044_subagent_surface_test.rs (Feature header comment points here).
  """

  Background: User Story
    As a long-running agent session on a custom LLM provider
    I want the compactor sub-agent dispatched on my provider to stay on the read-only tool surface
    So that the ephemeral compactor cannot recurse into compaction or mutate state


  Scenario: Custom-provider compactor sub-agent stays on the read-only tool surface
    Given the compactor sub-agent is dispatched for a session on a registered Rhai custom provider
    When the compactor agent is built through the custom-provider path
    Then the built agent exposes exactly the seven read-only tools (Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) and no other tool
    And in particular it has no GenerateCompaction tool (no recursion into compaction), no InjectSummaryTool, no DeepSearchTool, no AgentManagerTool, and no Write/Edit tool

