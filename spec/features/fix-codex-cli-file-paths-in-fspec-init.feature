@feature-management
@cli
@setup
@wip
@init
@INIT-015
Feature: Fix Codex CLI file paths in fspec init

  """
  Updates work-units.json descriptions and attachment documentation for Codex CLI references
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Codex CLI must use spec/AGENTS.md for documentation, not spec/CODEX-CLI.md
  #   2. Codex CLI must use .codex/prompts/fspec.md for prompt file, not .codex/commands/fspec.md
  #   3. All references to the old paths must be updated (agentRegistry.ts, .gitignore, work-units.json, docs)
  #   4. Yes, both Codex and Codex CLI should use spec/AGENTS.md
  #
  # EXAMPLES:
  #   1. Running 'fspec init --agent=codex-cli' creates .codex/prompts/fspec.md and spec/AGENTS.md
  #   2. agentRegistry.ts docTemplate for codex-cli is 'AGENTS.md' not 'CODEX-CLI.md'
  #   3. .gitignore excludes .codex/prompts/fspec.md not .codex/commands/fspec.md
  #   4. Work unit descriptions reference the correct paths for Codex CLI
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should the Codex agent (non-CLI) also use spec/AGENTS.md, or only Codex CLI?
  #   A: true
  #
  #   Q: Are there any other files beyond agentRegistry.ts, .gitignore, work-units.json, and attachment docs that need updating?
  #   A: true
  #
  #   Q: Should we also check template files or other generated content for these path references?
  #   A: true
  #
  # ASSUMPTIONS:
  #   1. No, only agentRegistry.ts, .gitignore, work-units.json, and attachment docs need updating
  #   2. No, template files and generated content do not need to be checked
  #   3. No, only agentRegistry.ts, .gitignore, work-units.json, and attachment docs need updating
  #   4. No, template files and generated content do not need to be checked
  #   5. No, template files and generated content do not need to be checked
  #
  # ========================================

  Background: User Story
    As a developer using fspec with Codex CLI
    I want to initialize fspec with correct Codex CLI file paths
    So that the agent loads the correct prompt file and documentation

  Scenario: Codex CLI agent registry uses correct documentation path
    Given the agentRegistry.ts file exists
    When I check the codex-cli agent configuration
    Then the docTemplate should be "AGENTS.md"
    And the docTemplate should not be "CODEX-CLI.md"


  Scenario: Codex CLI agent registry uses correct prompt file path
    Given the agentRegistry.ts file exists
    When I check the codex-cli agent configuration
    Then the promptPath should be ".codex/prompts/fspec.md"
    And the promptPath should not be ".codex/commands/fspec.md"


  Scenario: Gitignore excludes correct Codex CLI prompt path
    Given the .gitignore file exists
    When I check the gitignore entries
    Then it should contain ".codex/prompts/fspec.md"
    And it should not contain ".codex/commands/fspec.md"

