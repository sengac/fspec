@done
@discovery
@high
@cli
@research-tools
@RES-018
Feature: Unconfigured research tool visibility and discovery
  """
  Uses existing research tool registry and loadConfig() validation from each tool. Adds new --all flag to 'fspec research' command. Modifies system-reminder output to always show all tools. Updates error messages to include full JSON config examples.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Option B with enhancement: Show all tools with visual distinction in system-reminders for AI awareness. User-facing output shows configured tools by default, with --all flag to show unconfigured tools. This balances AI discoverability with clean user experience.
  #   2. Option A: Fail fast with setup instructions. Show clear error message with step-by-step setup guide and suggest alternative configured tools. This is clear, non-interactive (good for AI agents), and educational.
  #   3. Option A: Always show all tools equally in system-reminders with accurate configuration information. Show config file paths (spec/fspec-config.json or ~/.fspec/fspec-config.json) and JSON structure, NOT environment variable examples.
  #   4. Configuration uses JSON files at spec/fspec-config.json (project) or ~/.fspec/fspec-config.json (user), with structure: {"research": {"toolName": {config}}}
  #   5. AST tool requires no configuration. Perplexity requires apiKey. JIRA requires jiraUrl, username, apiToken. Confluence requires confluenceUrl, username, apiToken. Stakeholder requires at least one platform webhook/token.
  #   6. Tools validate configuration on execution and throw helpful errors with setup instructions pointing to config file locations
  #   7. Option A: Try to validate the full configuration. Use the existing loadConfig() and validation logic from each tool to accurately determine if it's configured. Accept the slight performance cost for accuracy.
  #   8. Option B: Show full JSON example inline. This is clear and immediately actionable - users can copy-paste the JSON structure directly into their config file. System-reminders can be more concise since AI agents can run --help commands.
  #   9. User-facing 'fspec research' output shows only configured tools by default. Use --all flag to show unconfigured tools with setup instructions.
  #   10. System-reminders (AI-only) always show all tools with status to enable AI agents to suggest and guide setup for better tools.
  #
  # EXAMPLES:
  #   1. User runs 'fspec research' with no tools configured. Output shows only AST (no config needed) as available. Shows message about other tools with --all flag.
  #   2. User runs 'fspec research --all'. Output shows all 5 tools: AST (✓ ready), Perplexity (✗ not configured - needs config.research.perplexity.apiKey in spec/fspec-config.json), etc.
  #   3. AI agent sees system-reminder listing all 5 tools with configuration status and JSON config structure examples for each unconfigured tool.
  #   4. User runs 'fspec research --tool=perplexity --query="test"' without config. Error message shows: missing apiKey, how to add to spec/fspec-config.json with JSON example, suggests using AST instead.
  #   5. User has only AST configured. Runs 'fspec research'. Sees: 'Available: ast (ready)' and footer message: 'Run fspec research --all to see 4 additional tools that require configuration.'
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should 'fspec research' list only configured tools, all tools with visual distinction, or have a flag to show all?
  #   A: true
  #
  #   Q: When using an unconfigured tool, should it fail with setup instructions, offer alternative tool, have interactive setup, or different behavior for AI vs humans?
  #   A: true
  #
  #   Q: In system-reminders, should all tools be shown equally, or should configured tools be emphasized with unconfigured tools mentioned separately?
  #   A: true
  #
  #   Q: For checking if tools are configured, should we do full validation, quick key existence check, or hybrid approach?
  #   A: true
  #
  #   Q: For setup guidance, show minimal hint, full JSON example inline, or reference setup command?
  #   A: true
  #
  # ========================================
  Background: User Story
    As a developer or AI agent using fspec research tools
    I want to discover available research tools even when not configured
    So that I know what tools exist and can configure them when needed

  Scenario: List tools with no tools configured
    Given I have no research tools configured in spec/fspec-config.json
    When I run 'fspec research'
    Then I should see only AST listed as available
    And I should see a message about using --all to see unconfigured tools

  Scenario: List all tools including unconfigured ones
    Given I have no research tools configured except AST
    When I run 'fspec research --all'
    Then I should see all 5 research tools listed
    And AST should show as configured with ✓ indicator
    And Perplexity should show as not configured with ✗ indicator
    And each unconfigured tool should show JSON config example

  Scenario: System-reminder shows all tools to AI agents
    Given I am an AI agent
    When a system-reminder about research tools is displayed
    Then I should see all 5 tools with configuration status
    And some research tools are not configured
    And each unconfigured tool should show JSON config structure
    And config file paths should be mentioned

  Scenario: Error when using unconfigured tool
    Given Perplexity is not configured
    When I run 'fspec research --tool=perplexity --query="test"'
    Then the command should fail with exit code 1
    And the error should mention missing apiKey
    And the error should show JSON config example for spec/fspec-config.json
    And the error should suggest using AST as alternative

  Scenario: Discovery message when listing configured tools
    Given only AST is configured
    When I run 'fspec research'
    Then I should see AST marked as ready
    And I should see a footer message stating the number of unconfigured tools
    And the footer should mention using --all to see them
