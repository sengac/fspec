@done
@tool-execution
@tool-integration
@rust
@tools
@tool-024
@TOOL-024
Feature: Post-deserialization arg errors include recovery guidance (TOOL-023 follow-up)
  """
  ToolError.tool is &'static str (tools/src/error.rs), so a per-call central enrichment at the blanket ToolDyn impl cannot append usage to Validation/Execution payloads without changing the error type (invasive — ToolError is used across all providers, napi, agent-loop, tests). Preferred direction: enrich at the SOURCE sites (the 6+ direct tools with empty-value checks + Schedule handler) using codelet_common::tool_usage::append_usage_to_message(self.name(), &schema, &msg) where the schema comes from the same definition() as TOOL-023 used. Secondary cleanup: DoneTool tool:'done' -> Self::NAME (done.rs:318/336/349); DeepSearch empty-query ToolError::Execution -> ToolError::Validation (deep_search/mod.rs:459); ScheduleTool must detect ScheduleResult::Error for argument-missing cases and map to enriched ToolError::Validation instead of serializing the whole result to JSON (schedule/mod.rs:167-173); RequestUserInput empty-questions path (request_user_input.rs:334-355). Grep's empty-pattern error reaches the LLM via ToolError::Execution(tool:'grep') from execute()'s ToolOutput::error (grep.rs:178-181, call() map at 462-474).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Post-deserialization argument-validation failures for direct tools (args that serde-parsed successfully but fail an internal empty/missing-value check) MUST return an error containing: (1) the tool name, (2) the specific parameter that was empty or invalid, (3) the tool's accepted-parameter reference (required/optional, types, enums) rendered from its schema, (4) at least one complete canonical example call. Applies at minimum to: Bash (command), Grep (pattern), AstGrep (pattern, language), AstGrepRefactor (pattern, language, source_file), DeepSearch (query), RequestUserInput (questions).
  #   2. The enrichment MUST cover every ToolError variant raised for wrong/missing arguments by direct tools — ToolError::Validation AND ToolError::Execution both count (DeepSearch's empty-query check currently uses ToolError::Execution). An implementation is acceptable ONLY if it guarantees the recovery block on all argument-validation error variants, e.g. by wrapping errors centrally where ToolDyn/ToolSet returns them, or by converting mislabeled variants to ToolError::Validation at the source.
  #   3. Every agent-exposed tool's argument-validation failure MUST surface as a tool error in the standard recovery format (tool name + bad parameter + accepted-parameter reference + example call). The Schedule tool currently returns raw JSON {"success":false,"error":"..."} for add with a missing/empty cron and other handler errors — this MUST be converted to a proper tool error with the recovery block, preserving the existing error message as a substring (other work units may assert 'Cron expression is required').
  #   4. Argument-validation errors MUST identify the tool by its registered rig NAME (Self::NAME / DONE_TOOL_NAME's display name 'Done'), not a lowercase internal alias. DoneTool currently reports tool: 'done' in its ToolError::Validation payloads; this MUST match the tool's registered name 'Done' (existing message substrings about 'summary' etc. stay intact).
  #   5. The tool_usage parameter-reference renderer MUST render union schemas sanely: oneOf with non-object variants (e.g. AgentManager session_id string|array) must list each variant's type instead of empty '* {}' objects; anyOf properties (e.g. ConnectMCP action/transport) must describe the alternatives instead of bare '(any)' with no sub-values. Rendering must never emit empty-object placeholders for variant schemas that have no properties key.
  #
  # EXAMPLES:
  #   1. Bash called with command set to the empty string -> error keeps the substring 'command parameter is required', names the tool 'Bash' (not lowercase 'bash'), appends the accepted-parameter reference (command string required; cwd string|null optional) and a complete example call with a <command> placeholder
  #   2. Grep or AstGrep called with pattern set to the empty string -> error keeps the substring 'pattern parameter is required', names the tool 'Grep'/'AstGrep', lists accepted parameters (pattern required, path/output_mode/glob/limit or language/limit optional with types) and a complete example call
  #   3. AstGrepRefactor called with source_file set to the empty string -> error keeps the substring 'source_file parameter is required', names the tool 'AstGrepRefactor', lists accepted parameters (language, pattern, source_file required; replacement/batch/preview/transforms optional with types) and a complete example call
  #   4. DeepSearch called with query set to the empty string -> error keeps the substring 'query is required and must not be empty', names the tool 'DeepSearch', lists accepted parameters (query required; scope/max_depth/max_recursion_depth optional with types/defaults) and a complete example call; the underlying error variant is argument validation, not an execution failure
  #   5. RequestUserInput called with an empty questions array -> error keeps the substring 'questions array must not be empty', names the tool 'RequestUserInput', lists accepted parameters (questions array required, max 3; each question's id/header/question required, options optional) and a complete example call
  #   6. Schedule add action with no cron expression -> the LLM-visible error is NO LONGER raw JSON {"success":false,"error":"Cron expression is required"}; it names the tool 'Schedule', says 'cron' is required for action add, lists all accepted parameters with types and conditionally-required notes (name, cron, timezone, job_type, role, prompt, command, overlap_policy with default skip), and shows a complete example add call
  #   7. Done tool called with no arguments -> error reads 'Done tool: invalid arguments: missing field `summary`...' (registered name, not lowercase 'done tool'), lists accepted parameters (summary string required; evidence array; goal_assessment string) and an example call; the summary-required message substring stays intact
  #   8. AgentManager called with action get_status and session_id missing -> parameter reference lists session_id as (string | array of string, required) with both allowed types, NOT empty '* {}' object placeholders
  #   9. ConnectMCP called with an invalid action value -> parameter reference renders the anyOf union for 'action' and 'transport' with each alternative described (e.g. the action shape with its required type/url fields) instead of bare '(any)' with no sub-values
  #
  # ========================================
  Background: User Story
    As a LLM agent
    I want to get a self-contained recovery surface from any argument-validation failure
    So that recover from a wrong-args call without re-consulting the tool definition

  Scenario: Bash empty command error names the tool and shows full usage
    # Rule 1, Rule 4
    Given I register the Bash direct tool with a schema declaring required 'command' (string) and optional 'cwd'
    When I call Bash with command set to the empty string
    Then the error contains the substring 'command parameter is required'
    And the error names the tool 'Bash' with its registered name, not a lowercase alias
    And the error lists the accepted parameters (command string required, cwd optional with type)
    And the error shows a complete example call with a <command> placeholder

  Scenario: Grep and AstGrep empty pattern errors name the tool and show full usage
    # Rule 1, Rule 2
    Given I register the Grep and AstGrep direct tools
    When I call Grep with pattern set to the empty string
    Then the error contains the substring 'pattern parameter is required'
    And the error names the tool 'Grep'
    And the error lists accepted parameters (pattern required; path, output_mode, glob, limit optional with types)
    And the error shows a complete example call
    When I call AstGrep with pattern set to the empty string
    Then the error contains the substring 'pattern parameter is required'
    And the error names the tool 'AstGrep' and lists its accepted parameters and a complete example call

  Scenario: AstGrepRefactor empty source_file error names the tool and shows full usage
    # Rule 1, Rule 2
    Given I register the AstGrepRefactor direct tool
    When I call AstGrepRefactor with source_file set to the empty string
    Then the error contains the substring 'source_file parameter is required'
    And the error names the tool 'AstGrepRefactor'
    And the error lists accepted parameters (language, pattern, source_file required; replacement, batch, preview, transforms optional with types)
    And the error shows a complete example call

  Scenario: DeepSearch empty query error is argument validation with full usage
    # Rule 1, Rule 2
    Given I register the DeepSearch direct tool
    When I call DeepSearch with query set to the empty string
    Then the error contains the substring 'query is required and must not be empty'
    And the error names the tool 'DeepSearch'
    And the error is an argument-validation failure, not an execution failure
    And the error lists accepted parameters (query required; scope, max_depth, max_recursion_depth optional with types and defaults)
    And the error shows a complete example call

  Scenario: RequestUserInput empty questions error names the tool and shows full usage
    # Rule 1, Rule 2
    Given I register the RequestUserInput direct tool
    When I call RequestUserInput with an empty questions array
    Then the error contains the substring 'questions array must not be empty'
    And the error names the tool 'RequestUserInput'
    And the error lists accepted parameters (questions array required, maximum 3; each question id, header, question required; options optional)
    And the error shows a complete example call

  Scenario: Schedule add without cron returns a tool error with full usage
    # Rule 3, Rule 4
    Given I register the Schedule direct tool with a schema for actions add, list, pause, resume, remove
    When I call Schedule with action add and a name but no cron expression
    Then the LLM-visible error is not the raw serialized JSON blob {"success":false,"error":"..."}
    And the error contains the substring 'Cron expression is required'
    And the error names the tool 'Schedule'
    And the error states that 'cron' is required for the add action
    And the error lists all accepted parameters with types and conditionally-required notes (name, cron, timezone, job_type, role, prompt, command, overlap_policy with default skip)
    And the error shows a complete example add call

  Scenario: Done tool arg error uses the registered tool name
    # Rule 4
    Given I register the Done direct tool whose registered name is 'Done'
    When I call Done with no arguments
    Then the error names the tool 'Done', not a lowercase alias
    And the error keeps the existing missing-summary message substring intact
    And the error lists accepted parameters (summary string required; evidence array; goal_assessment string)
    And the error shows a complete example call

  Scenario: oneOf with non-object variants renders each variant type
    # Rule 5
    Given a tool whose schema declares a required property 'session_id' as oneOf string or array of string
    When I call the tool with session_id missing
    Then the parameter reference lists session_id with both allowed types (string and array of string)
    And the parameter reference contains no empty object placeholder for a variant that has no properties

  Scenario: anyOf properties render their alternatives
    # Rule 5
    Given a tool whose schema declares properties 'action' and 'transport' as anyOf unions
    When I call the tool with an invalid 'action' value
    Then the parameter reference describes each alternative of 'action' (e.g. the action shape with its required type and url fields)
    And the parameter reference does not render 'action' or 'transport' as bare '(any)' with no sub-values
