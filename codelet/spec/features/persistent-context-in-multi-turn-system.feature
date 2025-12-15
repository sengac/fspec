@session-management
@cli
@context-management
@agent-execution
@CLI-008
Feature: Persistent Context in Multi-Turn System
  """
  SessionWrapper pattern: Create Session struct owning ProviderManager, messages Vec<Message>, ContextManager, TokenTracker. Matches codelet's REPL scope pattern. Uses rig's stateless agent with caller-managed history via with_history(&mut Vec<Message>). Messages persist across REPL iterations but cleared on provider switch. Interruption support with snapshot/restore for 'continue' command.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Message array must persist across REPL iterations within the same session
  #   2. All messages (user, assistant, tool results) must be accumulated in the message array
  #   3. System reminders must be deduplicated by type to prevent context bloat
  #   4. Agent must NOT be recreated on each REPL iteration - reuse existing agent instance
  #   5. Provider switching must clear the conversation context and create new session
  #   6. Tool execution must happen within the streaming loop without exiting the REPL iteration
  #
  # EXAMPLES:
  #   1. User asks 'List files in current directory', Bash tool executes and shows 5 files, then user says 'Edit the first one' and agent knows which file to edit
  #   2. User asks to create file 'test.rs', Write tool succeeds, then user asks 'Add a main function to it' and agent remembers the file was just created
  #   3. System reminder for environment info is added on first turn, then on second turn the old reminder is removed and new one is prepended (no duplication)
  #   4. User has conversation with Claude, then types ':claude' to switch to OpenAI, and the message history is cleared for the new provider
  #   5. User asks to run tests, Bash tool executes 'cargo test' with streaming output, test results are captured in message array, then agent responds with summary
  #
  # QUESTIONS (ANSWERED):
  #   Q: Should we store the agent instance in ProviderManager, create a new SessionWrapper struct, or enhance RigAgent with message storage?
  #   A: Option B: SessionWrapper - Create a new Session struct that owns ProviderManager, messages Vec, ContextManager, and TokenTracker. This matches codelet's REPL scope pattern exactly and works perfectly with rig's stateless agent design requiring caller-managed history via with_history(&mut Vec<Message>)
  #
  #   Q: Does the Rig framework's Agent have built-in conversation history support that we're not utilizing, or do we need to manage it ourselves?
  #   A: Rig does NOT have built-in conversation history support. The Agent is completely stateless with no messages field. We MUST manage history ourselves using with_history(&mut Vec<Message>) which rig mutates in place. This is confirmed by rig-core examples showing explicit caller-managed history (e.g., complex_agentic_loop_claude.rs lines 169-177)
  #
  #   Q: Should we create a new CoreMessage enum matching codelet's structure, or reuse the existing Message struct from agent/mod.rs?
  #   A: Reuse the existing Message struct from agent/mod.rs. It already matches codelet's CoreMessage pattern perfectly: has role (MessageRole enum), content (MessageContent with Text and Parts variants), supports tool use/results via ContentPart enum, is serializable, and has helper constructors (Message::user/assistant/system). No need to create a new type - the existing one is well-designed and type-safe.
  #
  #   Q: On Ctrl+C interruption, should we preserve context for resume (like codelet does), or clear it?
  #   A: Preserve context on Ctrl+C/ESC interruption (like codelet does). Session should snapshot messages before each turn and allow resume via 'continue' command. This matches codelet's proven UX pattern (messagesBeforeInterruption), is user-friendly for long operations, and SessionWrapper naturally supports it with messages_before_interruption: Option<Vec<Message>> field and snapshot/restore methods.
  #
  # ========================================
  Background: User Story
    As a developer using codelet in interactive mode
    I want to have multi-turn conversations where the agent remembers previous context
    So that I can refer to previous tool results and conversation history without repeating myself

  Scenario: Provider switching clears conversation context
    Given I am in an interactive REPL session with Claude provider
    And I have had a multi-turn conversation with message history
    When I type "/openai" to switch providers
    Then the message history should be cleared
    And the session should start fresh with the new provider
    And previous conversation context should not be accessible

  Scenario: Session creates and stores message history
    Given I create a new Session
    When I access the messages vector
    Then the messages vector should be empty initially
    And I should be able to add messages to it

