@done
@BUG-110
Feature: Codex list_dir facade missing offset and limit pagination params
  """
  Facades map Codex list_dir params to InternalLsParams::List with offset, limit, depth. LsToolFacadeWrapper applies pagination. Follows same pattern as read_file mode/indentation.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. CodexListDirFacade schema must include offset, limit, and depth properties matching the Codex CLI spec
  #   2. CodexListDirFacade::map_params must extract and pass through dir_path, offset, limit, and depth to InternalLsParams
  #   3. InternalLsParams::List must include optional offset, limit, and depth fields
  #   4. All existing LsToolFacade implementations (ZAI, generic) must set offset, limit, and depth to None for backward compatibility
  #   5. LsToolFacadeWrapper must apply offset and limit pagination post-hoc on the LsTool output lines
  #
  # EXAMPLES:
  #   1. CodexListDirFacade maps list_dir with dir_path /src, offset 5, and limit 10 to InternalLsParams with all three params preserved
  #   2. CodexListDirFacade maps list_dir with only dir_path /src (no offset/limit/depth) to InternalLsParams with offset=None, limit=None, depth=None
  #   3. CodexListDirFacade schema has offset, limit, and depth properties; only dir_path is required
  #   4. ZAI list_dir facade produces InternalLsParams with offset=None, limit=None, depth=None for backward compat
  #   5. CodexListDirFacade maps depth 3 to InternalLsParams with depth=Some(3)
  #
  # ========================================
  Background: User Story
    As a Codex model
    I want to paginate directory listings with offset and limit parameters
    So that I can navigate large directories incrementally instead of receiving all entries at once

  Scenario: CodexListDirFacade maps offset and limit to InternalLsParams
    Given a CodexListDirFacade instance
    When the Codex model calls list_dir with dir_path "/src" offset 5 and limit 10
    Then the facade maps to InternalLsParams::List with path "/src" offset 5 and limit 10
    Then depth is None

  Scenario: CodexListDirFacade backward compatible without optional params
    Given a CodexListDirFacade instance
    When the Codex model calls list_dir with only dir_path "/src"
    Then the facade maps to InternalLsParams::List with offset None limit None and depth None

  Scenario: CodexListDirFacade schema includes offset limit and depth properties
    Given a CodexListDirFacade instance
    When the tool definition schema is inspected
    Then the schema has an "offset" property of type "integer"
    Then the schema has a "limit" property of type "integer"
    Then the schema has a "depth" property of type "integer"
    Then only "dir_path" is in the required array

  Scenario: CodexListDirFacade maps depth to InternalLsParams
    Given a CodexListDirFacade instance
    When the Codex model calls list_dir with dir_path "/src" and depth 3
    Then the facade maps to InternalLsParams::List with path "/src" and depth 3
    Then offset is None and limit is None

  Scenario: Other facades provide None for offset limit and depth
    Given a ZAIListDirFacade instance
    When the ZAI model calls list_dir with path "/src"
    Then the facade maps to InternalLsParams::List with offset None limit None and depth None
