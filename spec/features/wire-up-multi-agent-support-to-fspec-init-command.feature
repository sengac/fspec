@setup
@done
@phase1
@cli
@initialization
@multi-agent-support
@refactoring
@INIT-006
Feature: Wire up multi-agent support to fspec init command
  """
  Architecture notes:
  - Refactor registerInitCommand to use new multi-agent infrastructure
  - Replace old single-agent init() with installAgents()
  - Remove deprecated functions after migration is complete

  Critical implementation requirements:
  - Interactive mode: Parse no --agent flag → show agent selector → call installAgents()
  - CLI mode: Parse --agent flags → skip selector → call installAgents() directly
  - Remove old --type and --path options (replaced by --agent)
  - Delete init(), generateTemplate(), copyClaudeTemplate() functions

  Dependencies:
  - Existing installAgents() function (src/commands/init.ts)
  - Agent registry (src/utils/agentRegistry.ts)
  - Agent detection (src/utils/agentDetection.ts)
  - Interactive selector (to be implemented using ink/react)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. registerInitCommand must call installAgents() with interactive selector when no --agent flag provided
  #   2. Old init() function, generateTemplate(), and copyClaudeTemplate() must be removed after migration
  #   3. Command must accept --agent flag (can be repeated) for non-interactive mode
  #   4. Interactive selector must use agent auto-detection to pre-select detected agents
  #   5. Slash command template must be embedded in src/utils/slashCommandTemplate.ts as TypeScript string literal
  #   6. Agent doc template (AGENT.md) must be embedded in src/utils/agentDocTemplate.ts as TypeScript string literal
  #   7. Templates must use template literal placeholders like ${agentName} for agent-specific replacements
  #   8. No file path dependencies - must work after npm install -g fspec
  #   9. Remove all __dirname usage from init.ts and templateGenerator.ts
  #
  # EXAMPLES:
  #   1. User runs 'fspec init' with no flags, sees interactive agent selector with auto-detected agents pre-selected
  #   2. User runs 'fspec init --agent=cursor', skips interactive selector and installs Cursor directly
  #   3. After migration, old init(), generateTemplate(), copyClaudeTemplate() functions are deleted from init.ts
  #   4. User runs 'npm install -g fspec' then 'fspec init' in fresh project, templates are loaded from embedded code not files
  #   5. Vite bundles dist/index.js with embedded template strings, no external file reads needed
  #   6. Template contains ${agentName} which gets replaced with 'Claude Code', 'Cursor', etc during generation
  #
  # ========================================
  Background: User Story
    As a developer using any AI coding agent
    I want to run 'fspec init' and get an interactive agent selector
    So that I can choose my agent and get proper setup without needing to know the --agent flag syntax

  Scenario: Interactive mode shows agent selector
    Given I am in a project directory
    And no --agent flag is provided
    When I run "fspec init"
    Then an interactive agent selector should be displayed
    And cursor highlights the first available agent (or auto-detected agent if present)
    And I can navigate with arrow keys (↑↓)
    And pressing Enter immediately installs the highlighted agent
    And installAgents() is called with the single selected agent

  Scenario: CLI mode skips interactive selector
    Given I am in a project directory
    When I run "fspec init --agent=cursor"
    Then the interactive selector should not be displayed
    And installAgents() should be called with ["cursor"]
    And Cursor-specific files should be installed

  Scenario: Deprecated code is removed
    Given the multi-agent wiring is complete
    When I inspect src/commands/init.ts
    Then the init() function should not exist
    And the generateTemplate() function should not exist
    And the copyClaudeTemplate() function should not exist
    And the old --type and --path options should not exist

  Scenario: Works after global npm install
    Given I have installed fspec globally with "npm install -g fspec"
    When I run "fspec init" in a fresh project directory
    Then the command should execute successfully
    And agent-specific files should be created with correct content
    And no errors about missing files or templates should occur

  Scenario: Agent-specific customization
    Given I run "fspec init --agent=cursor"
    When the installation completes
    Then generated files should contain "Cursor" not "Claude Code"
    And the slash command path should be ".cursor/commands/"
    And the documentation should reference "CURSOR.md"

  Scenario: Embedded templates work independently
    When I run fspec init in any directory
    Then templates are loaded from embedded TypeScript modules
    And the command works without requiring external template files
    And all agents receive consistent template content
