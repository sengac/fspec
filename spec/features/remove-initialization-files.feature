@done
@feature-management
@initialization
@cli
@medium
@INIT-009
Feature: Remove initialization files
  """
  Uses interactive prompt component similar to AgentSelector in fspec init. Leverages Ink React for terminal UI. Detects agents using detectAgents() from agentDetection.ts. Reads spec/fspec-config.json to identify installed agent. Uses fs/promises.rm() with force:true to silently skip missing files.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Use an interactive prompt (similar to agent selector in fspec init) to ask user if they want to keep or remove spec/fspec-config.json
  #   2. No additional confirmation needed - the interactive prompt about keeping/removing config serves as the confirmation step. Delete immediately after user makes their choice.
  #   3. Auto-detect installed agents using spec/fspec-config.json or file detection, and only remove files for detected agents (not all known agents)
  #   4. Silently skip missing files and continue removing what exists. No error if files are already deleted.
  #
  # EXAMPLES:
  #   1. User runs 'fspec remove-init-files', interactive prompt asks 'Keep spec/fspec-config.json?', user selects 'No', all files removed including config
  #   2. User runs 'fspec remove-init-files', interactive prompt asks 'Keep spec/fspec-config.json?', user selects 'Yes', only agent files removed (spec/CLAUDE.md, .claude/commands/fspec.md), config preserved
  #   3. User runs 'fspec remove-init-files', spec/fspec-config.json contains 'claude' agent, only Claude files removed (not Cursor or other agents)
  #   4. User runs 'fspec remove-init-files', .claude/commands/fspec.md already missing, command silently skips and removes other files without error
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should fspec remove-init-files remove spec/fspec-config.json, or only agent-specific files (spec/AGENT.md and slash command files)?
  #   A: true
  #
  #   Q: Should fspec remove-init-files prompt for confirmation before deleting, or support a --force flag?
  #   A: true
  #
  #   Q: Should fspec remove-init-files auto-detect installed agents (from spec/fspec-config.json or detection) and only remove those files, or remove all known agent files regardless?
  #   A: true
  #
  #   Q: If some init files don't exist (e.g., .claude/commands/fspec.md is missing), should the command fail with an error, or silently skip and report what was removed?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer who wants to remove fspec files or switch agents
    I want to remove all fspec init files
    So that my project is clean when I switch agents or remove fspec entirely

  Scenario: Remove all files including config
    Given I have fspec initialized with Claude agent
    Given spec/fspec-config.json exists with agent 'claude'
    When I run 'fspec remove-init-files'
    When the interactive prompt asks 'Keep spec/fspec-config.json?'
    When I select 'No'
    Then spec/CLAUDE.md should be removed
    Then .claude/commands/fspec.md should be removed
    Then spec/fspec-config.json should be removed

  Scenario: Keep config and remove only agent files
    Given I have fspec initialized with Claude agent
    Given spec/fspec-config.json exists with agent 'claude'
    When I run 'fspec remove-init-files'
    When the interactive prompt asks 'Keep spec/fspec-config.json?'
    When I select 'Yes'
    Then spec/CLAUDE.md should be removed
    Then .claude/commands/fspec.md should be removed
    Then spec/fspec-config.json should still exist

  Scenario: Remove only detected agent files
    Given I have fspec initialized with Claude agent only
    Given spec/fspec-config.json contains agent 'claude'
    Given Cursor files do not exist
    When I run 'fspec remove-init-files'
    When I select 'No' to keep config prompt
    Then only Claude files should be removed (spec/CLAUDE.md, .claude/commands/fspec.md)
    Then Cursor files should not be attempted for removal

  Scenario: Handle missing files gracefully
    Given I have spec/fspec-config.json with agent 'claude'
    Given spec/CLAUDE.md exists
    Given .claude/commands/fspec.md is already deleted
    When I run 'fspec remove-init-files'
    When I select 'No' to keep config prompt
    Then the command should succeed without errors
    Then spec/CLAUDE.md should be removed
    Then the missing .claude/commands/fspec.md should be silently skipped
