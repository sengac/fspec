@done
@session
@git
@state-history
@session-merge
@rust
@WT-018
Feature: Discarding a worktree names the cards its session changed
  """
  Board model (product owner, 2026-11-08): every board write made while the project root holds a state-root pointer records the pointer's session id: stateHistory entries carry sessionId, a created card carries createdInSession, and every other edit (rules, examples, questions, architecture notes, user story, estimate, dependencies, attachments, any card field) records the session id on what it writes, so the cards a session moved, created or edited are found exactly, with no inference. Discard: the TUI /merge-worktree Discard asks the daemon for the session's card changes before discarding and shows them in the confirmation; 'discard and move back' runs a discard transition from the main checkout that restores each moved card's status from before the session (backlog included; the only path back to backlog) and appends a stateHistory entry with reason discard; a card whose latest status entry is not the session's is shown as changed by someone else since and is not moved. Wide board-model change: done in a private worktree and landed as one compiling commit.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every stateHistory entry a command writes while its project root holds a state-root pointer records the pointer's session id (sessionId); an entry written without a pointer has no sessionId
  #   2. A card created while the project root holds a state-root pointer records the creating session's id (createdInSession); a card created without a pointer has none
  #   3. The status before the session's first change is the state of the stateHistory entry just before that change; a card with no earlier entry was in backlog
  #   4. The user chooses between discarding and moving the listed cards back, discarding and keeping every status, or cancelling; nothing on the board changes without that choice
  #   5. Cards the session created are listed and kept, so their IDs are never reused; rules, examples and questions the session added are kept
  #   6. A session that moved and created no card discards exactly as today, with no card list
  #   7. (a) Discard and move back restores each card's exact status from before the session, backlog included. It is the only path allowed to move a card back to backlog, and the move is recorded in the card's state history as a discard (product owner, 2026-11-08)
  #   8. Every board edit (rules, examples, questions, every other edit) records the id of the session that made it, so discard lists exactly the cards the session moved, created and edited; a board-model change, exact, with no inference (product owner, 2026-11-08)
  #   9. A card whose status someone else changed after this session's last change to it is shown as 'changed by someone else since' and is not moved (product owner, 2026-11-08)
  #   10. Before a worktree is discarded, the confirmation lists every card on the shared board that the session moved, created or edited (entries carrying its session id), each with its id, title and current status, and for a moved card the status it had before the session's first change
  #   11. Moving a card back runs from the main checkout and is recorded in the card's stateHistory as a discard; it restores the exact status from before the session, backlog included (the only path back to backlog); a move that fails is reported naming the card and the error, and the other cards still move
  #
  # EXAMPLES:
  #   1. In a worktree whose pointer names session s1, update-work-unit-status STORY-001 specifying appends {state: specifying, timestamp, sessionId: s1}; the same command in the main checkout appends an entry without sessionId
  #   2. In a worktree of session s1, create-story writes a card holding createdInSession s1
  #   3. Session s1 moved STORY-001 from specifying to testing and then implementing, and created BUG-009: the discard confirmation lists STORY-001 (implementing, before the session: specifying) and BUG-009 (created, kept)
  #   4. Session s1 moved STORY-002 from backlog to specifying; the confirmation lists STORY-002 with before the session: backlog
  #   5. The user picks discard and move back: the worktree is removed, STORY-001 is moved back to specifying through update-work-unit-status from the main checkout, and BUG-009 stays
  #   6. The user picks discard and keep statuses: the worktree is removed and every card keeps its status
  #   7. A move back is refused by a status gate: the notice names the card and the gate's error, and the other listed cards are moved back
  #   8. Session s1 changed no card: Discard shows no card list and removes the worktree as today
  #   9. Session s1 only added a rule to STORY-003: the confirmation lists STORY-003 as edited, and discard and move back leaves its status alone
  #   10. Session s1 moved STORY-004 to testing, then the user moved it to implementing from the main checkout: the confirmation shows STORY-004 as changed by someone else since, and discard and move back does not move it
  #
  # QUESTIONS (ANSWERED):
  #   Q: Most sessions take a card out of backlog, but update-work-unit-status refuses every move back to backlog ('Cannot move work back to backlog'). When a card's status before the session was backlog, should discard and move back (a) move it to backlog anyway, as a discard-only exception, (b) leave its status and only name it, or (c) move it to the earliest status allowed (specifying)?
  #   A: (a) Discard and move back restores each card's exact status from before the session, backlog included. It is the only path allowed to move a card back to backlog, and the move is recorded in the card's state history as a discard (product owner, 2026-11-08)
  #
  #   Q: The card says the confirmation names cards the session 'moved or edited'. The board records no author for rules, examples, questions or other edits. Should the list also name cards the session only edited without moving (each edit would then record the session id), or only cards it moved or created?
  #   A: Every board edit (rules, examples, questions, every other edit) records the id of the session that made it, so discard lists exactly the cards the session moved, created and edited; a board-model change, exact, with no inference (product owner, 2026-11-08)
  #
  #   Q: If another session or the user changed a card's status after this session's last change to it, should discard still offer to move it back to its status before the session, or only name it as changed since and leave its status?
  #   A: A card whose status someone else changed after this session's last change to it is shown as 'changed by someone else since' and is not moved (product owner, 2026-11-08)
  #
  # ========================================
  Background: User Story
    As a user who discards an isolated session's worktree
    I want to see which cards that session moved or created, and choose to move them back to their status before the session
    So that the shared board never claims progress whose code was thrown away

  Scenario: Board writes made from a worktree record the session id
    Given a worktree whose state-root pointer names session "s1"
    When update-work-unit-status moves STORY-001 to specifying, create-story creates a card and add-rule adds a rule to STORY-003 in the worktree
    Then STORY-001's new stateHistory entry carries sessionId "s1"
    And the new card carries createdInSession "s1"
    And STORY-003's new rule carries the session id "s1"
    And the same commands run in the main checkout record no session id

  Scenario: The discard confirmation lists the cards the session moved, created and edited
    Given session "s1" moved STORY-001 from specifying to implementing, moved STORY-002 from backlog to specifying, created BUG-009 and added a rule to STORY-003
    When the user chooses Discard in the merge dialog of session "s1"
    Then the confirmation lists STORY-001 as moved, before the session: specifying
    And it lists STORY-002 as moved, before the session: backlog
    And it lists BUG-009 as created and kept
    And it lists STORY-003 as edited

  Scenario: Discard and move back restores each card's status from before the session
    Given the discard confirmation of session "s1" lists STORY-001 (before: specifying) and STORY-002 (before: backlog)
    When the user picks discard and move back
    Then the worktree is removed
    And STORY-001 is in specifying and STORY-002 is in backlog
    And each card's newest stateHistory entry records the move as a discard
    And BUG-009 and STORY-003's rule stay

  Scenario: Discard and keep statuses changes no card
    Given the discard confirmation of session "s1" lists moved cards
    When the user picks discard and keep statuses
    Then the worktree is removed
    And every card keeps its status

  Scenario: A card someone else moved since is shown and not moved
    Given session "s1" moved STORY-004 to testing and the user then moved it to implementing from the main checkout
    When the user chooses Discard in the merge dialog of session "s1" and picks discard and move back
    Then the confirmation shows STORY-004 as changed by someone else since
    And STORY-004 stays in implementing

  Scenario: A move back that fails is reported and the other cards still move
    Given the discard confirmation of session "s1" lists STORY-001 and STORY-005
    And moving STORY-005 back fails
    When the user picks discard and move back
    Then the notice names STORY-005 and the error
    And STORY-001 is moved back

  Scenario: A session that changed no card discards as before
    Given session "s1" moved, created and edited no card
    When the user chooses Discard in the merge dialog of session "s1"
    Then no card list is shown
    And the worktree is removed as before
