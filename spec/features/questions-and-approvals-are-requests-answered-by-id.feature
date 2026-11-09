@done
@pause-integration
@rpc
@session
@rust
@AMGR-020
Feature: Each question and approval is a request with an id, an origin and a parent chain, answered only by its id
  """
  Design (2026-11-07): rpc-types gains RequestRoute {request_id, origin, kind, chain, holder} and PendingRequest<T> {route, payload}; get_pause_state/get_hitl_request return PendingRequest<PauseState>/PendingRequest<HitlRequest>; pause_resume/pause_confirm/pause_triple/send_hitl_response take the request id; StreamChunk::InteractionRequestChanged {route, status: Pending|Answered{by}|Cancelled} is sent on every session of the chain. codelet-sessions: InteractionSlot keeps the route with the id; BackgroundSession computes the chain from its owning manager's first supervisor links when a request is raised; every reply goes through one checked call taking a Responder (User | Session) that checks id, kind and holder. reply_interaction_checked and get_interaction_snapshot are removed (client MCP passes the id itself). The method names stay so the done RPC-037/RPC-406/RPC-411 features keep naming them.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every question and approval a session raises is a request with an id, its origin session, its kind (question, approval or human action) and its parent chain from the origin to the root (origin, its parent, ... the root), read from the spawner links when it is raised (product owner, 2026-11-07, AMGR-019 rule 1)
  #   2. A new request is held by the origin's parent; a session with no parent sends it straight to the user (product owner, 2026-11-07, decisions 1 and 2)
  #   3. A human action (WebSearch 'interact with the page, then press Enter', /goal 'human review needed') is always held by the user: only a person can do it, so no parent may continue it
  #   4. get_hitl_request and get_pause_state return the pending request with its id, origin, kind, chain and holder
  #   5. Every session on the chain receives a stream event when the request is raised and when it is answered (naming who answered) or cancelled
  #   6. Every reply names the request id: pause_resume, pause_confirm, pause_triple and send_hitl_response (RPC, TUI websocket and embedded transports, client MCP) and the bridge pause_response; a reply whose id is not the pending request's id (answered, replaced or cancelled) is rejected and answers nothing
  #   7. A reply is accepted from the request's current holder or from the user; any other session is rejected (product owner, 2026-11-07 ~21:40, Q2: the user can always answer, taking the request over); every RPC, TUI, bridge and client MCP reply is the user's
  #   8. A reply of the wrong kind (an approval choice for a question, an answer for an approval) is rejected
  #   9. There is no unchecked reply path: the per-session unchecked replies, the separate snapshot-then-reply client MCP path and the bridge pause_response without an id are gone
  #
  # EXAMPLES:
  #   1. Bot R spawned sub-agent A; A asks 'Which auth library?'; the request has kind question, origin A, chain [A, R] and holder R
  #   2. A session with no parent asks a question; the request's chain is [the session] and its holder is the user
  #   3. R spawned A and A spawned B; B's Bash command is held by the blocklist; the request has kind approval, chain [B, A, R] and holder A
  #   4. Sub-agent A's WebSearch asks the user to interact with the page and press Enter; the request has kind human action and holder the user
  #   5. A's question is held by R; R and A each receive a request event with that id; the user answers it with send_hitl_response naming the id; A's tool gets the answer and R and A each receive an event saying the user answered it
  #   6. A's question is held by R; sibling sub-agent S replies to it; the reply is rejected as not from the holder and A's question stays pending
  #   7. A asked a question and then asked a new one; a reply naming the first question's id is rejected and the new question stays pending
  #   8. A waits on a blocklist approval; pause_triple names that id but the reply is sent as a question answer; it is rejected and the approval stays pending
  #   9. A bridge pause_response for A's approval without a request id is rejected and the approval stays pending; with the id it answers it
  #   10. A client MCP pause_triple call without a request_id fails schema validation; with the id from get_pause_state it answers the approval
  #
  # ========================================
  Background: User Story
    As a user whose agents spawn sub-agents
    I want to have every question and approval be a request with an id, an origin and a parent chain that only its holder or I can answer, by its id
    So that an answer never lands on a request it was not meant for and the parent chain can route it

  Scenario: A sub-agent's question is a request held by its parent
    Given bot R spawned sub-agent A
    When A asks the question "Which auth library?"
    Then get_hitl_request for A returns a request of kind question with origin A, chain [A, R] and holder R

  Scenario: A session with no parent sends its question to the user
    Given a session with no parent
    When it asks a question
    Then the request's chain is that session alone and its holder is the user

  Scenario: A nested sub-agent's approval is held by its own parent
    Given R spawned A and A spawned B
    When the blocklist holds B's Bash command for approval
    Then get_pause_state for B returns a request of kind approval with origin B, chain [B, A, R] and holder A

  Scenario: A human action is always held by the user
    Given bot R spawned sub-agent A
    When A's WebSearch asks for the page to be used and Enter pressed
    Then the request has kind human action and its holder is the user

  Scenario: The user answers a sub-agent's question by its id and every session on the chain is told
    Given A's question is held by R
    When the user answers it with send_hitl_response naming its id
    Then A's tool receives the answer, and A and R each received an event for that id when it was raised and one saying the user answered it

  Scenario: The holder answers its sub-agent's approval
    Given A's approval is held by R
    When R replies Deny naming its id
    Then A's approval is answered Deny, and A and R each received an event saying R answered it

  Scenario: A reply from a session that is not the holder is rejected
    Given A's question is held by R
    When A's sibling sub-agent S replies to it naming its id
    Then the reply is rejected as not from the holder and the question stays pending

  Scenario: A reply naming a replaced request is rejected
    Given A asked a question and then a new one
    When a reply names the first question's id
    Then the reply is rejected and the new question stays pending

  Scenario: A reply of the wrong kind is rejected
    Given A waits on a blocklist approval
    When a question answer is sent naming the approval's id
    Then the reply is rejected and the approval stays pending

  Scenario: The TUI answers a prompt with its request id
    Given the TUI shows a session's approval prompt and question prompt read with get_pause_state and get_hitl_request
    When the user approves the approval and submits the question
    Then the backend receives pause_triple and send_hitl_response naming that session and each request's id

  Scenario: A bridge pause_response must name the request id
    Given a session waits on a blocklist approval
    When the bridge sends pause_response for it without a request id and then with the approval's id
    Then the first is rejected leaving the approval pending and the second answers it

  Scenario: Client MCP replies must name the request id
    Given a session waits on a blocklist approval
    When a client MCP pause_triple call omits request_id and then names the id from get_pause_state
    Then the first fails schema validation and the second answers the approval

  Scenario: No unchecked reply path is left
    Given the daemon, RPC, TUI, bridge and client MCP sources
    When they are searched for the ways to answer a question or approval
    Then every one takes a request id, and the unchecked session replies and the client MCP snapshot-then-reply wrapper are gone
