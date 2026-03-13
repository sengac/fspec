@BUG-105
Feature: Codex tool-calling facade omits apply_patch and falls back to shell patching

  """
  The apply_patch tool is implemented as a standalone rig::tool::Tool struct (not through facade traits) since it has no equivalent in other providers. It takes a single 'patch' string parameter, parses the Codex freeform format, then delegates to internal file operations (create, edit, delete). It lives in codelet/tools/src/apply_patch.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The Codex integration should expose a first-class file editing path and must not fall back to executing apply_patch as a shell command.
  #   2. If Codex does not expose a dedicated apply_patch tool, it should use the existing structured editing tools such as Edit and Write instead of shell fallback.
  #   3. Editing behavior should remain consistent across provider integrations so patch-style file changes do not silently degrade only in Codex-backed sessions.
  #   4. The Codex facade must expose an apply_patch tool that accepts freeform patch text and maps it to internal Write/Edit/Delete operations
  #   5. The apply_patch tool must parse the Codex freeform patch format: '*** Begin Patch' / '*** Add File:' / '*** Update File:' / '*** Delete File:' / '*** End Patch'
  #   6. The apply_patch tool must be registered in CodexProvider::create_rig_agent alongside the existing facade tools
  #   7. WriteTool and EditTool should be removed from the Codex agent once apply_patch is available, to prevent the model from choosing between competing edit interfaces
  #   8. apply_patch parser must handle multi-file patches (multiple Add/Update/Delete blocks in a single patch)
  #   9. apply_patch Update File operations must use context lines for accurate matching, similar to unified diff '@@ context' markers
  #
  # EXAMPLES:
  #   1. Agent calls apply_patch with a single 'Add File' block → file is created with the specified content
  #   2. Agent calls apply_patch with an 'Update File' block containing context, removals, and additions → correct lines are replaced in the file
  #   3. Agent calls apply_patch with a 'Delete File' block → file is removed from disk
  #   4. Agent calls apply_patch with multiple file operations in one patch → all operations are applied atomically
  #   5. Agent sends malformed patch text missing '*** Begin Patch' → apply_patch returns a clear error
  #   6. Agent calls apply_patch with Update File but context lines don't match → apply_patch returns an error describing the mismatch
  #
  # ========================================

  Background: User Story
    As a developer using Codex provider
    I want to edit files through the Codex-backed agent
    So that file changes succeed via first-class tool calls instead of failing with 'apply_patch: command not found'

  Scenario: Add a new file via apply_patch
    Given a Codex session with the apply_patch tool registered
    When the agent calls apply_patch with an Add File block for "/tmp/test/new_file.rs"
    Then the file "/tmp/test/new_file.rs" is created with the specified content
    And the tool returns a success message listing the created file

  Scenario: Update an existing file via apply_patch
    Given a Codex session with the apply_patch tool registered
    And a file "/tmp/test/existing.rs" exists with known content
    When the agent calls apply_patch with an Update File block containing context lines, removals, and additions
    Then the matching lines in "/tmp/test/existing.rs" are replaced
    And unchanged context lines remain intact
    And the tool returns a success message listing the updated file

  Scenario: Delete a file via apply_patch
    Given a Codex session with the apply_patch tool registered
    And a file "/tmp/test/to_delete.rs" exists
    When the agent calls apply_patch with a Delete File block for "/tmp/test/to_delete.rs"
    Then the file "/tmp/test/to_delete.rs" no longer exists on disk
    And the tool returns a success message listing the deleted file

  Scenario: Multi-file patch with mixed operations
    Given a Codex session with the apply_patch tool registered
    And a file "/tmp/test/update_me.rs" exists with known content
    And a file "/tmp/test/delete_me.rs" exists
    When the agent calls apply_patch with Add, Update, and Delete blocks in a single patch
    Then the new file is created
    And the existing file is updated
    And the deleted file is removed
    And the tool returns a success message listing all affected files

  Scenario: Malformed patch missing Begin Patch marker
    Given a Codex session with the apply_patch tool registered
    When the agent calls apply_patch with text that does not start with "*** Begin Patch"
    Then the tool returns an error mentioning the missing patch marker

  Scenario: Update File with non-matching context lines
    Given a Codex session with the apply_patch tool registered
    And a file "/tmp/test/mismatch.rs" exists with known content
    When the agent calls apply_patch with an Update File block whose context lines do not match the file
    Then the tool returns an error describing the context mismatch
    And the file "/tmp/test/mismatch.rs" is not modified

  Scenario: apply_patch tool is registered in Codex agent
    Given a CodexProvider configured with a valid model
    When create_rig_agent is called
    Then the agent has an "apply_patch" tool in its tool set
    And the agent does not have "Write" or "Edit" tools registered
