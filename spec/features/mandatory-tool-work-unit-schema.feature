@TOOL-025
@tools
@providers
@tool-execution
@schema-design
@rust
Feature: Every executable agent tool advertises mandatory work-unit attribution
  """
  Every argument object has a required top-level non-null work_unit_id
  string: an existing project ID or exact NONE. It is harness metadata, not a
  business argument. Preserve remote schemas and reject reserved-name collisions.
  Shared per-agent policy decorates static, dynamic, manually implemented and
  embedding tools, including hot-added MCP tools and auxiliary agent builders.
  Strict mode keeps Codex patch execution as a JSON function and does not expose
  hosted native execution without a model-supplied attribution channel.
  """

  Background: User Story
    As an AI agent
    I want one mandatory attribution contract on every executable tool
    So that the harness can track my card or explicitly untracked activity

  Scenario: Built-in and completion tools advertise the same required attribution
    Given a normal agent toolset including file shell input and completion tools
    When their model-visible definitions are requested
    Then every definition requires top-level "work_unit_id" as a non-null string
    And the original business arguments remain intact
    And descriptions explain that "NONE" clears activity rather than retaining it

  Scenario: Strict remote tool schemas preserve business work-unit arguments
    Given a remote tool with strict arguments and its own "target_work_unit_id" property
    When its definition is decorated for harness attribution
    Then its business work-unit argument retains its original meaning
    And required lists references and additional-property restrictions are preserved
    And repeated decoration does not add duplicate requirements

  Scenario: A reserved metadata collision fails explicitly
    Given a tool whose business schema already owns top-level "work_unit_id"
    When the agent requests its attributed definition
    Then an actionable collision error prevents presenting a corrupted schema
    And the remote business property is not silently overwritten

  Scenario: Newly added tools and rebuilt agents cannot omit attribution
    Given an agent with static dynamic and manually implemented tools
    When an MCP tool is registered mid-turn and the agent is rebuilt for another model
    Then every newly exposed definition retains the attribution requirement
    And read-only auxiliary agents use the same advertised contract
