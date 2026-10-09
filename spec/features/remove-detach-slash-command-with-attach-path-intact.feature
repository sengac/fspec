@done
@bug
@BUG-205
@menu-bar
@agent-view
@slash-command
@tui
@regression
Feature: The /detach slash command is removed while the attach path stays intact
  """
  BUG-205 removed the /detach slash command (RPC-050). Its only effect was
  clearing the session's work-unit binding via backend.set_work_unit_context
  (session, None); with the dialog's Detach option gone it had no UI home, and
  the board's Close Session teardown already clears the binding.

  The detach-only plumbing is deleted with it: SlashCommandAction::Detach and
  its 'detach' registry entry, App::handle_slash_detach,
  App::handle_work_unit_detached, and the Action::WorkUnitDetached variant.
  Typing/picking '/detach' now falls through to backend.send_input like any
  unregistered command (BUG-169 parser: NotASlashCommand).

  The attach side (Action::AttachWorkUnitToSession / WorkUnitAttached /
  handle_attach_work_unit_to_session / handle_work_unit_attached) and the
  backend set_work_unit_context(Some) / get_work_unit_context RPC surface
  remain intact and are pinned here.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   R3: The /detach slash command no longer exists: the
  #       SlashCommandAction::Detach variant and its 'detach' registry entry
  #       are removed from the palette, and typing/picking '/detach' falls
  #       through to backend.send_input like any unregistered command
  #       (BUG-169 parser: NotASlashCommand).
  #   R4: The detach-only plumbing is removed: the Action::WorkUnitDetached
  #       variant, its App::dispatch arm, the handle_work_unit_detached
  #       helper, and handle_slash_detach are deleted. The attach path
  #       (Action::AttachWorkUnitToSession, WorkUnitAttached,
  #       handle_attach_work_unit_to_session, handle_work_unit_attached) and
  #       the backend set_work_unit_context(Some)/get_work_unit_context RPC
  #       surface remain intact.
  #
  # EXAMPLES:
  #   3. User opens the '/' slash palette and the '?' help dialog → neither
  #      lists a /detach command; the agent help slash list ends at
  #      /worktrees.
  #   4. User types '/detach' and presses Enter → the text is forwarded to
  #      the LLM as plain input (no slash handler intercepts it, no notice,
  #      no binding change).
  #
  # ========================================
  Background: User Story
    As a TUI agent
    I want the /detach slash command gone and the attach path intact
    So that a pointless command (whose only effect was clearing a work-unit binding) no longer exists while attaching a work unit still works

  @regression
  Scenario: The slash command palette no longer offers /detach
    Given the slash command registry after the removal
    Then the registry contains no command named "detach"
    And SlashCommandAction::from_name("detach") resolves to None
    And the agent help slash-command list contains no "/detach" row

  @regression
  Scenario: Typing /detach falls through to the LLM like any unregistered command
    Given an App with one open session s-1 in AgentView
    And s-1 is bound to work unit AUTH-001
    When the input is submitted with text "/detach"
    Then the text "/detach" is forwarded to backend.send_input
    And no work-unit binding change is made for session "s-1"

  @regression
  Scenario: The attach path and the backend work-unit context RPC surface remain intact
    Given the codebase after the removal
    And an App with open session s-1 whose BoardStore holds AUTH-001
    When Action::AttachWorkUnitToSession("AUTH-001") is dispatched
    Then backend.set_work_unit_context(Some(ctx)) still binds a work unit
    And folds it into AgentViewStore
