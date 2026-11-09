@done
@BUG-607
@version-management
@agent-integration
@cli
@high
@INIT-011
@BUG-285
Feature: Automatic version check and update for slash command files
  """
  Implements the top-level `fspec --sync-version <version>` flag of the Rust CLI (a hidden clap field on the root `Cli`, because the generated /fspec slash command runs it with no subcommand). The CLI bridge rust/fspec/src/sync_version.rs only marshals JSON and prints; the single source of truth is codelet_fspec_core::commands::sync_version::run, which compares the embedded version with the running binary version (CARGO_PKG_VERSION, the same constant `fspec init` and `fspec bootstrap` embed). On a mismatch it reinstalls the agent's slash command and spec doc file through init's existing install_agent_files (no duplicated scaffolding) and returns an agent-specific restart message; the agent is read from spec/fspec-config.json with a filesystem-detection fallback (generic restart message). On a match it reuses the configure-tools checks (check_test_command / check_quality_commands) and is silent when a test command is configured. Exit code 0 for a configured match, 1 for a mismatch or missing test command (stops the workflow).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Version check command must be the FIRST command in the command list (item 1 after "IMMEDIATELY - run these commands")
  #   2. Version embedded in fspec.md must match the running fspec binary version for check to pass
  #   3. On version mismatch, command must update BOTH slash command file AND spec doc file
  #   4. Exit code 1 on version mismatch (stops workflow), exit code 0 on match (continues workflow)
  #   5. Restart instructions must be agent-specific using agent detection from fspec-config.json
  #   6. fspec init and fspec bootstrap embed the running binary version, never a hard-coded version (BUG-285)
  #
  # EXAMPLES:
  #   1. User upgrades the fspec binary, runs /fspec in Claude Code, the stale 'fspec --sync-version 0.0.1' executes, the command detects the mismatch, rewrites .claude/commands/fspec.md and spec/AGENTS.md, prints 'Exit this conversation and start a new one. Run /fspec again.' in a <system-reminder>, exits with code 1
  #   2. User with the current version and a configured test command runs /fspec, 'fspec --sync-version <current>' executes silently, exits 0, AI continues with fspec bootstrap
  #   3. Cursor user upgrades, runs /fspec, version check detects mismatch, updates .cursor/commands/fspec.md and spec/CURSOR.md, prints 'Restart Cursor and run /fspec again' as plain text
  #   4. User deletes spec/fspec-config.json, the agent is detected from the .claude/ directory, files are updated anyway, generic message: 'Restart your AI agent and run /fspec again'
  #   5. fspec init embeds the running binary version into the generated fspec.md: 'fspec --sync-version <current>'
  #   6. BUG-285: fspec 0.10.9 failed with "error: unknown option '--sync-version'" because the Rust CLI never registered the flag
  #
  # QUESTIONS (ANSWERED):
  #   Q: When the user runs /fspec, should version check happen BEFORE loading any help docs, or only when they try to execute fspec commands?
  #   A: true
  #
  #   Q: Where should the version number be stored in fspec.md? At the very top as a comment, or in a specific section?
  #   A: true
  #
  #   Q: Which files need version tracking and auto-replacement? Just .claude/commands/fspec.md, or also spec/AGENTS.md, or both?
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
    Given a project with .claude/commands/fspec.md containing "fspec --sync-version 0.0.1"
    And spec/fspec-config.json contains agent "claude"
    And the installed fspec binary reports a newer version than 0.0.1
    When the AI agent runs "fspec --sync-version 0.0.1" as the first command of /fspec
    Then it should update .claude/commands/fspec.md with new content including "fspec --sync-version" and the current version
    And it should update spec/AGENTS.md with latest documentation
    And it should print "⚠️  fspec files updated from v0.0.1 to v" followed by the current version
    And it should print Claude-specific restart message wrapped in <system-reminder> tags
    And the restart message should say "Exit this conversation and start a new one. Run /fspec again."
    And it should exit with code 1 (stopping workflow)

  Scenario: Version match - no update needed
    Given a project initialised with "fspec init --agent=claude" by the installed fspec binary
    And spec/fspec-config.json has a test command configured
    When the AI agent runs "fspec --sync-version" with the current version
    Then it should not update any files
    And it should not print any output (silent)
    And it should exit with code 0 (continuing workflow)

  Scenario: Version mismatch detected for Cursor agent (no system-reminders)
    Given a project with .cursor/commands/fspec.md containing "fspec --sync-version 0.0.1"
    And spec/fspec-config.json contains agent "cursor"
    When the AI agent runs "fspec --sync-version 0.0.1"
    Then it should update .cursor/commands/fspec.md with "fspec --sync-version" and the current version
    And it should update spec/CURSOR.md with latest documentation
    And it should print "⚠️  fspec files updated from v0.0.1 to v" as plain text (no system-reminder tags)
    And it should print Cursor-specific restart instructions "Restart Cursor and run /fspec again"
    And it should exit with code 1

  Scenario: Agent detection fallback when config missing
    Given a project with .claude/commands/fspec.md containing "fspec --sync-version 0.0.1"
    And spec/fspec-config.json is missing
    When the AI agent runs "fspec --sync-version 0.0.1"
    Then it should detect the agent from the filesystem (.claude/ directory exists)
    And it should update both files (.claude/commands/fspec.md and spec/AGENTS.md)
    And it should wrap the message in <system-reminder> tags because Claude supports them
    And it should print generic restart instructions "Restart your AI agent and run /fspec again"
    And it should exit with code 1

  Scenario: fspec init embeds current version automatically
    Given an empty project directory
    When I run "fspec init --agent=claude"
    Then it should generate .claude/commands/fspec.md with "fspec --sync-version" followed by the version reported by "fspec --version"
    And the version check command should appear right after the "IMMEDIATELY - run this command" line
    And it should create spec/AGENTS.md with latest documentation
    And it should create spec/fspec-config.json with agent "claude"
    And "fspec bootstrap" should print "fspec --sync-version" followed by the same version instead of a hard-coded one

  Scenario: Emit tool config checks when versions match
    Given a project initialised with "fspec init --agent=claude" by the installed fspec binary
    And spec/fspec-config.json does not have tools configured
    When the AI agent runs "fspec --sync-version" with the current version
    Then sync-version should emit the configure-tools test command check
    And sync-version should emit the configure-tools quality commands check
    And the output should contain a system-reminder with "NO TEST COMMAND CONFIGURED"
    And it should exit with code 1 so the AI agent runs "fspec configure-tools" first

  Scenario: A stale doc is reinstalled although the versions match
    Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    And spec/AGENTS.md holds instructions this fspec no longer writes
    When the AI agent runs "fspec --sync-version" with the current version
    Then spec/AGENTS.md is rewritten to exactly what "fspec init --agent=claude" writes
    And the output names spec/AGENTS.md as updated and tells the agent to restart, in a system-reminder
    And it should exit with code 1

  Scenario: An edited slash command is reinstalled although the versions match
    Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    And .claude/commands/fspec.md was edited by hand
    When the AI agent runs "fspec --sync-version" with the current version
    Then .claude/commands/fspec.md is rewritten to exactly what "fspec init --agent=claude" writes
    And the output names .claude/commands/fspec.md as updated and tells the agent to restart
    And it should exit with code 1

  Scenario: A deleted doc is written again although the versions match
    Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    And spec/AGENTS.md was deleted
    When the AI agent runs "fspec --sync-version" with the current version
    Then spec/AGENTS.md exists again with exactly what "fspec init --agent=claude" writes
    And it should exit with code 1

  Scenario: Files that match what this fspec writes are left alone
    Given a project initialised with "fspec init --agent=claude" by the installed fspec binary, with a test command configured
    When the AI agent runs "fspec --sync-version" with the current version twice
    Then neither run writes a file
    And both runs print nothing and exit with code 0

  Scenario: Emit tool config checks when spec/fspec-config.json does not exist
    Given a project initialised with "fspec init --agent=claude" by the installed fspec binary
    And spec/fspec-config.json was then deleted
    When the AI agent runs "fspec --sync-version" with the current version
    Then the output should contain "NO TEST COMMAND CONFIGURED"
    And the output should tell the AI agent to run "fspec configure-tools --test-command <cmd>"
    And it should exit with code 1 so the AI agent runs "fspec configure-tools" first
