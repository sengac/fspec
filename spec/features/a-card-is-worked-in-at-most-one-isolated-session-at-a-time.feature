@done
@session
@multi-session
@concurrency
@rust
@high
@WT-019
Feature: A card is worked in at most one isolated session at a time
  """
  Product owner decision 2026-11-07: a work unit can be claimed by at most one isolated session; a second isolated session trying to take the same card is refused, naming the session that holds it. A session's claim is its activity focus (BLOCK-006/WUA focus), so every focus change goes through SessionManager::apply_resolved_focus, which refuses an isolated session's move onto a card that another live isolated session of the same project focuses. Closing the holder, detaching it from its worktree (merge or discard) or moving its focus releases the card. Non-isolated sessions neither hold nor are refused a claim.
  Tests: rust/sessions/tests/wt019_one_isolated_session_per_card.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. An isolated session focusing (by set-focus, tool attribution or a status change) a card that another live isolated session of the same project already focuses is refused with an error naming the card and the session holding it; the refused session's focus is unchanged (product owner, 2026-11-07)
  #   2. The claim is the holder's focus: it is released when the holder focuses another card or none, is closed, or is detached from its worktree (merge or discard)
  #   3. Non-isolated sessions neither hold nor are refused a claim
  #   4. Claims are per project: isolated sessions of different projects never conflict
  #   5. Checking and taking a claim is atomic, so two isolated sessions focusing the same card at once cannot both win
  #
  # EXAMPLES:
  #   1. Isolated session A focuses STORY-001; isolated session B setting focus to STORY-001 fails naming STORY-001 and A's session id, and B's focus stays empty
  #   2. A focuses STORY-001; A is closed; B can then focus STORY-001
  #   3. A focuses STORY-001; A is detached from its worktree after a merge; B can then focus STORY-001
  #   4. A focuses STORY-001; a non-isolated session focuses STORY-001 without error
  #   5. A focuses STORY-001 then STORY-002; B can then focus STORY-001
  #
  # ========================================
  Background: User Story
    As a developer running several isolated sessions on one project
    I want to have a card worked in at most one isolated session at a time
    So that two worktrees never write competing features and coverage for the same card that only collide at merge

  Scenario: A second isolated session cannot take a card another isolated session holds
    Given a temp git repo whose board has STORY-001 and STORY-002, and isolated sessions A and B on it
    And A focuses STORY-001
    When B sets its focus to STORY-001
    Then it fails naming STORY-001 and session A
    And B has no focus

  Scenario: Moving, closing or detaching the holder releases the card
    Given a temp git repo whose board has STORY-001 and STORY-002, and isolated sessions A and B on it
    When A focuses STORY-001 and then STORY-002
    Then B can focus STORY-001
    When B is detached from its worktree
    Then A can focus STORY-001
    When A is closed
    Then a new isolated session C can focus STORY-001

  Scenario: Non-isolated sessions are unaffected
    Given a temp git repo whose board has STORY-001 and STORY-002, an isolated session A and a non-isolated session N on it
    And A focuses STORY-001
    When N sets its focus to STORY-001
    Then N focuses STORY-001
