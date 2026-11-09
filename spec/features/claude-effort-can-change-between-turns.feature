@done
@model-selection
@rust
@thinking-level
@providers
@high
@MODEL-015
Feature: Claude effort can change between turns
  """
  Plug-in points (research attachment): the effort message needs a new history entry kind, because rig's Message has only User/Assistant and non-empty content (patches/rig-core/src/completion/message.rs:36-45; anthropic completion.rs:195-198); the request body builder must emit it inside messages (patches/rig-core/src/providers/anthropic/request_body.rs:43-48); the beta header is fixed per client today (providers/src/claude.rs:59-112) and needs a per-request form; persistence (StoredMessage role user|assistant, core/src/persistence/messages.rs:59-77; sessions/src/replay_content.rs:95-97) and compaction replay (cli/src/session/compaction.rs:160-208) must carry or re-declare it; the session must remember the conversation's starting level (only the current level is kept today, sessions/src/background_session.rs:140). Other providers (Gemini, Codex) are out of scope: their level change is a request parameter with no cache-keeping form.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Per-turn Claude effort changes are in scope (product owner, 2026-11-04, decision 5): a session's thinking level may change between turns of one conversation through the thinking dialog or /thinking (decision 8), and the next turn runs at the new effort
  #   2. On a Claude model that takes per-message effort (Fable 5.1, Mythos 5.1, Opus 5.5, Opus 5, Sonnet 5.5, Haiku 5.5) with thinking on, an effort change is sent as an effort-only message ({role: system, content: [], output_config: {effort}}) appended to the conversation where the change happens, with the beta header mid-conversation-output-config-2026-07-01, and the request's top-level effort stays the level the conversation started with, so the prompt cache is kept
  #   3. An effort message is part of the conversation history: it is persisted, replayed on resume and sent on every later request at its position, never removed or moved (Claude's preserved-thinking check rejects edited history)
  #   4. Which models take per-message effort comes from an explicit fspec table naming those models, never from model-id patterns or guesses; a model not in it changes effort at the top level
  #   5. When thinking is off (Off on a model with an off form), the level change is a thinking change, not an effort change, and goes at the top level; a per-message effort message is never sent with thinking off (the API refuses it)
  #   6. After compaction, the level in effect is re-declared at the top level of the next request (an effort set inside summarized messages is not carried over by the API)
  #   7. Nothing is remapped: the effort sent is the level chosen, checked against the model's capability as today (MODEL-009/010)
  #   8. On a Claude model without per-message effort, a mid-conversation level change is applied at the top level as it is today, with no notice and no refusal (product owner, 2026-11-06, option a)
  #   9. A level change applies from the user's next message only: the effort message goes right before the new user message; a change made while a turn or tool loop is running does not affect that running turn (product owner, 2026-11-06)
  #   10. The per-message effort beta header is sent on Claude subscription (OAuth) requests too, the same as API-key requests (product owner, 2026-11-06)
  #
  # EXAMPLES:
  #   1. A conversation on anthropic/claude-opus-5-5 started at high; the user picks low with /thinking and sends the next message: the request keeps top-level effort high, carries the beta header, and has {role: system, content: [], output_config: {effort: low}} just before the new user message; the cached prefix is read, not rewritten
  #   2. Later in that conversation the user picks xhigh: a second effort message is appended before that turn's user message; the first one stays where it was in every later request
  #   3. The session is resumed: the replayed history still holds both effort messages at their positions, and the next request sends them unchanged
  #   4. A conversation on anthropic/claude-sonnet-5-5 at Off (between_tools) switches to medium: thinking changes, so the request's top-level thinking and effort change and no effort message is sent
  #
  # QUESTIONS (ANSWERED):
  #   Q: On a Claude model that does not take per-message effort (Opus 4.8/4.7/4.6, Sonnet 5, Sonnet 4.6, Opus 4.5, Fable 5), changing the level mid-conversation changes the top-level effort, which makes Claude re-read the whole conversation uncached on the next turn (it costs more). Should fspec (a) just do it, (b) do it and tell the user once, in the session, that the change drops the prompt cache, or (c) refuse a mid-conversation change on those models (the level then only applies to a new conversation)?
  #   A: On a Claude model without per-message effort, a mid-conversation level change is applied at the top level as it is today, with no notice and no refusal (product owner, 2026-11-06, option a)
  #
  #   Q: Where does a per-message effort change take effect? The API applies an effort message placed after the user's message to the reply to that message, and one placed earlier from the next user message on. fspec only sends a request when the user sends a message, so the natural place is just before the new user message (the new level applies to that reply). Is that right, or should a change made while a turn is running (or in a tool loop) also apply to that running turn's next request?
  #   A: A level change applies from the user's next message only: the effort message goes right before the new user message; a change made while a turn or tool loop is running does not affect that running turn (product owner, 2026-11-06)
  #
  #   Q: The beta header is added only to requests on models that take per-message effort, and only once a conversation has an effort message; Claude via the Claude Code subscription (OAuth) sends the same Messages API. Is it acceptable to send this beta header on subscription (OAuth) requests too, or only on API-key requests (fspec already sends claude-code-20250219 and oauth-2025-04-20 betas on OAuth)?
  #   A: The per-message effort beta header is sent on Claude subscription (OAuth) requests too, the same as API-key requests (product owner, 2026-11-06)
  #
  # ========================================
  Background: User Story
    As a user of a Claude model
    I want to change the thinking effort between turns of one conversation
    So that a hard turn gets more effort and routine turns less, without losing the prompt cache where Claude allows it

  Scenario: An effort change on a model that takes per-message effort keeps the cache
    Given a conversation on anthropic/claude-opus-5-5 that started at effort high
    When the user chooses low and sends the next message
    Then the request keeps top-level effort high and carries the beta header mid-conversation-output-config-2026-07-01
    And an effort-only system message with effort low sits right before the new user message

  Scenario: Earlier effort messages stay in place
    Given that conversation already changed to low before an earlier user message
    When the user chooses xhigh and sends the next message
    Then the request still holds the low effort message at its position
    And a new xhigh effort message sits right before the new user message

  Scenario: Effort messages survive a resume
    Given a conversation on anthropic/claude-opus-5-5 that changed to low before an earlier user message
    When the session is restored and the user sends the next message
    Then the low effort message is sent at its position unchanged

  Scenario: A model without per-message effort changes the top-level effort
    Given a conversation on anthropic/claude-opus-4-8 that started at effort high
    When the user chooses low and sends the next message
    Then the request's top-level effort is low
    And it carries no effort message and no per-message effort beta header

  Scenario: A thinking change goes at the top level
    Given a conversation on anthropic/claude-sonnet-5-5 that started with thinking off
    When the user chooses medium and sends the next message
    Then the request's top-level thinking is adaptive with effort medium
    And it carries no effort message

  Scenario: A change during a running turn applies from the next user message
    Given a turn on anthropic/claude-opus-5-5 at effort high is running a tool loop
    When the user chooses low before the loop's next request
    Then the loop's next request carries no new effort message
    And the user's next message is preceded by the low effort message

  Scenario: After compaction the level in effect is declared at the top level
    Given a conversation on anthropic/claude-opus-5-5 that started at high and changed to low
    When the conversation is compacted and the user sends the next message
    Then the request's top-level effort is low
    And no effort message from before the compaction is sent

  Scenario: Subscription requests carry the per-message effort beta header too
    Given a conversation on anthropic/claude-opus-5-5 signed in with the Claude subscription (OAuth)
    When the user changes effort and sends the next message
    Then the request carries the beta header mid-conversation-output-config-2026-07-01 with the subscription beta headers
