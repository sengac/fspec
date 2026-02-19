@security
@tools
@BLOCK-002
Feature: Blocklist Core - Command/Tool Filtering
  """
  Rust Blocklist Module: Create codelet/tools/src/blocklist/ with BlocklistConfig (load/save JSON), BlocklistRule struct, BlocklistMatcher (regex evaluation). FilterMiddleware wraps existing tool execution in FacadeToolWrapper, checks rules before passing to base tool.
  NAPI Bindings: Wire up blocklist_load, blocklist_save, blocklist_check. Config stored at ~/.fspec/blocklist.json (user) or .fspec/blocklist.json (project), project takes precedence.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Config files live in ~/.fspec/ (system-level) and <project>/.fspec/ (project-level)
  #   2. Project rules extend/override system rules
  #   3. Command blocking: block specific bash commands (e.g., git checkout) with guidance on what to use instead
  #   4. Tool blocking: block entire tool usage (e.g., don't use Bash for file reading - use Read tool instead)
  #   5. Block action returns reason AND guidance (what to do instead) - not just 'blocked' but educational
  #
  # EXAMPLES:
  #   1. Command block with guidance: AI runs 'git checkout main' → Error: 'Blocked: git checkout is deprecated. Use git switch main instead.'
  #   2. Tool redirect: AI runs 'cat src/file.ts' via Bash → Error: 'Blocked: Use the Read tool for file reading, not Bash.'
  #   3. Config hierarchy: System blocks 'rm -rf' → Project allows 'rm -rf ./node_modules' → AI runs it → Allowed
  #
  # ========================================
  Background: Blocklist Configuration
    Given the user has fspec configured with blocklist rules

  # ====================
  # COMMAND BLOCKING
  # ====================
  Scenario: Block dangerous command with guidance
    Given a blocklist rule exists blocking "git checkout" with reason "Use git switch instead"
    When the AI runs "git checkout main" via Bash
    Then the command should be blocked
    And the AI should receive error "Blocked: git checkout is deprecated. Use git switch main instead."

  Scenario: Block Bash usage for file reading with tool guidance
    Given a blocklist rule exists blocking "cat" commands with reason "Use Read tool instead"
    When the AI runs "cat src/file.ts" via Bash
    Then the command should be blocked
    And the AI should receive error "Blocked: Use the Read tool for file reading, not Bash. This ensures proper encoding and line number display."

  # ====================
  # CONFIG HIERARCHY
  # ====================
  Scenario: Project config overrides system config
    Given system blocklist at "~/.fspec/blocklist.json" blocks "rm -rf"
    And project blocklist at ".fspec/blocklist.json" allows "rm -rf ./node_modules"
    When the AI runs "rm -rf ./node_modules"
    Then the command should be allowed
    When the AI runs "rm -rf /"
    Then the command should be blocked
