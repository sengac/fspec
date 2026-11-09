@done
@external-bridge
@rust
@bridge
@BRIDGE-022
Feature: The bridge can answer questions, not only approvals
  """
  No rpc-types or RPC wire change. agent-loop agent_loop/bridge.rs adds request (the questions) to the forwarded JSON of the session's own Pending question event; tools bridge_multiplexed::route_inbound keeps an object data.response as JSON text; agent-loop bridge_control::control_handler answers question_response with answer_question(request_id, RequestHolder::User, HitlResponse).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A session's bridge event for a question it raised (interactionRequestChanged, status Pending, kind Question, origin this session) carries the question text: 'request' holds the questions with their ids, headers and options
  #   2. The bridge control action question_response answers a question: data.request_id names it and data.response is the HitlResponse ({answers: {<question id>: {selected, other}}} or {cancelled: true}); it answers as the user, so it is accepted whoever holds the question (AMGR-019 Q2) and the holder is told
  #   3. A question_response without a request id, naming another or a stale request, naming an approval, or whose response does not parse is rejected and answers nothing; pause_response still answers only a three-choice approval (BUG-631)
  #
  # EXAMPLES:
  #   1. Session S asks 'Which auth library?' with options jose and jsonwebtoken; S's bridge event is interactionRequestChanged Pending with request.questions[0].question 'Which auth library?' and both option labels
  #   2. The dashboard sends question_response with S's request id and answers auth_library selected jose; S's tool receives jose
  #   3. A question_response naming an old request id, or none, or S's approval's id, is rejected; S's question stays pending
  #   4. A question held by S's parent is answered from the bridge; the parent is told the user answered it
  #
  # ========================================
  Background: User Story
    As a user on the remote bridge dashboard
    I want to see a session's question and answer it from the dashboard
    So that a question raised while I am away from the terminal does not wait until I am back at it

  Scenario: A session's question event on the bridge carries the question text
    Given a session with a bridge asks "Which auth library?" with options jose and jsonwebtoken
    When the bridge forwards its interactionRequestChanged Pending event
    Then the event's request holds the question "Which auth library?" with the option labels jose and jsonwebtoken

  Scenario: question_response answers the question named by its request id
    Given a session waiting on its question
    When the bridge sends question_response with that request id and auth_library selected jose
    Then the question's wait returns the answer jose

  Scenario: A question_response with a wrong, missing or approval id answers nothing
    Given a session waiting on its question
    When the bridge sends question_response with no request id, then a stale id, then the id of the session's approval, then an unparsable response
    Then the question is still pending with the same request id
    And cancelling it then returns cancelled as its first answer

  Scenario: The bridge answers a question its parent holds and the parent is told
    Given a sub-agent's question held by its parent session
    When the bridge sends question_response for it
    Then the sub-agent's tool receives the answer
    And the parent is told that the user answered it
