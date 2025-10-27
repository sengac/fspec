@done
@agent-integration
@template-generation
@initialization
@cli
@high
@INIT-012
Feature: Dynamic bootstrap command for slash command template
  """

  - Uses existing slashCommandTemplate.ts section functions (getHeaderSection, getAcddConceptSection, etc.)
  - Bootstrap command internally calls all 13 section-generating functions and combines output
  - Applies string replacement for <test-command> and <quality-check-commands> placeholders
  - Template migration handled by existing --sync-version mechanism (no special logic needed)
  - No options/flags allowed on bootstrap command (always outputs everything)
  - Single command execution replaces 8 separate Bash invocations (faster, lower token usage)

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The slash command template must contain ONLY two commands: 'fspec --sync-version X.X.X' and 'fspec bootstrap'
  #   2. All documentation content currently after 'fspec --sync-version' must be moved to dynamically generated output of 'fspec bootstrap'
  #   3. 'fspec bootstrap' must execute the same help commands that are currently hardcoded (fspec --help, fspec help specs, etc.)
  #   4. Keep output identical to current template. Use existing template generation functions (with string replacement for <test-command> and <quality-check-commands> placeholders)
  #   5. Bootstrap is ABSOLUTELY MANDATORY. If skipped, fspec commands MUST FAIL with clear error message instructing user to run 'fspec bootstrap' first. No fspec workflow commands should work until bootstrap completes successfully.
  #   6. NO customization allowed. 'fspec bootstrap' MUST output everything with NO options to skip sections. No --skip-help, no --minimal, no --skip-sections flags. Bootstrap outputs complete documentation ALWAYS.
  #
  # EXAMPLES:
  #   1. User runs 'fspec board' without running 'fspec bootstrap' first, command exits with error: 'ERROR: fspec bootstrap must be run first. Run: fspec bootstrap'
  #   2. User runs 'fspec --sync-version 0.6.0' then 'fspec bootstrap' successfully, then runs 'fspec board' - command executes normally
  #   3. AI runs 'fspec bootstrap', command internally calls getHeaderSection(), getAcddConceptSection(), etc., combines output, applies string replacement, prints to stdout
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should 'fspec bootstrap' output be identical to what's currently in the template after fspec --sync-version, or can we improve/reorganize it?
  #   A: true
  #
  #   Q: Should 'fspec bootstrap' be mandatory (fail if not run), or optional (gracefully handle if skipped)?
  #   A: true
  #
  #   Q: Should the bootstrap command check if documentation is already loaded (avoid duplicate loading in same session)?
  #   A: true
  #
  #   Q: What should happen to existing .claude/commands/fspec.md files when users upgrade? Auto-migrate to new format, or require manual deletion?
  #   A: true
  #
  #   Q: Should 'fspec bootstrap' accept arguments (like --skip-help or --minimal) to customize what gets loaded?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No need to track or prevent duplicate runs. If user runs 'fspec bootstrap' multiple times, it just outputs documentation again. No harm done.
  #   2. No special migration logic needed. Existing --sync-version mechanism handles it automatically. When version mismatch detected, installAgentFiles() regenerates template with new 2-command format. Clean, simple, follows existing architectural pattern.
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec
    I want to load documentation dynamically via bootstrap command
    So that slash command template stays lightweight and content is always fresh

  Scenario: Bootstrap command outputs complete documentation
    given I have fspec installed
    when I run 'fspec bootstrap'
    then the command should output complete documentation by internally calling all section functions (getHeaderSection, getAcddConceptSection, etc.)
    And the output should include string replacement for <test-command> and <quality-check-commands> placeholders
    And the output should be identical to what currently appears after 'fspec --sync-version' in templates

  Scenario: Template contains only two commands
    given I run 'fspec init --agent=claude'
    when the template file .claude/commands/fspec.md is generated
    then the command list should contain ONLY 2 commands: 'fspec --sync-version X.X.X' and 'fspec bootstrap'
    And the template should NOT contain 'fspec --help', 'fspec help specs', etc.
    And the template should instruct AI to run both commands before continuing

  Scenario: Template migration on version upgrade
    given I have fspec v0.6.0 installed with .claude/commands/fspec.md containing 8 commands
    when I upgrade to fspec v0.7.0 and run '/fspec' which executes 'fspec --sync-version 0.6.0'
    then the sync-version command should detect version mismatch (0.6.0 != 0.7.0)
    And the template contains 'fspec --sync-version 0.6.0'
    And it should call installAgentFiles() to regenerate template
    And the new template should contain only 2 commands: 'fspec --sync-version 0.7.0' and 'fspec bootstrap'
    And it should exit with code 1 and tell user to restart

  Scenario: Bootstrap internally executes all help commands
    given I have fspec installed
    when I run 'fspec bootstrap'
    then the command should internally execute 'fspec --help'
    And the command should internally execute 'fspec help specs'
    And the command should internally execute 'fspec help work'
    And the command should internally execute 'fspec help discovery'
    And the command should internally execute 'fspec help metrics'
    And the command should internally execute 'fspec help setup'
    And the command should internally execute 'fspec help hooks'
    And the combined output should be printed to stdout

  Scenario: Bootstrap command has no customization options
    given I have fspec installed with bootstrap command
    when I run 'fspec bootstrap --help'
    then the help output should show NO options or flags
    And there should be no --skip-help, --minimal, or --skip-sections flags
    And running 'fspec bootstrap' should always output complete documentation

  Scenario: Help content getter functions return complete documentation not stub text
    Given I have the display*Help() functions in src/help.ts with full command documentation
    When I call getSpecsHelpContent(), getWorkHelpContent(), getDiscoveryHelpContent(), getMetricsHelpContent(), getSetupHelpContent(), getHooksHelpContent()
    Then each function should return the EXACT SAME content as its corresponding display function but as plain text without chalk formatting
    And getSpecsHelpContent() should return full 333 lines of content from displaySpecsHelp() showing ALL commands with examples and options
    And getWorkHelpContent() should return full 308 lines of content from displayWorkHelp() showing ALL commands including checkpoints, dependencies, analysis commands
    And all 6 getter functions should NOT return stub text like 'Commands for creating and managing... and more' which provides no value to AI agents
    And the getter functions MUST NOT duplicate code - they should reuse the display functions by intercepting console output

  Scenario: Bootstrap displays explainer about help command outputs
    Given I run 'fspec bootstrap'
    When the bootstrap command outputs its content
    Then the output should contain an explainer section before the help command outputs
    And the explainer should explain what the content is (output from fspec help commands)
    And the explainer should explain WHY this information matters (complete command reference)
    And the explainer should explain HOW to access sections again (run individual help commands, NOT bootstrap)
