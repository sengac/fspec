@done
@facade
@provider-abstraction
@rust
@dispatch
@tools
@providers
@PROV-069
Feature: Extend custom provider maps_to dispatch to non-file categories
  """
  Extends the existing default_to_internal_file pattern in codelet/providers/src/custom/tool_facade.rs. A new tool_dispatch module adds default_to_internal_<category> functions for bash, search:grep, search:glob, ls, web_search:search, fspec, bridge, exec:run, and hitl. A top-level default_to_internal(maps_to, params) enum-returning dispatcher routes to each category function. Conversion errors surface as CustomProviderError::RhaiRuntimeError. Routes map onto Internal*Params structs already exported from codelet_tools::facade.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The dispatch module provides one default_to_internal_<category> function per maps_to category listed in KNOWN_MAPS_TO
  #   2. Each dispatch function converts a serde_json::Value params object into the matching Internal* params struct from codelet_tools::facade
  #   3. Malformed params yield CustomProviderError::RhaiRuntimeError with a message naming the maps_to category
  #   4. A top-level default_to_internal dispatcher routes any KNOWN_MAPS_TO value to its category-specific dispatch function
  #   5. Unknown maps_to values return a clear error listing valid identifiers
  #
  # EXAMPLES:
  #   1. A tool with maps_to bash and params {command: ls} dispatches to InternalBashParams::Execute with command ls
  #   2. A tool with maps_to search:grep and params {pattern: foo, path: src} dispatches to InternalSearchParams::Grep
  #   3. A tool with maps_to search:glob and params {pattern: *.rs} dispatches to InternalSearchParams::Glob
  #   4. A tool with maps_to ls and params {path: /tmp} dispatches to InternalLsParams::List
  #   5. A tool with maps_to web_search:search and params {query: rust} dispatches to InternalWebSearchParams::Search
  #   6. A tool with maps_to fspec and params {command: board args: {} project_root: .} dispatches to InternalFspecParams
  #   7. A tool with maps_to bridge and params {action: {type: list}} dispatches to InternalBridgeParams::List
  #   8. A tool with maps_to exec:run and params {command: ls} dispatches to InternalExecParams::Run
  #   9. A tool with maps_to hitl and params {questions: [...]} dispatches to InternalHitlParams::Request
  #   10. Unknown maps_to value returns error whose message lists valid identifiers
  #
  # ========================================
  Background: User Story
    As a custom provider author
    I want to define tools with non-file maps_to targets
    So that custom providers can route tool calls to bash search ls web_search fspec bridge exec and hitl internal tools

  Scenario: Dispatch bash maps_to to InternalBashParams::Execute
    Given a params JSON object with command set to "ls"
    When I call default_to_internal with maps_to "bash"
    Then the result is an InternalBashParams::Execute whose command is "ls"

  Scenario: Dispatch search:grep maps_to to InternalSearchParams::Grep
    Given a params JSON object with pattern "foo" and path "src"
    When I call default_to_internal with maps_to "search:grep"
    Then the result is an InternalSearchParams::Grep whose pattern is "foo" and path is Some("src")

  Scenario: Dispatch search:glob maps_to to InternalSearchParams::Glob
    Given a params JSON object with pattern "*.rs"
    When I call default_to_internal with maps_to "search:glob"
    Then the result is an InternalSearchParams::Glob whose pattern is "*.rs"

  Scenario: Dispatch ls maps_to to InternalLsParams::List
    Given a params JSON object with path "/tmp"
    When I call default_to_internal with maps_to "ls"
    Then the result is an InternalLsParams::List whose path is Some("/tmp")

  Scenario: Dispatch web_search:search maps_to to InternalWebSearchParams::Search
    Given a params JSON object with query "rust"
    When I call default_to_internal with maps_to "web_search:search"
    Then the result is an InternalWebSearchParams::Search whose query is "rust"

  Scenario: Dispatch fspec maps_to to InternalFspecParams
    Given a params JSON object with command "board", args "{}" and project_root "."
    When I call default_to_internal with maps_to "fspec"
    Then the result is an InternalFspecParams with command "board"

  Scenario: Dispatch bridge maps_to to InternalBridgeParams::List
    Given a params JSON object with action.type set to "list"
    When I call default_to_internal with maps_to "bridge"
    Then the result is an InternalBridgeParams::List variant

  Scenario: Dispatch exec:run maps_to to InternalExecParams::Run
    Given a params JSON object with command "ls"
    When I call default_to_internal with maps_to "exec:run"
    Then the result is an InternalExecParams::Run whose command equals the input command

  Scenario: Dispatch hitl maps_to to InternalHitlParams::Request
    Given a params JSON object with a questions array containing one valid HitlQuestion
    When I call default_to_internal with maps_to "hitl"
    Then the result is an InternalHitlParams::Request whose questions vec has length 1

  Scenario: Unknown maps_to value returns error listing valid identifiers
    Given any params JSON object
    When I call default_to_internal with maps_to "mystery:foo"
    Then I receive a CustomProviderError whose message contains "mystery:foo" and "bash"
