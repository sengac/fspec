@done
@BLOCK-007
Feature: Integrate Blocklist Prompt Action with Tool Pause System
  """
  Add PauseKind::Triple variant to codelet/tools/src/tool_pause.rs alongside Continue and Confirm
  Add PauseResponse::AllowOnce and PauseResponse::AllowSession variants
  Modify check_file_path() and check_bash_command() in middleware.rs: when result.blocked==false && !result.allowed (prompt case), check is_session_allowed(pattern), if false call pause_for_user(Triple), handle response
  Add session_pause_triple(session_id, choice) NAPI binding in codelet/napi/src/session_manager.rs
  Add 'triple' to PauseKind type in src/tui/types/pause.ts
  Modify InputTransition.tsx to show triple-choice UI inline when pauseInfo.kind === 'triple' - same inline style as continue/confirm pauses, NOT a popup dialog
  Add keyboard handler in AgentView.tsx for triple pause: ←/→ to navigate selection, Enter to select, call sessionPauseTriple(sessionId, choice)
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Blocklist rules with action='prompt' trigger the tool pause system with PauseKind::Triple
  #   2. Session allowances are checked BEFORE pausing - if pattern already allowed, skip prompt
  #   3. PauseKind::Triple has three responses: AllowOnce, AllowSession, Deny
  #   4. AllowSession response calls allow_for_session(pattern) to store in session memory
  #   5. AllowOnce response permits the current operation but doesn't store anything
  #   6. Deny response returns BlockedError to the tool with 'User denied access' message
  #   7. TUI shows InputTransition with triple-choice UI when pauseInfo.kind === 'triple'
  #   8. NAPI binding session_pause_triple(session_id, choice) handles triple responses
  #
  # EXAMPLES:
  #   1. AI reads ~/.env → Prompt shows → User selects Allow Once → File read → Next access to ~/.env prompts again
  #   2. AI reads ~/.ssh/config → User selects Allow Session → File read → Later access to ~/.ssh/known_hosts → No prompt (same pattern)
  #   3. AI reads ~/.env → User selects Deny → Read blocked → AI receives 'User denied access to sensitive file' error
  #   4. AI reads ~/.env → User previously allowed .env for session → No prompt, file reads directly
  #   5. TUI shows ⏸ Read: Sensitive file access (.env) with ←/→ navigation between [Allow Once] [Allow Session] [Deny]
  #   6. User restarts TUI → Session allowances cleared → AI reads ~/.env → Prompt shown again
  #
  # ========================================
  Background: User Story
    As a developer using the TUI
    I want to be prompted with Allow Once / Allow Session / Deny when AI accesses sensitive files
    So that I can make informed decisions about file access without completely blocking legitimate AI operations

  # ====================
  # ALLOW ONCE BEHAVIOR
  # ====================
  Scenario: User allows sensitive file access once
    Given a blocklist rule with action "prompt" exists for ".env" files
    And the AI session is active
    When the AI attempts to read "~/.env"
    Then the TUI should show an inline triple pause with message "Sensitive file access (.env)"
    And the pause should display three options: Allow Once, Allow Session, Deny
    When the user presses Enter to select "Allow Once"
    Then the file should be read successfully
    When the AI attempts to read "~/.env" again
    Then the TUI should prompt again

  # ====================
  # ALLOW SESSION BEHAVIOR
  # ====================
  Scenario: User allows sensitive file access for session
    Given a blocklist rule with action "prompt" exists for "~/.ssh" access
    And the AI session is active
    When the AI attempts to read "~/.ssh/config"
    Then the TUI should show an inline triple pause
    When the user navigates right and selects "Allow Session"
    Then the file should be read successfully
    When the AI attempts to read "~/.ssh/known_hosts" later
    Then the file should be read without prompting

  # ====================
  # DENY BEHAVIOR
  # ====================
  Scenario: User denies sensitive file access
    Given a blocklist rule with action "prompt" exists for ".env" files
    And the AI session is active
    When the AI attempts to read "~/.env"
    Then the TUI should show an inline triple pause
    When the user navigates to "Deny" and presses Enter
    Then the read should be blocked
    And the AI should receive an error message "User denied access to sensitive file"

  # ====================
  # SESSION MEMORY
  # ====================
  Scenario: Session allowances bypass prompting
    Given a blocklist rule with action "prompt" exists for ".env" files
    And the user previously allowed ".env" pattern for the session
    When the AI attempts to read "~/.env"
    Then the file should be read without prompting

  Scenario: Session allowances cleared on TUI restart
    Given a blocklist rule with action "prompt" exists for ".env" files
    And the user allowed ".env" pattern for the session
    When the user restarts the TUI
    And the AI attempts to read "~/.env"
    Then the TUI should prompt again
