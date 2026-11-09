@done
@critical
@cli
@configuration
@setup
@cross-platform
@CONFIG-002
Feature: Conversational Test and Quality Check Tool Detection
  """
  fspec does no framework detection: system-reminders guide the AI agent, which detects tools with Read/Glob/Grep and stores them with fspec configure-tools (spec/fspec-config.json: tools.test.command, tools.qualityCheck.commands). Storage, --reconfigure and help are proven by configure-tools-rust-port and configure-tools-cli-subcommand; the validating reminders (missing command, configured command, chained commands, date-aware search query) by tool-detection-check-functions-not-integrated-into-workflow. This feature keeps the agent-doc placeholder substitution: rust/fspec-core/src/commands/init/templates.rs generate_agent_doc replaces <test-command> and <quality-check-commands> in the generated agent doc from the merged user and project config (quality commands chained with &&).
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
  #   9. ALL documentation MUST be updated: src/help.ts, src/commands/*-help.ts, spec/AGENTS.md, docs/, README.md for every aspect of tool configuration
  #   10. When AI detects multiple frameworks, chain them with && (e.g., '<framework1> && <framework2>'). All detected tools run sequentially.
  #   11. Include date-aware search queries in system-reminders (e.g., 'best <platform> testing tools'). Platform placeholder filled by fspec based on project detection.
  #
  # EXAMPLES:
  #   1. AI in validating phase, fspec checks spec/fspec-config.json for tools.test.command, finds none, emits system-reminder: 'No test command configured. Use Read/Glob to detect test framework, then run: fspec configure-tools --test-command <cmd>'
  #   2. AI receives system-reminder, uses Read/Glob tools to detect test framework, runs: fspec configure-tools --test-command '<detected-command>', fspec writes to spec/fspec-config.json
  #   3. Next validation, fspec reads spec/fspec-config.json, finds tools.test.command='<command>', emits system-reminder: 'Run tests: <command>'
  #   4. AI uses Read/Glob to detect multiple test frameworks, chains them: fspec configure-tools --test-command '<framework1> && <framework2>', fspec stores chained command
  #   5. No test tools detected: system-reminder includes date-aware search query 'best <platform> testing tools', AI searches, configures result
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
  #   Q: Should system-reminders include search queries for best practices (e.g., 'Search for best Rust testing tools') or just guide AI to search?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec across different platforms
    I want to detect and configure test and quality check tools conversationally
    So that fspec works on any platform (Node.js, Python, Rust, Go, etc.) without hardcoded assumptions

  Scenario: Replace placeholders in generated spec/AGENTS.md with configured commands
    Given spec/fspec-config.json has tools.test.command = 'npm test' and tools.qualityCheck.commands = ['npm run lint', 'npm run fmt']
    When fspec init generates spec/AGENTS.md for the claude agent
    Then the generated spec/AGENTS.md file should have all <test-command> placeholders replaced with 'npm test'
    And the generated spec/AGENTS.md file should have all <quality-check-commands> placeholders replaced with 'npm run lint && npm run fmt'
