@done
@RPC-072
@rust
@agent-loop
@rpc
@boundary
@regression
Feature: codelet-agent-loop has zero codelet-napi dependency after Phase A lift
  """
  RPC-072 Phase A foundation refit (this card): the NAPI-free
  codelet-agent-loop crate must not depend on codelet-napi after the
  building-block lift (BackgroundOutput, run_with_provider!,
  InputWithImages, dispatch table, deep_search_handler,
  graph_search_handler, agent_manager_handler, schedule_handler,
  session_search_handler, inject_summary_handler, persist, thinking_config,
  thinking_level_detection, stream_chunk_json, bridges helpers).

  The forbidden-arrow invariant lets the Rust fspec binary reach the
  agent surface through the NAPI-free crate graph (codelet-fspec →
  codelet-agent-loop → codelet-sessions / codelet-graph / codelet-core /
  codelet-tools / codelet-cli / codelet-providers) without ever pulling
  codelet-napi as a transitive package.

  Body-port work (tokio::select! loop, lifecycle hooks, recovery,
  persistence call-sites, MCP injection drain, etc.) is split across
  follow-up cards RPC-080..RPC-091 — each owns its own feature file.
  """

  Background: User Story
    As a fspec developer
    I want codelet-agent-loop to be a true NAPI-free home for the building blocks the agent loop body will call into
    So that the Rust fspec binary's crate graph stays free of codelet-napi while still reaching every NAPI-side helper through the lifted modules

  Scenario: codelet-agent-loop has zero dependency on codelet-napi after the lift
    Given the codelet-agent-loop crate exists under codelet/agent-loop/
    When cargo metadata is invoked for the codelet-agent-loop package
    Then the transitive package set does not contain "codelet-napi"
    And no .rs file under codelet/agent-loop/src/ contains the substring "codelet_napi"
