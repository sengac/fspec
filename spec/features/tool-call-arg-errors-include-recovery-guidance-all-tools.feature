@done
@tool-execution
@tool-integration
@rust
@tools
@tool-023
@TOOL-023
Feature: Tool-call arg errors include recovery guidance (all tools)

  # Background: when the LLM calls ANY tool (Fspec, WebSearch, Read, Write,
  # Bash, or any provider-facade tool) with wrong or missing arguments, the
  # error text that lands in the tool result is the LLM's ONLY recovery
  # surface. Errors that name neither the tool nor how to call it correctly
  # force the LLM to re-consult the tool definition or guess. This feature
  # makes the UNIVERSAL layer of arg-related tool-call failures
  # self-explanatory (companion to the dispatcher feature
  # spec/features/tool-call-arg-errors-include-recovery-guidance.feature):
  # name the tool, say exactly what was missing or wrong, list the tool's
  # accepted parameters (full usage: required + optional, types, defaults,
  # enums), and show at least one complete canonical example call.
  #
  # Architecture notes:
  # - DIRECT tools: the patched rig-core `ToolDyn` blanket impl
  #   (rust/patches/rig-core/src/tool/mod.rs) is the single choke point for
  #   every direct tool (WebSearch, Read, Write, Edit, Bash, Ls, Grep,
  #   Glob, AstGrep, AstGrepRefactor, Bridge, DeepSearch, SessionSearch,
  #   GraphSearch, AgentManager, Schedule, Done, RequestUserInput,
  #   InjectSummary, UnifiedExec). On a serde arg-deserialization failure it
  #   wraps the error in a recovery message built by
  #   codelet-common::tool_usage::arg_error_recovery (tool name + original
  #   serde reason verbatim + accepted-parameter reference + canonical
  #   example call), instead of the opaque `JSON error: missing field `X``
  #   passthrough.
  # - FACADE tools: every FacadeToolWrapper call site in
  #   rust/tools/src/facade/wrapper.rs enriches a `map_params`
  #   ToolError::Validation via `enrich_facade_arg_error`, appending the
  #   tool's accepted-parameter reference + canonical example call (rendered
  #   from the facade's definition() schema) to the original message — the
  #   original message is preserved verbatim so existing per-tool substring
  #   assertions keep passing.
  # - The shared schema → usage rendering lives in
  #   codelet-common::tool_usage (single source of truth; no per-tool
  #   duplication).
  Scenario: Calling a direct tool with no arguments names the tool and lists accepted parameters
    Given I have a direct rig tool registered (e.g. WebSearchTool) whose schema declares a required 'action' parameter plus optional parameters
    When I call the tool with an empty argument object (missing required field)
    Then the error returned to the LLM names the tool (e.g. 'WebSearch')
    And the error states which required parameter is missing ('action')
    And the error lists the tool's accepted parameters from its schema (required and optional, with types and valid values where declared)
    And the error shows at least one complete canonical example call that would succeed

  Scenario: Facade tool missing a required parameter names the tool and shows full usage
    Given I call a provider-facade tool (e.g. the Claude WebSearch facade) with arguments missing a required field
    When the facade maps the provider parameters and finds the required field missing
    Then the validation error names the tool and the missing parameter
    And the error includes the tool's accepted parameters (required and optional with types, defaults, and enums) rendered from its definition
    And the error shows at least one complete canonical example call that would succeed

  Scenario: Argument error for any tool (not just Fspec) names the tool, the missing field, and the full usage
    Given I have any agent-exposed tool registered (e.g. WebSearch, Read, Bash, or a facade tool)
    When I call the tool with arguments missing a required field (e.g. WebSearch called with no parameters at all)
    Then the LLM-visible error does NOT stop at the raw serde passthrough (e.g. it is not just 'JSON error: missing field `action` at line 1 column 2')
    And the error names the tool (e.g. 'WebSearch')
    And the error states which field(s) are required and were missing or invalid (e.g. 'action')
    And the error lists the tool's full accepted parameters with types, valid values (enums/oneOf), required-vs-optional, and defaults
    And the error includes at least one complete canonical example call showing how to invoke the tool correctly
