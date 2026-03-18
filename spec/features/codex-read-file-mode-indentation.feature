@BUG-109
Feature: Codex read_file facade missing mode and indentation params
  """
  Add InternalIndentationParams struct to traits.rs with all 5 optional fields
  Use extract_optional_string for mode, extract_optional_uint/extract_optional_bool for indentation sub-fields, matching existing param_extract patterns
  Schema must use additionalProperties: false on both the top-level and indentation nested object, matching existing Codex facade conventions
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The read_file schema must include a 'mode' property of type string with description mentioning 'slice' and 'indentation'
  #   2. The read_file schema must include an 'indentation' property of type object with nested properties: anchor_line (integer), max_levels (integer), include_siblings (boolean), include_header (boolean), max_lines (integer)
  #   3. InternalFileParams::Read must include mode (Option<String>) and indentation (Option<InternalIndentationParams>) fields
  #   4. CodexReadFileFacade::map_params must extract mode and indentation from the input JSON and populate the InternalFileParams::Read fields
  #   5. mode and indentation params are accepted in the schema for model compatibility but are currently passed through to InternalFileParams only — the ReadTool slice behavior is the fallback when mode is not supported
  #   6. Other facades (ZAI, Gemini) that construct InternalFileParams::Read must provide None for the new mode and indentation fields
  #   7. The FileToolFacadeWrapper in wrapper.rs must pass mode and indentation from InternalFileParams::Read through (even if ReadTool doesn't use them yet)
  #   8. The indentation object's inner properties should all be optional to match the Codex CLI spec
  #
  # EXAMPLES:
  #   1. Codex model sends read_file with mode='indentation' and indentation={anchor_line: 50, max_levels: 2} — facade maps all fields to InternalFileParams::Read
  #   2. Codex model sends read_file with mode='slice' — facade maps mode to Some('slice') and indentation to None
  #   3. Codex model sends read_file without mode or indentation — facade maps mode to None and indentation to None (backward compatible)
  #   4. Schema inspection shows indentation object with all 5 nested properties (anchor_line, max_levels, include_siblings, include_header, max_lines)
  #   5. ZAI facade constructs InternalFileParams::Read with mode=None and indentation=None (unchanged behavior)
  #   6. Codex model sends read_file with indentation={include_siblings: true, include_header: true, max_lines: 100} — all booleans and integers correctly extracted
  #
  # ========================================
  Background: User Story
    As a Codex model
    I want to use mode and indentation parameters in read_file
    So that navigate code by semantic blocks using indentation-aware reading

  Scenario: CodexReadFileFacade maps mode indentation to InternalFileParams::Read
    Given a CodexReadFileFacade instance
    When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {anchor_line: 50, max_levels: 2}
    Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
    And mode is Some("indentation")
    And indentation anchor_line is Some(50)
    And indentation max_levels is Some(2)
    And indentation include_siblings is None
    And indentation include_header is None
    And indentation max_lines is None

  Scenario: CodexReadFileFacade maps mode slice without indentation
    Given a CodexReadFileFacade instance
    When the Codex model calls read_file with file_path "/src/main.rs" and mode "slice"
    Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
    And mode is Some("slice")
    And indentation is None

  Scenario: CodexReadFileFacade backward compatible without mode or indentation
    Given a CodexReadFileFacade instance
    When the Codex model calls read_file with only file_path "/src/main.rs"
    Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
    And mode is None
    And indentation is None

  Scenario: Codex read_file schema includes mode and indentation properties
    Given a CodexReadFileFacade instance
    When the tool definition schema is inspected
    Then the schema has a "mode" property of type "string"
    And the schema has an "indentation" property of type "object"
    And the indentation object has an "anchor_line" property of type "integer"
    And the indentation object has a "max_levels" property of type "integer"
    And the indentation object has an "include_siblings" property of type "boolean"
    And the indentation object has an "include_header" property of type "boolean"
    And the indentation object has a "max_lines" property of type "integer"
    And the indentation object has additionalProperties false

  Scenario: Other facades provide None for mode and indentation fields
    Given a ZAIReadFileFacade instance
    When the ZAI model calls read_file with file_path "/src/main.rs"
    Then the facade maps to InternalFileParams::Read with mode None and indentation None

  Scenario: CodexReadFileFacade extracts all indentation boolean and integer fields
    Given a CodexReadFileFacade instance
    When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {include_siblings: true, include_header: true, max_lines: 100}
    Then indentation include_siblings is Some(true)
    And indentation include_header is Some(true)
    And indentation max_lines is Some(100)
    And indentation anchor_line is None
    And indentation max_levels is None
