@done
@rust
@pause-integration
@agent-view
@tui
@AMGR-022
Feature: The root view shows a sub-agent's request headed with who asks, and every view on the chain shows a waiting marker
  """
  fspec-tui only, no wire change. store/agent_view/request_chain.rs keeps the pending requests seen in InteractionRequestChanged events (one copy arrives per chain member, so updates are idempotent by request id), names who asks and finds each request's root view. app/dispatch_request_chain.rs folds the events: a Pending event held by the user whose origin has no prompt read for that id reads it (handle_pause_chunk on the origin); Answered or Cancelled clears the origin's prompt slot. views/agent/relayed_prompt.rs paints the who-asks header plus the origin's HITL or pause prompt in the root view and caches last_hitl / last_pause with the ORIGIN session, so the existing key routing replies to the origin with the stored request id; it also paints the waiting row. The agent-bar chip marker is added in views/board/menu_snapshot.rs after build_chips.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A request held by the user whose origin is a sub-agent is drawn as the inline question or approval prompt in its root view: the highest session of its chain that is open in the TUI, other than the origin. A header row above it names who is asking, from the origin up to the root, e.g. 'sub-agent #2 of bot #1 asks' (product owner, 2026-11-07, decision 3; AMGR-019 rule 3)
  #   2. Answering the prompt in the root view sends the checked reply to the origin session naming the request's id; the user can still answer from the sub-agent's own tab, which works as before (product owner, 2026-11-07 ~21:40, Q2)
  #   3. While a request is pending, every view on its chain above the origin shows a waiting row naming who asks, what kind of request it is and who holds it, and that view's agent-bar chip shows the waiting marker '?' after its status glyph, in yellow (product owner, 2026-11-07, decision 3)
  #   4. The prompt and the markers follow the InteractionRequestChanged events: a Pending event (raised, escalated or passed up) records the request's current holder; when the holder is the user the root view shows the prompt, reading the origin's question or approval if it has not been read yet; an Answered or Cancelled event removes the prompt from the root view and the origin's tab and clears the markers
  #   5. A session's own question or approval is shown before a relayed one; with several relayed requests the oldest is shown first
  #   6. Sessions are named by their TUI slot: the root is 'bot #N' for a bot and 'agent #N' otherwise, every other chain member is 'sub-agent #N'; a session that is not open in the TUI is named by the first 8 characters of its id
  #
  # EXAMPLES:
  #   1. Bot #1 spawned sub-agent #2; #2's question 'Which auth library?' is escalated to the user; bot #1's view shows 'sub-agent #2 of bot #1 asks' above the question; the user picks jose there; send_hitl_response goes to #2 naming the request id
  #   2. Agent #1 holds sub-agent #2's approval; agent #1's view shows 'Waiting: sub-agent #2 asks for approval; agent #1 holds it' and no prompt; agent #1's chip reads '#1 ●?'
  #   3. Bot #1 spawned #2 which spawned #3; #3's approval is passed up to the user; bot #1's view shows 'sub-agent #3 of sub-agent #2 of bot #1 asks' above the approval; #2's view shows 'Waiting: sub-agent #3 asks for approval; the user holds it'; Enter in bot #1's view sends pause_triple approve to #3 naming the request id
  #   4. Agent #1 answers sub-agent #2's request; the Answered event arrives; no prompt, waiting row or chip marker is left in any view
  #   5. The user answers sub-agent #2's escalated question from #2's own tab; the reply goes to #2 naming the request id; when the Answered event arrives bot #1's view no longer shows it
  #   6. Bot #1 has its own approval pending and sub-agent #2's question is held by the user; bot #1's view shows its own approval; once that is answered it shows #2's question
  #
  # ASSUMPTIONS:
  #   1. Who-is-asking names use the TUI's session slot numbers (#N, the chips and SessionHeader convention) and bot/agent from the TUI-114 role overlay, because the TUI has no session names; derived, not decided by the product owner
  #   2. While a session holds the request, the root view shows the waiting row rather than the prompt (the card: the prompt is drawn when the user holds it); the user takes a session-held request over from the sub-agent's own tab (Q2)
  #
  # ========================================
  Background: User Story
    As a user running agents and bots in the TUI
    I want to see and answer a sub-agent's question or approval in the view of the agent or bot at the top of its chain, headed with who is asking, and see which views wait on one
    So that a sub-agent's request that reaches me is never hidden in a tab I am not looking at

  Scenario: The root view shows a question escalated to the user, headed with who asks, and answers it
    Given bot #1 spawned sub-agent #2 and #2's question "Which auth library?" is escalated to the user
    When the user looks at bot #1's view
    Then it shows "sub-agent #2 of bot #1 asks" above the question
    When the user picks jose there
    Then send_hitl_response goes to sub-agent #2 naming the request id

  Scenario: A view whose session holds a sub-agent's approval shows a waiting row and its chip a waiting marker
    Given agent #1 holds sub-agent #2's approval
    When the user looks at agent #1's view
    Then it shows "Waiting: sub-agent #2 asks for approval; agent #1 holds it" and no prompt
    And agent #1's chip reads "#1 ●?" with the marker in yellow

  Scenario: An approval passed up a three-level chain to the user shows in the root view and is answered there
    Given bot #1 spawned sub-agent #2, #2 spawned sub-agent #3, and #3's approval is passed up to the user
    When the user looks at bot #1's view and at #2's view
    Then bot #1's view shows "sub-agent #3 of sub-agent #2 of bot #1 asks" above the approval
    And #2's view shows "Waiting: sub-agent #3 asks for approval; the user holds it"
    When the user presses Enter in bot #1's view
    Then pause_triple approve goes to sub-agent #3 naming the request id

  Scenario: An answered request leaves no prompt, waiting row or marker
    Given agent #1's view shows a waiting row for sub-agent #2's request
    When the Answered event for that request arrives
    Then no view shows a prompt or a waiting row for it and no chip shows the waiting marker

  Scenario: The user answers an escalated question from the sub-agent's own tab
    Given sub-agent #2's question is escalated to the user and bot #1's view shows it
    When the user answers it from #2's own tab
    Then the reply goes to sub-agent #2 naming the request id
    And once the Answered event arrives bot #1's view no longer shows it

  Scenario: The root's own approval is shown before a relayed question
    Given bot #1 has its own approval pending and sub-agent #2's question is held by the user
    When the user looks at bot #1's view
    Then it shows bot #1's own approval
    When the user answers that approval
    Then bot #1's view shows sub-agent #2's question headed "sub-agent #2 of bot #1 asks"
