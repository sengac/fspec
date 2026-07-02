@done
@tui
@rust
@ts-parity
@agent-view
@RPC-388
Feature: Tool call argument header
  """
  Port mirrors src/tui/utils/chunkProcessor.ts:130-205 extractToolArgsDisplay; replaces the single-key selection in tool_args.rs with the three-branch algorithm + value formatter
  Use char-boundary-safe slicing (chars().take(100)) for the 100-char cap; preserve JSON object insertion order (serde_json Map already used by existing tests); update existing tool_args tests to the new parity outputs
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. For Edit/Write-family tools (edit, replace, write, write_file, matched case-insensitively) the header shows only the file_path value, or an empty string when file_path is absent
  #   2. When the input has a command (or else action_type) key, the header shows that value first; if other params exist they follow as ', { key: value, ... }' for ALL remaining params
  #   3. Otherwise the header shows ALL params as '{ key: value, ... }', or an empty string when there are no params
  #   4. Each value is formatted: string values are single-quoted, null renders bare, other values use compact JSON; every value is capped at 100 characters with a literal '...' suffix when longer
  #   5. When the input JSON cannot be parsed, the header shows the raw input string verbatim; parameter order follows the input JSON object's insertion order
  #
  # EXAMPLES:
  #   1. Edit with {file_path:/a.rs, old_string:x, new_string:y} renders header args '/a.rs'
  #   2. Write with {content:...} and no file_path renders empty header args ''
  #   3. Bash with {command:ls -la, timeout:5000} renders header args 'ls -la, { timeout: 5000 }'
  #   4. WebSearch with {action_type:search, query:hi} renders header args 'search, { query: \'hi\' }'
  #   5. Grep with {pattern:foo, glob:*.rs} (no command/action_type) renders header args '{ pattern: \'foo\', glob: \'*.rs\' }'
  #   6. A param string of 120 characters is capped to its first 100 characters followed by '...' in the header
  #   7. Invalid JSON input 'not-json' renders header args 'not-json' verbatim
  #
  # ========================================
  Background: User Story
    As a developer watching the agent TUI
    I want to see a tool call's arguments rendered the same way the TypeScript reference renders them — all parameters, with long values capped at 100 characters
    So that the header is informative yet bounded, matching the reference UI

  Scenario: Edit-family tool shows only the file_path value
    Given a tool call for "Edit" with input '{"file_path":"/a.rs","old_string":"x","new_string":"y"}'
    When the tool-call argument header is extracted
    Then the header args are "/a.rs"

  Scenario: Write-family tool with no file_path shows an empty string
    Given a tool call for "Write" with input '{"content":"..."}'
    When the tool-call argument header is extracted
    Then the header args are ""

  Scenario: Command tool shows the command first then remaining params
    Given a tool call for "Bash" with input '{"command":"ls -la","timeout":5000}'
    When the tool-call argument header is extracted
    Then the header args are "ls -la, { timeout: 5000 }"

  Scenario: action_type tool shows the action first then remaining params
    Given a tool call for "WebSearch" with input '{"action_type":"search","query":"hi"}'
    When the tool-call argument header is extracted
    Then the header args are "search, { query: 'hi' }"

  Scenario: Tool with no command or action_type shows all params as an object
    Given a tool call for "Grep" with input '{"pattern":"foo","glob":"*.rs"}'
    When the tool-call argument header is extracted
    Then the header args are "{ pattern: 'foo', glob: '*.rs' }"

  Scenario: A value longer than 100 characters is capped with an ellipsis
    Given a tool call for "Grep" with a single param whose string value is 120 characters long
    When the tool-call argument header is extracted
    Then the value is the first 100 characters followed by "..."

  Scenario: Invalid JSON input is shown verbatim
    Given a tool call for "Bash" with input "not-json"
    When the tool-call argument header is extracted
    Then the header args are "not-json"
