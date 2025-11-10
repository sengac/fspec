@done
@discovery
@p1
@cli
@research-tools
@RES-009
Feature: Research Tool Discovery and Configuration UX
  """
  Feedback loop pattern for tool suggestions via system-reminders (no hardcoded semantic analysis)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Tool listing must show tool status (configured vs unconfigured) at a glance
  #   2. Tool listing must include brief description of what each tool does and when to use it
  #   3. System-reminder when entering specifying state must prominently suggest research tools with status
  #   4. Tool-specific help must be accessible via --help flag (not generic research help)
  #   5. Configuration wizard must guide setup for unconfigured tools with validation
  #   6. Research tool configs must support environment variables for API keys (avoid plain text)
  #   7. Config validation command must check all tools and report which are ready vs need setup
  #   8. Research output with --work-unit flag must prompt to save as attachment automatically
  #   9. Attachment prompt must offer to extract rules/examples from research findings
  #   10. AST tool must work in production mode (not just FSPEC_TEST_MODE=1)
  #   11. No environment variables for API keys - use config file only
  #   12. AI auto-parse research findings but ask user for clarification when uncertain (Example Mapping approach)
  #   13. Three configuration methods: 1) CLI flags (--api-key=...), 2) Interactive wizard (detect TTY), 3) TUI config view (in main fspec TUI)
  #   14. Yes, but use feedback loop pattern like 'fspec reverse' - emit system-reminder with tool suggestions, let AI analyze and choose, no hardcoded semantic analysis
  #   15. AST tool help/interface exists but production implementation blocked - needs actual AST parsing logic implemented and tested (currently only works with FSPEC_TEST_MODE=1)
  #
  # EXAMPLES:
  #   1. BEFORE: Run 'fspec research' → get list of tool names only (ast, confluence, jira, perplexity, stakeholder)
  #   2. AFTER: Run 'fspec research' → see status (✓ configured ✗ not configured), descriptions, and usage hints
  #   3. BEFORE: Run 'fspec research --tool=perplexity --help' → get GENERIC research help (not tool-specific)
  #   4. AFTER: Run 'fspec research --tool=perplexity --help' → get perplexity script's --help output (query, model, format options)
  #   5. BEFORE: Try perplexity tool → get error about missing API key, manually create ~/.fspec/fspec-config.json
  #   6. AFTER: Run 'fspec research --configure perplexity' → wizard prompts for API key, validates, saves config
  #   7. BEFORE: Research output to stdout → manually save to file → manually attach with 'fspec add-attachment'
  #   8. AFTER: Research with --work-unit flag → prompts 'Save to attachments? (y/n)' → auto-attaches → offers to extract rules
  #   9. BEFORE: AST tool requires FSPEC_TEST_MODE=1 → unusable in production Example Mapping workflow
  #   10. AFTER: AST tool works in production without test mode flag
  #   11. CURRENT: Perplexity script has outdated model 'llama-3.1-sonar-small-128k-online' → must guess 'sonar'
  #   12. FIXED: Perplexity default model is up-to-date and works without guessing
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should research tool configs stay in ~/.fspec/fspec-config.json or move to project spec/fspec-config.json?
  #   A: true
  #
  #   Q: Which environment variables should be supported? (e.g., PERPLEXITY_API_KEY, JIRA_TOKEN, CONFLUENCE_TOKEN, etc.)
  #   A: true
  #
  #   Q: For rule/example extraction from research: AI auto-parse or AI suggest + human approve?
  #   A: true
  #
  #   Q: Config wizard: Interactive prompts (y/n questions) or command-line flags (--api-key=...)
  #   A: true
  #
  #   Q: Should system proactively suggest which research tool to use based on question type?
  #   A: true
  #
  #   Q: AST production mode - are there technical blockers or just not implemented yet?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. ~/.fspec/fspec-config.json (user-level config)
  #
  # ========================================
  Background: User Story
    As a AI agent using fspec for Example Mapping
    I want to discover and use research tools easily without reading source code
    So that I can answer technical questions efficiently and integrate findings into work units

  Scenario: List research tools with configuration status and descriptions
    Given I have no research tools configured
    When I run 'fspec research'
    Then I should see tool names with status indicators (✓ configured, ✗ not configured)
    And I should see brief descriptions of what each tool does
    And I should see usage hints for each tool

  Scenario: Get tool-specific help output
    Given I want to know how to use the perplexity tool
    When I run 'fspec research --tool=perplexity --help'
    Then I should see the perplexity script's --help output
    And the output should include query, model, and format options

  Scenario: Execute research tool with arguments forwarded
    Given I want to search for async functions using AST tool
    When I run "fspec research --tool=ast --query 'find all async functions'"
    Then the AST tool should execute with the --query flag forwarded
    And the output should contain async function results

