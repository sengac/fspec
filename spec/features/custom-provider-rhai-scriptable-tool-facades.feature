@done
@validator
@facade
@tools
@rust
@providers
@PROV-066
Feature: Custom provider Rhai-scriptable tool facades
  """
  RhaiToolFacadeAdapter is a getters-only adapter in rust/providers/src/custom/tool_facade.rs (not a full rig::Tool impl, since rig::Tool requires a const NAME incompatible with runtime-defined Rhai tool names). Rhai define_tools/map_tool_params are optional; tool_style presets are static lookup tables; maps_to identifiers route downstream to internal tool dispatchers in rust/tools
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. RhaiToolFacadeAdapter is a getters-only Rust struct (not a full rig-Tool impl) whose name()/parameters_schema()/maps_to() getters surface the RhaiToolDef so downstream code can bridge into rig-compatible request builders
  #   2. define_tools(config) Rhai function returns an array of tool definitions each with name, description, parameters (JSON schema), and maps_to string identifier
  #   3. maps_to identifiers include: file:read, file:write, file:edit, bash, search:grep, search:glob, ls, web_search:search, fspec, bridge, exec:run, hitl
  #   4. When define_tools is not defined, a tool_style preset (claude, openai, gemini, codex) determines the default tool set; default tool_style is claude
  #   5. tool_style preset 'openai' yields snake_case tool names (read_file, write_file, edit_file, run_bash, etc.)
  #   6. map_tool_params(config, tool_name, maps_to, params) Rhai function transforms LLM-supplied params; returning () (Rhai unit) means use default field-by-field mapping
  #   7. If define_tools returns tools only for file:read and bash maps_to targets, the LLM only sees read and bash tools; other categories are hidden
  #   8. Rhai errors in define_tools or map_tool_params are logged and surface as ToolError rather than panicking
  #   9. Resolving the tool list caches the final Vec<RhaiToolDef> on ProviderConfig (resolved_tools) so system prompt functions can reference it
  #
  # EXAMPLES:
  #   1. A script defining define_tools that returns a list including {name:'read_file', maps_to:'file:read', description:'...', parameters:{...}} produces a RhaiToolFacadeAdapter whose tool definition uses name 'read_file' with maps_to 'file:read'
  #   2. A config with tool_style 'openai' and no define_tools produces default tools: read_file, write_file, edit_file, run_bash, grep_search, glob_search, list_dir, web_search
  #   3. A config with no tool_style and no define_tools uses the 'claude' preset tools (Read, Write, Edit, Bash, Grep, Glob, LS, WebSearch)
  #   4. A script defining map_tool_params that renames 'filepath' to 'file_path' produces InternalFileParams::Read with correct file_path when called
  #   5. A script's map_tool_params returning () uses default field-by-field mapping (tool params JSON fed directly to serde_json::from_value for InternalFileParams)
  #   6. A define_tools returning only file:read and bash maps_to entries exposes exactly two RhaiToolDef entries and thus two adapter instances to the LLM
  #   7. A Rhai script whose define_tools throws a runtime error surfaces as a ToolError and falls back to the tool_style preset
  #   8. A tool definition with unknown maps_to identifier 'mystery:foo' is rejected at resolve time with a clear error listing valid identifiers
  #   9. RhaiToolFacadeAdapter.name() returns the tool name from the RhaiToolDef and .parameters_schema() returns the JSON schema that came from the Rhai script
  #   10. Resolving tools stores the final list on a shared cache accessible via ProviderConfig.resolved_tools so system prompt functions can introspect tool names
  #
  # ========================================
  Background: User Story
    As a custom provider author
    I want to define which tools the LLM sees and how parameter names map to internal tool types via optional Rhai functions
    So that my provider can present custom tool names and schemas (e.g., read_file/Read) without recompilation, routing calls to built-in tool implementations

  Scenario: define_tools produces custom tool definitions
    Given a Rhai script whose define_tools returns a list containing a read_file entry with maps_to "file:read"
    When I resolve the tool list for that provider
    Then the resolved list contains a RhaiToolDef with name "read_file" and maps_to "file:read"

  Scenario: tool_style openai preset generates snake_case tool names
    Given a ProviderConfig with tool_style "openai" and no define_tools function
    When I resolve the tool list
    Then the list contains read_file, write_file, edit_file, run_bash, grep_search, glob_search, list_dir, and web_search

  Scenario: Default tool_style claude generates PascalCase tool names
    Given a ProviderConfig with no tool_style and no define_tools
    When I resolve the tool list
    Then the list contains Read, Write, Edit, Bash, Grep, Glob, LS, and WebSearch

  Scenario: map_tool_params renames parameter names
    Given a Rhai script whose map_tool_params renames the incoming "filepath" to "file_path" for file:read
    When the adapter maps tool params {"filepath": "a.txt"}
    Then the resulting InternalFileParams::Read has file_path equal to "a.txt"

  Scenario: map_tool_params returning unit uses default mapping
    Given a Rhai script whose map_tool_params returns () for all tools
    When the adapter maps tool params {"file_path":"a.txt"} for file:read
    Then the resulting InternalFileParams::Read has file_path equal to "a.txt" via default field-by-field deserialization

  Scenario: Partial tool list hides unlisted categories
    Given a Rhai script whose define_tools returns only a file:read tool and a bash tool
    When I resolve the tool list
    Then the list contains exactly two RhaiToolDef entries and no others

  Scenario: Rhai error in define_tools falls back to preset
    Given a Rhai script whose define_tools throws a runtime error and tool_style is "claude"
    When I resolve the tool list
    Then the resolved list matches the claude preset and a tracing::warn is logged with the script error

  Scenario: Unknown maps_to identifier is rejected with clear error
    Given a Rhai script whose define_tools returns a tool with maps_to "mystery:foo"
    When I resolve the tool list
    Then I receive an error whose message contains "mystery:foo" and lists valid identifiers like "file:read"

  Scenario: Adapter exposes Rhai-provided name and definition
    Given a RhaiToolDef with name "my_read", description "read a file", and parameters schema {type:"object", properties:{path:{type:"string"}}}
    When I build a RhaiToolFacadeAdapter for that tool
    Then adapter.name() returns "my_read" and adapter.parameters_schema() returns the JSON schema that matches the supplied parameters

  Scenario: Resolved tools are cached for system prompt introspection
    Given a resolved tool list computed for a provider
    When I inspect ProviderConfig.resolved_tools after resolution
    Then the field contains exactly the RhaiToolDef entries returned by resolution
