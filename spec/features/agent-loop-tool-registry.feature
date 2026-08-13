@done
@session-management
@RPC-083
@rust
@agent-loop
@rpc
@tools
Feature: Agent loop registers tools and executes them end-to-end
  """
  RPC-083 (child of RPC-072 family). The tool registry must reach the
  rig agent via create_rig_agent + RigAgent::with_default_depth so the
  agent can Read/Write/Edit/Bash. MCP wrapper gathering via
  codelet_tools::gather_mcp_tool_wrappers + set_mcp_tool_server_handle
  must run inside each provider arm before the rig agent is wrapped.

  Originally scenario "Tools are registered and executable end-to-end"
  from rpc072-work-agent-roundtrip.feature.
  """

  Background: User Story
    As a fspec user
    I want the Work Agent to invoke Read/Write/Edit/Bash tools
    So that the assistant can act on my filesystem like the TS Ink frontend does

  Scenario: run_with_provider! macro body contains the five canonical tool-registry calls in order
    Given the source file rust/agent-loop/src/dispatch.rs
    When I locate the run_with_provider! macro body
    Then the body contains `codelet_tools::gather_mcp_tool_wrappers($session.id)`
    And the body contains `provider.create_rig_agent($session.id, role_preamble.as_deref(), $thinking.clone())`
    And the body contains `agent.tool_server_handle.add_tool(wrapper).await` inside a non-empty wrappers guard
    And the body contains `codelet_tools::set_mcp_tool_server_handle($session.id, agent.tool_server_handle.clone())`
    And the body contains `codelet_core::RigAgent::with_default_depth(agent)`
    And those five calls appear in the order: gather → create_rig_agent → add_tool loop → set_mcp_tool_server_handle → with_default_depth

  Scenario: OpenAI inlined arm in agent_loop.rs mirrors the macro body with the same five canonical calls
    Given the source file rust/agent-loop/src/agent_loop.rs
    When I locate the "openai" => match arm body
    Then the arm contains `codelet_tools::gather_mcp_tool_wrappers(session.id)`
    And the arm contains `provider.create_rig_agent(session.id, role_preamble.as_deref(), thinking_config_value.clone())`
    And the arm contains `agent.tool_server_handle.add_tool(wrapper).await`
    And the arm contains `codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())`
    And the arm contains `codelet_core::RigAgent::with_default_depth(agent)`

  Scenario: Custom-provider fallthrough arm wraps CustomProvider::create_rig_agent with the four follow-up calls
    Given the source file rust/agent-loop/src/agent_loop.rs
    When I locate the `_ =>` fallthrough match arm body
    Then the arm calls `codelet_providers::custom::CustomProvider::create_rig_agent`
    And the arm contains `codelet_tools::gather_mcp_tool_wrappers(session.id)`
    And the arm contains `agent.tool_server_handle.add_tool(wrapper).await`
    And the arm contains `codelet_tools::set_mcp_tool_server_handle(session.id, agent.tool_server_handle.clone())`
    And the arm contains `codelet_core::RigAgent::with_default_depth(agent)`

  Scenario: Every provider's create_rig_agent accepts the (session_id, preamble, thinking) signature the macro relies on
    Given the providers crate exposes ClaudeProvider, OpenAIProvider, GeminiProvider, ZaiProvider, CodexProvider, CopilotProvider
    When I take a closure reference to each provider's `create_rig_agent` method
    Then each closure compiles with parameters `(provider, session_id: uuid::Uuid, preamble: Option<&str>, thinking: Option<serde_json::Value>)`
    And the signature is stable across all six built-in providers

  Scenario: RigAgent::with_default_depth accepts any rig Agent built by create_rig_agent
    Given codelet_core::RigAgent::with_default_depth has signature `fn(Agent<M>) -> RigAgent<M>`
    When I take a closure reference to RigAgent::with_default_depth
    Then the closure compiles for the rig::agent::Agent returned by every provider's create_rig_agent

  Scenario: gather_mcp_tool_wrappers and set_mcp_tool_server_handle have stable session-id signatures
    Given codelet_tools exposes `gather_mcp_tool_wrappers` and `set_mcp_tool_server_handle`
    When I take closure references to both functions
    Then gather_mcp_tool_wrappers compiles as `fn(uuid::Uuid) -> Vec<McpToolWrapper>`
    And set_mcp_tool_server_handle compiles as `fn(uuid::Uuid, ToolServerHandle)`

  Scenario: gather_mcp_tool_wrappers returns an empty Vec when no MCP servers are connected for the session
    Given a freshly minted session id with no MCP servers connected
    When I call `codelet_tools::gather_mcp_tool_wrappers(session_id)`
    Then the returned vector is empty
    And the function is callable from the codelet-agent-loop crate's test scope
