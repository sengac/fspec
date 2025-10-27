@done
@version-management
@agent-integration
@cli
@high
@INIT-011
Feature: Automatic version check and update for slash command files
  """
  Implements fspec --sync-version <version> command that compares embedded version to package.json. Uses existing installAgentFiles() from init.ts to update files. Detects agent from spec/fspec-config.json. Generates agent-specific restart messages using similar pattern to getActivationMessage(). Exit code 0 for version match (silent), exit code 1 for mismatch (stops workflow).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Version check command must be the FIRST command in the command list (item 1 after "IMMEDIATELY - run these commands")
  #   2. Version embedded in fspec.md must match package.json version for check to pass
  #   3. On version mismatch, command must update BOTH slash command file AND spec doc file
  #   4. Exit code 1 on version mismatch (stops workflow), exit code 0 on match (continues workflow)
  #   5. Restart instructions must be agent-specific using agent detection from fspec-config.json
  #
  # EXAMPLES:
  #   1. User upgrades: npm install -g @sengac/fspec@0.7.0, runs /fspec in Claude Code, sees 'fspec --sync-version 0.6.0' execute, command detects mismatch, updates files, prints 'Updated to v0.7.0. Exit this conversation and start new one. Run /fspec again.', exits with code 1
  #   2. User with current version runs /fspec, 'fspec --sync-version 0.7.0' executes silently (versions match), exits 0, AI continues loading help commands normally
  #   3. Cursor user upgrades to v0.7.0, runs /fspec, version check detects mismatch, updates .cursor/commands/fspec.md and spec/CURSOR.md, prints 'Updated to v0.7.0. Restart Cursor and run /fspec again.'
  #   4. User deletes spec/fspec-config.json, version check can't detect agent, updates files anyway, shows generic message: 'Updated to v0.7.0. Restart your AI agent and run /fspec again.'
  #   5. fspec init command automatically embeds current package.json version into generated fspec.md: 'fspec --sync-version 0.7.0'
  #
  # QUESTIONS (ANSWERED):
  #   Q: When the user runs /fspec, should version check happen BEFORE loading any help docs, or only when they try to execute fspec commands?
  #   A: true
  #
  #   Q: Where should the version number be stored in fspec.md? At the very top as a comment, or in a specific section?
  #   A: true
  #
  #   Q: Which files need version tracking and auto-replacement? Just .claude/commands/fspec.md, or also spec/CLAUDE.md, or both?
  #   A: true
  #
  #   Q: Should the auto-update be silent, or should it notify the user that files were updated?
  #   A: true
  #
  #   Q: What should happen if the version check command fails (network issue, file permissions, etc.)? Continue anyway or block execution?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer who upgrades fspec to a new version
    I want to have slash command files automatically update to match the new version
    So that I don't need to manually reinstall or update fspec configuration files

  Scenario: Version mismatch detected on upgrade (Claude Code)
    Given I have fspec v0.5.0 installed with .claude/commands/fspec.md containing "fspec --sync-version 0.5.0"
    And spec/fspec-config.json contains agent "claude"
    When I upgrade to fspec v0.6.0 with "npm install -g @sengac/fspec@0.6.0"
    And I run /fspec in Claude Code
    And the AI agent executes "fspec --sync-version 0.5.0" as the first command
    Then the command should detect version mismatch (0.5.0 != 0.6.0)
    And it should update .claude/commands/fspec.md with new content including "fspec --sync-version 0.6.0"
    And it should update spec/CLAUDE.md with latest documentation
    And it should print "⚠️  fspec files updated from v0.5.0 to v0.6.0"
    And it should print Claude-specific restart message wrapped in <system-reminder> tags
    And the restart message should say "Exit this conversation and start a new one. Run /fspec again."
    And it should exit with code 1 (stopping workflow)
    And the AI agent should stop loading further help commands

  Scenario: Version match - no update needed
    Given I have fspec v0.6.0 installed with .claude/commands/fspec.md containing "fspec --sync-version 0.6.0"
    When I run /fspec in Claude Code
    And the AI agent executes "fspec --sync-version 0.6.0" as the first command
    Then the command should detect version match (0.6.0 == 0.6.0)
    And it should not update any files
    And it should not print any output (silent)
    And it should exit with code 0 (continuing workflow)
    And the AI agent should proceed to load help commands normally

  Scenario: Version mismatch detected for Cursor agent (no system-reminders)
    Given I have fspec v0.5.0 installed with .cursor/commands/fspec.md containing "fspec --sync-version 0.5.0"
    And spec/fspec-config.json contains agent "cursor"
    And Cursor agent does not support system-reminders (supportsSystemReminders: false)
    When I upgrade to fspec v0.6.0
    And I run /fspec in Cursor
    And the AI agent executes "fspec --sync-version 0.5.0" as the first command
    Then the command should detect version mismatch
    And it should update .cursor/commands/fspec.md with "fspec --sync-version 0.6.0"
    And it should update spec/CURSOR.md with latest documentation
    And it should print "⚠️  fspec files updated from v0.5.0 to v0.6.0" as plain text (no system-reminder tags)
    And it should print Cursor-specific restart instructions "Restart Cursor and run /fspec again"
    And it should exit with code 1

  Scenario: Agent detection fallback when config missing
    Given I have fspec v0.5.0 installed with .claude/commands/fspec.md containing "fspec --sync-version 0.5.0"
    And spec/fspec-config.json is missing or corrupted
    When I upgrade to fspec v0.6.0
    And I run /fspec
    And the AI agent executes "fspec --sync-version 0.5.0" as the first command
    Then the command should detect version mismatch
    And it should attempt to detect agent from filesystem (.claude/ directory exists)
    And it should update both files (.claude/commands/fspec.md and spec/CLAUDE.md)
    And it should detect Claude supports system-reminders and wrap message in tags
    And it should print generic restart instructions "Restart your AI agent and run /fspec again"
    And it should exit with code 1

  Scenario: fspec init embeds current version automatically
    Given I have fspec v0.6.0 installed
    When I run "fspec init --agent=claude"
    Then it should read version from package.json (0.6.0)
    And it should generate .claude/commands/fspec.md with "fspec --sync-version 0.6.0" as first command in the list
    And the version check command should appear as item 1 in the command list (after "IMMEDIATELY - run these commands" section)
    And it should create spec/CLAUDE.md with latest documentation
    And it should create spec/fspec-config.json with agent "claude"

  Scenario: Emit tool config checks when versions match
    Given embedded version matches current package.json version
    And spec/fspec-config.json does not have tools configured
    When AI runs 'fspec --sync-version 0.6.0'
    Then sync-version should call checkTestCommand function
    And sync-version should call checkQualityCommands function
    And system-reminders should be emitted: 'NO TEST COMMAND CONFIGURED'
    And this helps onboard new AI agents to configure tools
