@BLOCK-004
Feature: Blocklist TUI - List/Form Views
  """
  TUI Components: Create BlocklistListView.tsx (full-screen overlay, follows WatcherTemplateList pattern) and BlocklistFormView.tsx (full-screen form, follows WatcherCreateView pattern). Both use position=absolute with full width/height, black background, header with border, scrollable content, footer with keyboard hints. Use useInputCompat with InputPriority.CRITICAL.
  Integration Points: Register /blocklist in slashCommands.ts. Add isBlocklistMode state to AgentView.tsx.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. /blocklist as the TUI command name
  #   2. Rules can be toggled on/off dynamically in session via TUI command
  #   3. Session toggles are temporary - restored on TUI restart
  #   4. List view shows all rules from both system and project config
  #   5. Rules show their current state (enabled/disabled for session)
  #
  # EXAMPLES:
  #   1. View rules: User runs /blocklist → sees list of all rules with toggle state
  #   2. Session toggle: User runs /blocklist → disables 'git-checkout-block' rule → AI runs 'git checkout main' → Allowed
  #   3. Re-enable: User disables rule → later re-enables it → rule is enforced again
  #   4. Session cleared: User restarts TUI → 'git-checkout-block' rule is active again
  #
  # ========================================
  Background: Blocklist TUI Available
    Given the user has fspec TUI running
    And blocklist rules are configured

  # ====================
  # VIEWING RULES
  # ====================
  Scenario: View blocklist rules via /blocklist command
    Given system blocklist has rules "git-checkout-block" and "cat-block"
    And project blocklist has rule "rm-rf-allow-node-modules"
    When the user runs "/blocklist" command
    Then the user should see a full-screen overlay
    And the overlay should show all rules from system and project configs
    And each rule should show its name, pattern, and action type

  Scenario: View rule details
    Given a blocklist rule "git-checkout-block" exists
    When the user runs "/blocklist" command
    And the user selects "git-checkout-block"
    Then the user should see the rule details
    And the details should show the regex pattern
    And the details should show the guidance message

  # ====================
  # SESSION RULE TOGGLE
  # ====================
  Scenario: User disables rule for session
    Given a blocklist rule "git-checkout-block" exists blocking "git checkout"
    When the user runs "/blocklist" command
    And the user disables the "git-checkout-block" rule for this session
    Then the AI should be able to run "git checkout main"
    And the rule should show as "disabled (session)" in the list

  Scenario: User re-enables previously disabled rule
    Given the user has disabled "git-checkout-block" rule for this session
    When the user runs "/blocklist" command
    And the user enables the "git-checkout-block" rule
    Then the AI should be blocked from running "git checkout main"
    And the rule should show as "enabled" in the list

  Scenario: Session toggles cleared on TUI restart
    Given the user has disabled "git-checkout-block" rule for this session
    When the user restarts the TUI
    And the user runs "/blocklist" command
    Then the "git-checkout-block" rule should show as "enabled"
    And the AI should be blocked from running "git checkout main"

  # ====================
  # KEYBOARD NAVIGATION
  # ====================
  Scenario: Navigate blocklist with keyboard
    Given the user has run "/blocklist" command
    When the user presses "j" or down arrow
    Then the selection should move to the next rule
    When the user presses "k" or up arrow
    Then the selection should move to the previous rule
    When the user presses "Escape"
    Then the blocklist overlay should close
