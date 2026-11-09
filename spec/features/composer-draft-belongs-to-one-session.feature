@done
@tui
@rust
@agent-view
@multi-session
@persistence
@input
@BUG-549
Feature: A composer draft belongs to one session and ends when it is submitted
  """
  codelet-fspec-tui only, new module app/composer_draft.rs. App state ComposerDrafts holds the session whose draft the shared composer currently holds (bound) and a save slot per session (generation counter, ordering mutex, settled generation). App::sync_composer_draft runs at the end of every App::dispatch, App::handle_event and App::handle_paste: when the focused session differs from the bound one, the composer text is recorded as the bound session's draft (dropped if its tab is gone) and the focused session's in-memory draft is loaded into the composer (empty when it has none); otherwise any composer change is recorded as the focused session's draft. Every focus route goes through it, including the ones that never handled the composer (attach, new or child session, board chip, closing a tab). Recording sets SessionContext.input_draft and schedules a save.
  Per-session ordered saves: each scheduled write takes the session's next generation. A debounced write (300ms, non-empty text) sleeps, then takes the session's ordering mutex and writes only while it is still the latest generation; an empty draft is written as None at once. Writes for one session reach the daemon one at a time in schedule order, so the last write wins, and sessions never share a slot. Tasks are parked on pending_tasks (the RPC-012 drain seam); tests use tokio paused time. Submitting (top of handle_input_submitted, before every early return, and handle_input_queued) ends the focused session's draft: in-memory empty plus an immediate None write that supersedes any pending save.
  Hydration (spawn_hydrate_pending_input, on every attach and new tab) reads get_pending_input under the session's ordering mutex and only when every scheduled write has settled; its result (Action::PendingInputHydrated carrying the request generation) applies only when no draft change was scheduled since. Action::SeedPendingInput (the merge-conflict seed) is a draft change like typing: in-memory draft, composer when bound, and a save. Action::PendingInputChanged keeps its payload, but the handler records the composer's current text, which is the truth for the bound session. Sessions whose tabs are gone have their pending saves superseded, so a closed or deleted session never receives a late save; /clear, local or from another client (SessionCleared), ends the draft. No daemon change (see answered question).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A draft belongs to one session: whenever focus moves by any route (Shift+Left/Right cycle, mux focus, board chip, attach from the board or the resume list, a new session, a spawned child session, closing the focused tab), the outgoing composer text becomes the outgoing session's draft and the composer then shows the incoming session's draft, or empty; never another session's text (decided by the supervisor for the product owner, 2026-11-06)
  #   2. Submitting from the composer (send, steer, queue, or a slash command, whether it goes to the model or is handled locally, typed or picked from the slash popup) ends that draft: the composer, the in-memory draft and the daemon draft are all empty, and no save already in flight can bring it back; a refused send keeps today's behaviour (the notice is shown) and restoring the text is out of scope
  #   3. Draft saves to the daemon are ordered per session so the last write wins, and activity in one session never cancels another session's pending save
  #   4. The daemon copy is the durable copy and the in-memory and daemon copies agree: hydration on attach never overwrites text typed after focusing, and never replaces a local draft the daemon has not stored yet
  #   5. Code that leaves editable text in the composer (history recall, search insert, slash popup fill, file popup splice, merge-conflict seed) updates the draft just like typing; the exec-stdin answer box never touches the draft; a captured HITL freeform answer ends the draft (RPC-411: the draft is the initial answer)
  #   6. /clear (local, or from another client), closing a tab and deleting a session remove that session's draft everywhere, and a save still pending for it never lands afterwards
  #   7. An unsubmitted draft still survives Shift+Left/Right cycling, board round-trips and TUI reattach (RPC-024, RPC-052, RPC-096 kept)
  #   8. Conservative option taken, keeping RPC-411: the composer text is the session's draft whichever prompt is showing, so half-typed answer text is kept like any draft while the prompt is open, and a captured answer (or Esc out of Other, which clears the input) ends the draft in memory and in the daemon. Esc-cancel leaves the remaining text as the draft, as RPC-411 specifies. The exec-stdin answer box has its own per-session input and never touches the draft.
  #
  # EXAMPLES:
  #   1. Typing 'deploy the fix' in s-1 and pressing Enter before the 300ms save fires: once the save window has passed the daemon draft, the in-memory draft and the composer are empty, and Shift+Right then Shift+Left shows an empty composer
  #   2. s-1 is running; typing 'also check the logs' and pressing Enter steers it; once the save window has passed the composer, in-memory and daemon drafts are empty
  #   3. s-1 is running; typing 'next task' and pressing Alt+Enter queues it; once the save window has passed all three copies are empty
  #   4. Typing '/hel' and picking Help in the slash popup opens the help dialog; once the save window has passed all three copies are empty
  #   5. s-1's composer holds 'half typed'; attaching to s-2 from the resume list (daemon draft empty) shows an empty composer; s-1's draft is 'half typed' in memory and in the daemon, and Shift+Left back to s-1 shows 'half typed'
  #   6. s-1's composer holds 'half typed'; a new session s-2 is created and focused: the composer is empty and s-1 keeps 'half typed'
  #   7. In mux mode s-1's pane is focused with 'half typed'; a child session s-2 is spawned: s-2's draft is empty and the s-1 composer still holds 'half typed'
  #   8. s-1 holds the draft 's1 draft'; s-2 is focused with 'throwaway'; closing s-2's tab shows 's1 draft' in the composer and no save of 'throwaway' lands afterwards
  #   9. On the board, s-1's composer holds 's1 text' and s-2's draft is 's2 text'; activating s-2's chip in the board menu bar shows 's2 text' and s-1 keeps 's1 text'
  #   10. Attaching to s-2 whose daemon draft is 'old draft' and typing 'new' before the daemon answers: the composer, in-memory and daemon drafts are all 'new'
  #   11. s-1 holds 'fresh text' whose save has not been written yet; re-attaching s-1 from the resume list while the daemon still has 'stale text' keeps 'fresh text' in the composer and the daemon ends with 'fresh text'
  #   12. Shift+Up recalls 'earlier prompt' into the composer; once the save window has passed the in-memory and daemon drafts are 'earlier prompt'
  #   13. Picking 'searched prompt' in the Ctrl+R history search puts it in the composer; once the save window has passed the in-memory and daemon drafts are 'searched prompt'
  #   14. A merge of s-1 reports conflicts; the conflict context seeded into s-1's composer is also s-1's in-memory and daemon draft
  #   15. s-1's composer holds 'chat draft' while a command waits for input; typing 'y' in the focused stdin answer box and sending it leaves 'chat draft' in the composer, in memory and in the daemon
  #   16. s-1's composer holds 'deploy the fix' when a freeform HITL question arrives; pressing Enter submits it as the answer and the composer, in-memory and daemon drafts are empty
  #   17. s-1 holds the draft 'scratch'; another client clears s-1: the composer, in-memory and daemon drafts are empty
  #   18. A draft 'keep me' typed in s-1 survives Shift+Right and Shift+Left, a trip to the board and back, and a fresh TUI attaching to s-1 shows 'keep me'
  #   19. In mux mode with two agent panes and s-1 focused: typing 'alpha', clicking s-2's pane within 300ms and typing 'beta' shows only 'beta' in the composer; once the save window has passed the daemon holds 'alpha' for s-1 and 'beta' for s-2
  #   20. Typing 'alpha' in s-1, pressing Shift+Right within 300ms and typing 'beta' in s-2: once the save window has passed the daemon holds 'alpha' for s-1 and 'beta' for s-2
  #   21. s-2 holds the draft 's2 draft' and s-1 is focused; opening the resume list with /resume and deleting s-1 there shows 's2 draft' in the composer
  #   22. Typing '/role reviewer' and pressing Enter sets the role locally without sending anything to the model; once the save window has passed all three copies are empty
  #
  # QUESTIONS (ANSWERED):
  #   Q: R5 says the HITL overlay must never leak its own text into the draft, but RPC-411 (inline-hitl-prompt.feature, TS parity) makes the freeform answer box the SHARED composer: the existing draft is the initial answer, and Esc-cancel leaves the typed text in the composer. Which wins?
  #   A: Conservative option taken, keeping RPC-411: the composer text is the session's draft whichever prompt is showing, so half-typed answer text is kept like any draft while the prompt is open, and a captured answer (or Esc out of Other, which clears the input) ends the draft in memory and in the daemon. Esc-cancel leaves the remaining text as the draft, as RPC-411 specifies. The exec-stdin answer box has its own per-session input and never touches the draft.
  #
  #   Q: should the daemon also drop the pending draft when it accepts a steer or queue submission (it already drops it when a submission starts a new turn)?
  #   A: No daemon change. The TUI's per-session ordered save queue makes the clear land after any earlier save, which is the fix. A daemon-side drop on steer or queue would also wipe a newer draft typed while a slow submit is in flight, so it would break the rule that the in-memory and daemon copies agree. The daemon keeps dropping the draft when a submission starts a new turn, as it does today.
  #
  # ========================================
  Background: User Story
    As a user typing prompts in the TUI agent views
    I want to have each session's unsent composer text kept as that session's draft, and gone for good once I submit it
    So that switching views or sessions never brings back a prompt I already sent, never shows another session's text, and never loses text I have not sent

  # ─────────────────────────────────────────────────────────────────────
  # Submitting ends the draft (rule 2)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Pressing Enter before the draft save fires leaves no draft anywhere
    Given session s-1 is focused and session s-2 is open
    And the user types "deploy the fix" into the composer
    When the user presses Enter before the 300ms draft save fires
    And the draft save window has passed
    Then the composer is empty
    And the in-memory draft of s-1 is empty
    And the daemon draft of s-1 is empty
    When the user presses Shift+Right and then Shift+Left
    Then the composer is empty

  Scenario: Steering a running turn ends the draft
    Given session s-1 is focused and its turn is running
    And the user types "also check the logs" into the composer
    When the user presses Enter
    And the draft save window has passed
    Then "also check the logs" is submitted to s-1 as a steer
    And the composer, the in-memory draft and the daemon draft of s-1 are empty

  Scenario: Queueing behind a running turn ends the draft
    Given session s-1 is focused and its turn is running
    And the user types "next task" into the composer
    When the user presses Alt+Enter
    And the draft save window has passed
    Then "next task" is submitted to s-1 as a queued input
    And the composer, the in-memory draft and the daemon draft of s-1 are empty

  Scenario: A slash command handled locally ends the draft
    Given session s-1 is focused
    And the user types "/role reviewer" into the composer
    When the user presses Enter
    And the draft save window has passed
    Then the role "reviewer" is set on s-1 and nothing is sent to the model
    And the composer, the in-memory draft and the daemon draft of s-1 are empty

  Scenario: A slash command picked from the popup ends the draft
    Given session s-1 is focused
    And the user types "/hel" into the composer
    When the user picks Help in the slash command popup
    And the draft save window has passed
    Then the help dialog opens
    And the composer, the in-memory draft and the daemon draft of s-1 are empty

  # ─────────────────────────────────────────────────────────────────────
  # Every focus route swaps the drafts (rule 1)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Attaching to another session from the resume list swaps the drafts
    Given session s-1 is focused
    And the user types "half typed" into the composer
    And the daemon holds no draft for session s-2
    When the user attaches to session s-2 from the resume list
    Then the composer is empty
    And the in-memory draft of s-1 is "half typed"
    When the draft save window has passed
    Then the daemon draft of s-1 is "half typed"
    When the user presses Shift+Left
    Then the composer shows "half typed"

  Scenario: A new session opens with an empty composer
    Given session s-1 is focused
    And the user types "half typed" into the composer
    When a new session s-2 is created and focused
    Then the composer is empty
    And the in-memory draft of s-1 is "half typed"
    And the in-memory draft of s-2 is empty

  Scenario: A child session spawned in mux mode does not take the focused draft
    Given mux mode shows the board and two agent panes
    And session s-1 is focused in an agent pane
    And the user types "half typed" into the composer
    When a child session s-2 is spawned
    Then the in-memory draft of s-2 is empty
    And the composer shows "half typed"
    And the in-memory draft of s-1 is "half typed"

  Scenario: Closing the focused tab shows the next session's own draft
    Given session s-1 holds the draft "s1 draft"
    And session s-2 is focused
    And the user types "throwaway" into the composer
    When the user closes the focused session's tab before the 300ms draft save fires
    And the draft save window has passed
    Then the composer shows "s1 draft"
    And no save of "throwaway" reaches the daemon

  Scenario: Activating another session's chip from the board swaps the drafts
    Given session s-2 holds the draft "s2 text"
    And session s-1 is focused
    And the user types "s1 text" into the composer
    And the user goes back to the board
    When the user activates the s-2 chip in the board menu bar
    Then the agent view shows s-2 with the composer holding "s2 text"
    And the in-memory draft of s-1 is "s1 text"

  Scenario: Clicking another mux agent pane swaps the drafts and keeps both saves
    Given mux mode shows the board and two agent panes for sessions s-1 and s-2
    And the s-1 pane is focused
    And the user types "alpha" into the composer
    When the user clicks the s-2 pane before the 300ms draft save fires
    And the user types "beta" into the composer
    And the draft save window has passed
    Then the composer shows "beta"
    And the daemon draft of s-1 is "alpha"
    And the daemon draft of s-2 is "beta"

  # ─────────────────────────────────────────────────────────────────────
  # Saves are ordered per session (rule 3)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Typing in another session never cancels a pending save
    Given session s-1 is focused and session s-2 is open
    And the user types "alpha" into the composer
    When the user presses Shift+Right before the 300ms draft save fires
    And the user types "beta" into the composer
    And the draft save window has passed
    Then the daemon draft of s-1 is "alpha"
    And the daemon draft of s-2 is "beta"

  # ─────────────────────────────────────────────────────────────────────
  # Hydration never overwrites newer local text (rule 4)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Text typed before the attach hydration answers is kept
    Given session s-1 is focused
    And the daemon holds the draft "old draft" for session s-2
    When the user attaches to session s-2 from the resume list
    And the user types "new" into the composer before the daemon answers
    And the draft save window has passed
    Then the composer shows "new"
    And the in-memory draft of s-2 is "new"
    And the daemon draft of s-2 is "new"

  Scenario: Re-attaching a session keeps a draft the daemon has not stored yet
    Given session s-1 is focused and session s-2 is open
    And the user types "fresh text" into the composer
    And another client has left the draft "stale text" for s-1 in the daemon
    When the user attaches to session s-1 from the resume list before the 300ms draft save fires
    And the draft save window has passed
    Then the composer shows "fresh text"
    And the in-memory draft of s-1 is "fresh text"
    And the daemon draft of s-1 is "fresh text"

  # ─────────────────────────────────────────────────────────────────────
  # Text put in the composer by code is a draft like typing (rule 5)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: A recalled history entry becomes the draft
    Given session s-1 is focused and its input history holds "earlier prompt"
    When the user presses Shift+Up
    And the draft save window has passed
    Then the composer shows "earlier prompt"
    And the in-memory draft of s-1 is "earlier prompt"
    And the daemon draft of s-1 is "earlier prompt"

  Scenario: Text picked in the history search becomes the draft
    Given session s-1 is focused
    When the user picks "searched prompt" in the Ctrl+R history search
    And the draft save window has passed
    Then the composer shows "searched prompt"
    And the in-memory draft of s-1 is "searched prompt"
    And the daemon draft of s-1 is "searched prompt"

  Scenario: The merge-conflict context is saved as the session's draft
    Given session s-1 is focused
    When a merge of s-1 reports conflicts in "src/lib.rs"
    And the draft save window has passed
    Then the composer holds the conflict context naming "src/lib.rs"
    And the in-memory draft and the daemon draft of s-1 hold the same conflict context

  Scenario: Answering a command's stdin prompt never touches the draft
    Given session s-1 is focused and its turn is running
    And the user types "chat draft" into the composer
    And a command run by s-1 may be waiting for input
    When the user opens the stdin answer box, types "y" and presses Enter
    And the draft save window has passed
    Then "y" is written to the command's stdin
    And the composer shows "chat draft"
    And the in-memory draft of s-1 is "chat draft"
    And the daemon draft of s-1 is "chat draft"

  Scenario: A captured HITL freeform answer ends the draft
    Given session s-1 is focused
    And the user types "deploy the fix" into the composer
    And a freeform HITL question is showing for s-1
    When the user presses Enter
    And the draft save window has passed
    Then "deploy the fix" is sent as the HITL answer
    And the composer, the in-memory draft and the daemon draft of s-1 are empty

  # ─────────────────────────────────────────────────────────────────────
  # Clear and delete remove the draft (rule 6)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: Another client clearing the session removes its draft everywhere
    Given session s-1 is focused
    And the user types "scratch" into the composer
    And the draft save window has passed
    When another client clears session s-1
    And the draft save window has passed
    Then the composer, the in-memory draft and the daemon draft of s-1 are empty

  Scenario: Deleting the focused session from the resume list shows the next session's draft
    Given session s-2 holds the draft "s2 draft"
    And session s-1 is focused
    When the user opens the resume list with "/resume"
    And the user deletes session s-1 there
    Then the composer shows "s2 draft"

  # ─────────────────────────────────────────────────────────────────────
  # An unsubmitted draft survives (rule 7)
  # ─────────────────────────────────────────────────────────────────────
  Scenario: An unsubmitted draft survives cycling, the board and a reattach
    Given session s-1 is focused and session s-2 is open
    And the user types "keep me" into the composer
    When the user presses Shift+Right and then Shift+Left
    Then the composer shows "keep me"
    When the user presses Shift+Left to go to the board and Shift+Right to come back
    Then the composer shows "keep me"
    When the draft save window has passed
    And a fresh TUI attaches to the same daemon and opens s-1
    Then the composer shows "keep me"
