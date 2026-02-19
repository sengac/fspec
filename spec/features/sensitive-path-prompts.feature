@BLOCK-005
Feature: Sensitive Path Prompts
  """
  Prompt Dialog: Extend existing ConfirmationDialog component with a new 'triple' confirmMode that shows three buttons [Allow Once] [Allow Session] [Deny]. Uses visual mode pattern with ←/→ navigation. Triggered from tool facade middleware when rule.action='prompt'.
  Session Allowances: Stored in Arc<RwLock<HashSet<String>>> keyed by matched pattern. Wire up NAPI binding blocklist_allow_session. Memory cleared on TUI restart.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Sensitive path access: PROMPT (not block) for paths like ~/.fspec, ~/.ssh, ~/.aws, .env files
  #   2. User decides in context with Allow Once / Allow Session / Deny options
  #   3. Allow Session grants access to matching pattern for entire session
  #   4. Session allowances cleared on TUI restart
  #
  # EXAMPLES:
  #   1. Sensitive path prompt: AI reads ~/.ssh/config → Prompt → User chooses Allow Once → File read
  #   2. Session allowance: User chooses Allow Session → AI reads ~/.ssh/known_hosts → No prompt
  #   3. Deny: User chooses Deny → Read blocked → AI receives error
  #
  # ========================================
  Background: Sensitive Path Rules Configured
    Given the user has fspec configured with sensitive path rules

  # ====================
  # SSH DIRECTORY ACCESS
  # ====================
  Scenario: Prompt for SSH config access - user allows once
    Given a blocklist rule exists prompting for "~/.ssh" access with reason "SSH directory contains private keys"
    When the AI tries to read "~/.ssh/config"
    Then the user should see a prompt "AI wants to read SSH config - Allow Once / Allow Session / Deny?"
    When the user selects "Allow Once"
    Then the file should be read successfully
    And subsequent access to "~/.ssh/config" should prompt again

  Scenario: Prompt for SSH config access - user allows for session
    Given a blocklist rule exists prompting for "~/.ssh" access
    When the AI tries to read "~/.ssh/config"
    And the user selects "Allow Session"
    Then the file should be read successfully
    When the AI tries to read "~/.ssh/known_hosts" later in the same session
    Then the file should be read without prompting

  Scenario: Prompt for SSH config access - user denies
    Given a blocklist rule exists prompting for "~/.ssh" access
    When the AI tries to read "~/.ssh/config"
    And the user selects "Deny"
    Then the read should be blocked
    And the AI should receive an error indicating user denied access

  # ====================
  # ENVIRONMENT FILES
  # ====================
  Scenario: Prompt for environment file access
    Given a blocklist rule exists prompting for ".env" files with reason "Environment files may contain secrets"
    When the AI tries to read ".env"
    Then the user should see a prompt "AI wants to read environment file (may contain secrets) - Allow Once / Allow Session / Deny?"

  # ====================
  # FSPEC CONFIG
  # ====================
  Scenario: Prompt for fspec config access
    Given a blocklist rule exists prompting for "~/.fspec" access
    When the AI tries to read "~/.fspec/blocklist.json"
    Then the user should see a prompt "AI wants to read fspec config - Allow Once / Allow Session / Deny?"

  # ====================
  # SESSION MEMORY
  # ====================
  Scenario: Session allowances cleared on TUI restart
    Given a blocklist rule prompts for "npm install" commands
    When the AI runs "npm install" and user allows for session
    Then the AI can run "npm install lodash" without prompting
    When the user exits and restarts the TUI
    And the AI runs "npm install axios"
    Then the user should be prompted again
