@RES-022
Feature: Research tools fail when invoked via Fspec tool - reads process.argv instead of Commander args
  """
  Fix location: src/commands/research.ts line 304. The registerResearchCommand action handler must use varArgs (from Commander.js) instead of process.argv.slice(2). The varArgs array contains unknown options forwarded by allowUnknownOption(). Individual research tools (ast.ts, perplexity.ts, jira.ts, confluence.ts, stakeholder.ts) correctly parse args - no changes needed there.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Research command must use Commander.js varArgs parameter instead of process.argv
  #   2. Arguments forwarded to research tools must work identically via CLI and Fspec tool
  #   3. All 5 research tools (ast, perplexity, jira, confluence, stakeholder) must receive correct arguments
  #
  # EXAMPLES:
  #   1. Fspec tool calls research with ast tool and pattern arg - tool receives the pattern correctly
  #   2. Fspec tool calls research with ast tool but missing required arg - tool throws appropriate error
  #   3. CLI calls research with ast tool - behavior unchanged from before fix
  #
  # ========================================
  Background: User Story
    As a AI agent
    I want to use research tools via the Fspec tool
    So that I can research code patterns during Example Mapping without needing CLI access

  Scenario: Research tool receives correct arguments when invoked via Fspec tool
    Given the research command is registered with Commander.js
    And the ast research tool is available
    When I invoke research via the Fspec tool with arguments "--tool=ast --pattern=function --lang=typescript"
    Then the ast tool should receive the arguments ["--pattern=function", "--lang=typescript"]
    And the tool should execute successfully

  Scenario: Research tool throws error for missing required arguments via Fspec tool
    Given the research command is registered with Commander.js
    And the ast research tool is available
    When I invoke research via the Fspec tool with arguments "--tool=ast --lang=typescript"
    Then the ast tool should throw an error containing "--pattern is required"

  Scenario: CLI invocation continues to work after fix
    Given the research command is registered with Commander.js
    And the ast research tool is available
    When I invoke research via CLI with arguments "research --tool=ast --pattern=function --lang=typescript"
    Then the ast tool should receive the arguments ["--pattern=function", "--lang=typescript"]
    And the tool should execute successfully
