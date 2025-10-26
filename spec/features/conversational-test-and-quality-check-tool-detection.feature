@done
@critical
@cli
@configuration
@setup
@cross-platform
@CONFIG-002
Feature: Conversational Test and Quality Check Tool Detection
  """

  Key architectural decisions:
  - fspec does ZERO framework detection - purely conversational via system-reminders
  - AI agent performs all detection using Read/Glob/Grep tools
  - Config stored in spec/fspec-config.json (extends existing agent config file)
  - Priority chain: env var (FSPEC_TEST_CMD/FSPEC_QUALITY_CMDS) → config file → system-reminder prompt

  Dependencies and integrations:
  - Uses existing config utilities (loadConfig/writeConfig from src/utils/config.ts)
  - Follows agentRuntimeConfig.ts pattern for config lookup
  - Integrates with formatAgentOutput() for agent-specific system-reminder formatting
  - Integrates into ACDD workflow state transitions (emits reminders during validating phase)

  Critical implementation requirements:
  - MUST emit system-reminders guiding AI to detect tools (NEVER run detection code)
  - MUST support command storage via --test-command and --quality-commands flags
  - MUST be framework-agnostic (works with ANY language/platform)
  - MUST include date-aware search queries when no tools detected
  - MUST update all documentation (CLAUDE.md, help files, README.md) to remove hardcoded npm/platform-specific references
  - Config schema: {tools: {test: {command: string}, qualityCheck: {commands: string[]}}}

  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. fspec MUST NOT execute semantic code analysis - ALL detection done via system-reminders guiding AI
  #   2. Tool detection MUST be conversational like fspec review and fspec reverse
  #   3. Configuration MUST use same lookup system as system-reminder style detection
  #   4. AI MUST detect existing tools OR guide setup if none exist using current best practices
  #   5. All hardcoded npm test and npm check references MUST be replaced with platform-agnostic commands
  #   6. Store in spec/fspec-config.json with a 'tools' section (extend existing file that already stores agent config)
  #   7. One-time setup via explicit 'fspec configure-tools' command (like fspec init). Commands emit system-reminder if config missing, telling AI to run configure-tools. Detection happens once, used many times.
  #   8. Yes - 'fspec configure-tools' supports both initial setup AND reconfiguration. Includes --reconfigure flag for re-detection, manual override flags for explicit commands. Handles tool changes (switching frameworks, adding quality checks) conversationally.
  #   9. ALL documentation MUST be updated: src/help.ts, src/commands/*-help.ts, spec/CLAUDE.md, docs/, README.md for every aspect of tool configuration
  #   10. When AI detects multiple frameworks, chain them with && (e.g., '<framework1> && <framework2>'). All detected tools run sequentially.
  #   11. Include date-aware search queries in system-reminders (e.g., 'best <platform> testing tools 2025'). Platform placeholder filled by fspec based on project detection.
  #
  # EXAMPLES:
  #   1. AI in validating phase, fspec checks spec/fspec-config.json for tools.test.command, finds none, emits system-reminder: 'No test command configured. Use Read/Glob to detect test framework, then run: fspec configure-tools --test-command <cmd>'
  #   2. AI receives system-reminder, uses Read/Glob tools to detect test framework, runs: fspec configure-tools --test-command '<detected-command>', fspec writes to spec/fspec-config.json
  #   3. Next validation, fspec reads spec/fspec-config.json, finds tools.test.command='<command>', emits system-reminder: 'Run tests: <command>'
  #   4. AI uses Read/Glob to detect multiple test frameworks, chains them: fspec configure-tools --test-command '<framework1> && <framework2>', fspec stores chained command
  #   5. No test tools detected: system-reminder includes date-aware search query 'best <platform> testing tools 2025', AI searches, configures result
  #   6. AI detects quality check tools via Glob, runs: fspec configure-tools --quality-commands '<tool1>' '<tool2>' '<tool3>', fspec stores as array
  #   7. AI runs 'fspec configure-tools --reconfigure', fspec re-emits system-reminder for detection, AI updates config with new tools
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should fspec store detected tool configuration in spec/fspec-config.json or spec/.fspec/tools.json?
  #   A: true
  #
  #   Q: Should the conversational detection happen ONLY on first run, or re-detect on every command if config is missing?
  #   A: true
  #
  #   Q: Should there be an explicit 'fspec configure-tools' command for manual reconfiguration, or only automatic detection?
  #   A: true
  #
  #   Q: When multiple test frameworks detected (e.g., both Vitest and Jest), should AI ask which to use, or use heuristics to pick?
  #   A: true
  #
  #   Q: Should system-reminders include search queries for best practices (e.g., 'Search for best Rust testing tools 2025') or just guide AI to search?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec across different platforms
    I want to detect and configure test and quality check tools conversationally
    So that fspec works on any platform (Node.js, Python, Rust, Go, etc.) without hardcoded assumptions

  Scenario: Emit system-reminder when no test command configured
    Given spec/fspec-config.json does not have tools.test.command configured
    When fspec checks for test command during validating phase
    Then fspec should emit system-reminder
    And system-reminder should say: 'No test command configured'
    And system-reminder should tell AI to use Read/Glob tools to detect framework
    And system-reminder should tell AI to run: fspec configure-tools --test-command <cmd>

  Scenario: Store test command when AI configures tools
    Given AI detected test framework using Read/Glob tools
    When AI runs: fspec configure-tools --test-command '<detected-command>'
    Then fspec should write to spec/fspec-config.json
    And spec/fspec-config.json should contain tools.test.command = '<detected-command>'

  Scenario: Emit configured test command when config exists
    Given spec/fspec-config.json has tools.test.command = '<command>'
    When fspec checks for test command during validating phase
    Then fspec should emit system-reminder: 'Run tests: <command>'
    And fspec should NOT prompt AI to configure tools

  Scenario: Chain multiple test frameworks when AI provides chained command
    Given AI detected multiple test frameworks using Glob
    When AI runs: fspec configure-tools --test-command '<framework1> && <framework2>'
    Then fspec should store chained command in spec/fspec-config.json
    And next validation should emit: 'Run tests: <framework1> && <framework2>'

  Scenario: Include date-aware search queries when no tools detected
    Given AI used Read/Glob and found no test configuration files
    When fspec emits system-reminder for tool configuration
    Then system-reminder should include search query: 'best <platform> testing tools 2025'
    And platform placeholder should be filled based on project detection

  Scenario: Store multiple quality check commands
    Given AI detected quality check tools using Glob
    When AI runs: fspec configure-tools --quality-commands '<tool1>' '<tool2>' '<tool3>'
    Then fspec should store array of commands in spec/fspec-config.json
    And next validation should emit chained command: '<tool1> && <tool2> && <tool3>'

  Scenario: Support reconfiguration when tools change
    Given spec/fspec-config.json has existing tool configuration
    When AI runs: fspec configure-tools --reconfigure
    Then fspec should emit system-reminder for re-detection
    And AI should detect new tools and update configuration

  Scenario: Validate all documentation uses dynamic command placeholders not hardcoded npm
    Given fspec has help files, slash command sections, project management sections, and CLAUDE.md documentation
    When validation test scans all documentation files for hardcoded npm test, npm run build, npm check patterns
    Then test should pass when all examples use <test-command> or <quality-check-commands> placeholders
    Given fspec has help files, slash command sections, project management sections, and CLAUDE.md documentation
    When validation test scans all documentation files for hardcoded npm test, npm run build, npm check patterns
    Then test should pass when all examples use <test-command> or <quality-check-commands> placeholders
    And test should fail if any npm test, npm run, or npm check hardcoded patterns found

  Scenario: Replace placeholders in generated spec/CLAUDE.md with configured commands
    Given slash command section generators (src/utils/slashCommandSections/*.ts) return content with <test-command> and <quality-check-commands> placeholders
    When fspec init command calls slash command section generators and assembles the output
    Then the generated spec/CLAUDE.md file should have all <test-command> placeholders replaced with 'npm test'
    Given slash command section generators (src/utils/slashCommandSections/*.ts) return content with <test-command> and <quality-check-commands> placeholders
    And spec/fspec-config.json has tools.test.command = 'npm test' configured
    When fspec init command calls slash command section generators and assembles the output
    Then the generated spec/CLAUDE.md file should have all <test-command> placeholders replaced with 'npm test'
    And placeholder replacement logic reads spec/fspec-config.json to get configured commands
    And the generated spec/CLAUDE.md file should have all <quality-check-commands> placeholders replaced with configured quality commands
