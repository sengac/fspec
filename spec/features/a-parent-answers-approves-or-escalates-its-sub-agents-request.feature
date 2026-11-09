@done
@agent-manager
@session
@rust
@AMGR-021
Feature: A parent answers, approves or escalates its sub-agent's request through AgentManager
  """
  codelet-sessions: Raised.holder becomes mutable through InteractionSlot::update (id-checked, under the slot lock). The holder is the first live session above the origin on its chain (the user if none, or for a human action). Before a request is announced, its new holder is told: a holder inside await_idle on a chain member (BackgroundSession await registry) learns it from the InteractionRequestChanged event; any other holder gets a supervisor-input message (receive_incoming_message) that starts its next turn. escalate_request moves the holder from the caller to the next live session above it (or the user); interrupt() (also called by destroy_session) passes every request the session holds up the same way; a user answer to a session-held request tells that holder by message. A human action's parent is told once (a told set on the parent). SessionManager::pending_requests lists every live session's pending request. codelet-tools: AgentManagerAction gains Answer, Approve and Escalate; AwaitOutcome gains NeedsInput, WaitingOnUser and Pending; AwaitSessionResult, SessionEntry and SessionStatus carry the request (id, origin, kind, holder and what it asks). codelet-agent-loop: await_idle registers the await, scans pending requests and watches request events; the new actions find the request by id and reply through the checked answer_question / answer_pause / escalate_request. sessions bot.rs role text names answer, approve and escalate. No rpc-types or RPC change.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AgentManager await_idle returns at once when an awaited session, or a session below it, has a request held by the caller: that session's status is needs_input with the request id, the origin sub-agent, the kind and what it asks (the questions with their options, or the tool, message and details of the approval); every other awaited session that has not finished is reported pending; the caller can call await_idle again for the rest (product owner, 2026-11-07, decision 5)
  #   2. AgentManager answer {request_id, answers | cancelled} answers a question the caller holds; approve {request_id, choice approve | approve_session | deny} answers an approval the caller holds (a two-choice confirm takes approve or deny); escalate {request_id} moves the request to the next live session above the caller on its chain, or to the user at the root; each is checked against the request id, its kind and its holder, and a rejected one answers or moves nothing (product owner, 2026-11-07, decisions 1 and 2; AMGR-019 rule 9)
  #   3. A human action (WebSearch 'interact with the page', /goal 'human review needed') is always held by the user: no parent can answer, approve or escalate it; the parent is told once that its sub-agent waits on the user (product owner, 2026-11-08)
  #   4. A holder that is interrupted or closed passes every request it holds to the next live session above it on the request's chain, ending with the user; closing a bot still closes its sub-agents, which cancels their requests (BUG-605) (product owner, 2026-11-07, decision 6)
  #   5. A session that comes to hold a request (raised, escalated or passed up to it) while it is not inside await_idle on a session of that request's chain is told by a message that starts its next turn (at once if it is idle, right after its current turn if it is busy); it still holds the request (product owner, 2026-11-07 ~21:40, Q1)
  #   6. The user can always answer a request directly, taking it over from its holder; a holder session is then told by a message that the user answered it and how (product owner, 2026-11-07 ~21:40, Q2)
  #   7. A request is never dropped: a chain member that is not a live session (closed, or its spawner link lost after a daemon restart, BUG-587) is skipped, and with no live session above the origin the request is held by the user at the root
  #   8. AgentManager list and get_status show each session's pending request: its id, kind, origin and holder
  #   9. The bot role text tells bots to answer, approve or escalate a sub-agent's request with AgentManager answer, approve or escalate when await_idle reports needs_input, and never to message a paused sub-agent; bot and agent parents follow the same rules (product owner, 2026-11-07, decision 8)
  #   10. There is no deadline: a request waits until it is answered or cancelled; an await_idle timeout or interrupt that ends while an awaited session has a request held by the caller reports that session needs_input, not timed_out (product owner, 2026-11-07, decision 4)
  #
  # EXAMPLES:
  #   1. Bot R awaits sub-agent A with no timeout; A asks 'Which auth library?' with options jose and jsonwebtoken; await_idle returns at once with A needs_input naming the request id and the question; R calls answer selecting jose; A's tool receives jose
  #   2. R awaits A and S; A's Bash command is held by the blocklist; await_idle returns A needs_input with kind approval and the command, and S pending; R calls approve with choice deny; A's command is denied
  #   3. R spawned A and A spawned B; R awaits A; B asks a question held by A; R's await_idle does not return for it; A escalates it; R's await_idle returns A needs_input with origin B
  #   4. R is the root and holds A's question; R escalates it; the user holds it and R's next answer naming it is rejected as not from the holder
  #   5. A's sibling S calls answer for A's question held by R; it is rejected and the question stays pending
  #   6. R calls approve for A's question; it is rejected as the wrong kind; R calls answer with a stale request id; it is rejected
  #   7. A's WebSearch asks the user to interact with the page; R's await_idle returns A waiting_on_user once; R's approve naming it is rejected; R's next await_idle keeps waiting until A is idle
  #   8. R spawned A and A spawned B; A holds B's question; A is interrupted; R now holds it and is told; A is closed instead: R holds it
  #   9. Root agent A holds sub-agent B's approval and is interrupted; the user holds it
  #   10. R is idle when A asks a question; R gets a message naming the request that starts its next turn; R answers it with AgentManager answer
  #   11. R holds A's question; the user answers it from A's tab; R is told by a message that the user answered it with jose
  #   12. A's spawner link was lost in a daemon restart; A asks a question; the user holds it
  #   13. R calls get_status for A while A waits on a question R holds; the status shows the request id, kind question, origin A and holder R
  #   14. The bot role text names AgentManager answer, approve and escalate and no longer tells the bot to answer a sub-agent with message
  #   15. R awaits A with timeout 1; A asks a question held by R before the timeout; the result is A needs_input, not timed_out
  #
  # ASSUMPTIONS:
  #   1. A human action's parent is told once: the first await_idle that sees it returns waiting_on_user for that session (a parent that is not waiting gets one message), and later await_idle calls keep waiting, so a parent never spins on a request only the user can resolve. Derived from the product owner's 2026-11-08 answer ('the parent is told the sub-agent waits on the user'), not decided as such.
  #   2. Telling a session (a new holder, or the holder the user took a request over from) is a supervisor-input message from the origin sub-agent, queued with receive_incoming_message: it starts the next turn at once if the session is idle, or right after the current turn.
  #
  # ========================================
  Background: User Story
    As a parent agent or bot that started a sub-agent
    I want to be told when a sub-agent below me needs my answer or approval, and answer, approve or escalate it through AgentManager
    So that a sub-agent's question or approval never leaves me or the user hanging, and the user can still step in at any time

  Scenario: await_idle reports a sub-agent's question and the parent answers it
    Given bot "R" spawned sub-agent "A" and is awaiting it with no timeout
    When "A" asks "Which auth library?" with options "jose" and "jsonwebtoken"
    Then R's await_idle returns at once with "A" needs_input naming the request id, origin "A", kind question and the question with its options
    When R calls AgentManager answer with that request id selecting "jose"
    Then A's tool receives the answer "jose"

  Scenario: await_idle reports an approval and the rest as pending, and the parent denies it
    Given bot "R" spawned sub-agents "A" and "S", "S" is running and R awaits both
    When the blocklist holds A's Bash command "echo amgr021 git reset --hard" for approval
    Then R's await_idle returns "A" needs_input with kind approval, the tool, message and command, and "S" pending
    When R calls AgentManager approve with that request id and choice deny
    Then A's command is denied

  Scenario: A request below an awaited sub-agent reaches the parent when it is escalated to it
    Given bot "R" spawned "A", "A" spawned "B", and R awaits "A"
    When "B" asks a question, which "A" holds
    Then R's await_idle has not returned when "A" escalates the question
    And R's await_idle then returns "A" needs_input for that request with origin "B" and holder R

  Scenario: Escalating at the root hands the request to the user
    Given root bot "R" holds its sub-agent "A"'s question
    When R calls AgentManager escalate with its request id
    Then the request's holder is the user and every session on its chain is told
    And R's AgentManager answer naming it is then rejected as not from the holder

  Scenario: Replies that are not the holder's, of the wrong kind or stale are rejected
    Given bot "R" holds sub-agent "A"'s question and "S" is A's sibling
    When S calls answer for it, R calls approve for it and R calls answer naming an unknown request id
    Then each call is rejected and A's question is still pending with holder R

  Scenario: A human action is held by the user and the parent is told once
    Given bot "R" spawned sub-agent "A" and is awaiting it
    When A's WebSearch asks for the page to be used and Enter pressed
    Then R's await_idle returns "A" waiting_on_user with the request id and holder the user
    And R's approve naming it is rejected
    And R's next await_idle on "A" with timeout 1 times out instead of reporting it again

  Scenario: An interrupted or closed holder passes its requests up the chain
    Given bot "R" spawned "A", "A" spawned "B", and "A" holds "B"'s question
    When "A" is interrupted
    Then R holds the question and is told by a message naming the request
    When R is closed
    Then the user holds the question

  Scenario: A parent that is not waiting is told by a message and still holds the request
    Given bot "R" spawned sub-agent "A" and R is idle, not in await_idle
    When "A" asks "Which auth library?"
    Then R receives a message from "A" that starts its next turn, naming the request id, the question and the AgentManager answer and escalate actions
    And R still holds the request

  Scenario: The user takes a request over and the holder is told
    Given bot "R" holds sub-agent "A"'s question "Which auth library?"
    When the user answers it "jose" from A's tab
    Then A's tool receives "jose"
    And R receives a message saying the user answered the request with "jose"

  Scenario: A sub-agent with no live parent sends its request to the user
    Given session "A" whose spawner link is gone, as after a daemon restart
    When "A" asks a question
    Then the user holds the request

  Scenario: get_status and list show a session's pending request
    Given bot "R" holds sub-agent "A"'s question
    When R calls AgentManager get_status for "A" and list
    Then both show A's pending request with its id, kind question, origin "A" and holder R

  Scenario: An await_idle started after the request was raised reports needs_input, not timed_out
    Given bot "R" holds sub-agent "A"'s question, raised while R was not in await_idle
    When R calls await_idle on "A" with timeout 1
    Then the result is "A" needs_input for that request, not timed_out

  Scenario: The bot role answers, approves or escalates instead of messaging a paused sub-agent
    Given the orchestrator bot role text
    When a new bot is created with it
    Then it tells the bot to use AgentManager answer, approve or escalate when await_idle reports needs_input
    And it no longer tells the bot to answer a sub-agent with AgentManager message
