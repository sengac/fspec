@done
@provider-abstraction
@codelet
@facade-pattern
@tools
@BUG-111
Feature: Codex grep_files facade does not map include or limit params
  """
  The underlying GrepTool::execute() already supports glob filtering via a 'glob' param (grep.rs line 221). The fix is a plumbing issue: InternalSearchParams::Grep needs include/limit fields, the facade needs to extract them, and the wrapper needs to pass 'include' as 'glob' to GrepTool.execute().
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CodexGrepFilesFacade.map_params() must extract 'include' from input and pass it through InternalSearchParams::Grep to GrepTool
  #   2. CodexGrepFilesFacade.map_params() must extract 'limit' from input and pass it through InternalSearchParams::Grep to GrepTool
  #   3. InternalSearchParams::Grep must include optional 'include' and 'limit' fields so facades can pass them through
  #   4. SearchToolFacadeWrapper must pass 'include' as 'glob' to GrepTool.execute() args (because GrepTool already supports glob filtering via the 'glob' key)
  #   5. SearchToolFacadeWrapper must apply 'limit' to cap the number of results returned from GrepTool
  #   6. All other facades constructing InternalSearchParams::Grep (ZAI, Gemini, search.rs) must be updated for the new fields using None defaults
  #   7. GrepArgs struct must gain an optional 'glob' field so the call() method can pass it through to execute()
  #
  # EXAMPLES:
  #   1. Model sends {"pattern": "TODO", "include": "*.rs", "path": "/src"} → facade extracts include="*.rs" → InternalSearchParams::Grep has include=Some("*.rs") → wrapper passes glob="*.rs" to GrepTool → only .rs files searched
  #   2. Model sends {"pattern": ".", "include": "*.rs", "limit": 10} → only 10 file paths returned even if hundreds match
  #   3. Model sends {"pattern": "TODO"} with no include/limit → all files searched, all results returned (backward compatible)
  #   4. ZAI and Gemini grep facades continue to work with include=None and limit=None (non-breaking change)
  #
  # ========================================
  Background: User Story
    As a Codex model
    I want to filter grep_files results by file type using the include param and cap results with limit
    So that I can do glob-like file matching (the only way since glob tool was removed in BUG-107)

  Scenario: CodexGrepFilesFacade maps include param to InternalSearchParams::Grep
    Given a CodexGrepFilesFacade instance
    When the Codex model calls grep_files with pattern "TODO", include "*.rs", and path "/src"
    Then the facade maps to InternalSearchParams::Grep with pattern "TODO", path "/src", and include "*.rs"

  Scenario: CodexGrepFilesFacade maps limit param to InternalSearchParams::Grep
    Given a CodexGrepFilesFacade instance
    When the Codex model calls grep_files with pattern "." and limit 10
    Then the facade maps to InternalSearchParams::Grep with pattern "." and limit 10

  Scenario: CodexGrepFilesFacade maps both include and limit params
    Given a CodexGrepFilesFacade instance
    When the Codex model calls grep_files with pattern ".", include "*.rs", and limit 10
    Then the facade maps to InternalSearchParams::Grep with include "*.rs" and limit 10

  Scenario: CodexGrepFilesFacade remains backward compatible without include or limit
    Given a CodexGrepFilesFacade instance
    When the Codex model calls grep_files with only pattern "TODO"
    Then the facade maps to InternalSearchParams::Grep with include None and limit None

  Scenario: InternalSearchParams::Grep includes optional include and limit fields
    Given the InternalSearchParams::Grep enum variant
    Then it has an optional "include" field of type Option<String>
    And it has an optional "limit" field of type Option<usize>

  Scenario: SearchToolFacadeWrapper passes include as glob to GrepTool
    Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
    When the wrapper receives InternalSearchParams::Grep with include "*.rs"
    Then the wrapper passes "glob" = "*.rs" in the GrepTool execute args

  Scenario: SearchToolFacadeWrapper applies limit to cap grep results
    Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
    When the wrapper receives InternalSearchParams::Grep with limit 10
    Then the wrapper caps the grep output to at most 10 result lines

  Scenario: GrepArgs struct supports optional glob field
    Given a GrepArgs with pattern "TODO" and glob "*.rs"
    When the GrepTool call() method executes
    Then the glob filter is passed through to the execute() method

  Scenario: ZAI grep facade constructs InternalSearchParams::Grep with None defaults
    Given a ZAIGrepFilesFacade instance
    When the ZAI model calls grep_files with pattern "TODO" and path "src"
    Then the facade maps to InternalSearchParams::Grep with include None and limit None

  Scenario: Gemini grep facade constructs InternalSearchParams::Grep with None defaults
    Given a GeminiSearchFileContentFacade instance
    When Gemini sends parameters {pattern: 'TODO', dir_path: 'src'} to tool 'search_file_content'
    Then the facade maps to InternalSearchParams::Grep with include None and limit None
