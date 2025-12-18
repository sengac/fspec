@agent-integration
@NAPI-002
@high
@tui
@ai-agent
@codelet
Feature: TUI Integration for Codelet AI Agent
  """
  Architecture notes:
  - Integrates codelet-napi native module (Rust via NAPI-RS) into existing fspec Ink/React TUI
  - Modal overlay renders on top of existing TUI views (board, work unit details)
  - Uses CodeletSession class from codelet-napi for AI provider interactions
  - Streaming via ThreadsafeFunction callback for real-time chunk display
  - Fresh session per modal open (no persistence) simplifies state management
  - Provider auto-detection priority: Claude > Gemini > Codex > OpenAI
  - Tool calls from AI are auto-executed without user approval prompts

  Dependencies:
  - codelet-napi: Native module for AI agent interactions
  - Ink/React: TUI rendering framework already used by fspec
  - Existing TUI infrastructure in src/tui/

  Critical implementation requirements:
  - MUST use streaming for responsive UX (not blocking prompt())
  - MUST handle provider unavailable gracefully with clear error messages
  - MUST display token usage for cost awareness
  - MUST support Escape key to close modal
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. TUI uses modal overlay approach - conversation appears as overlay on existing views
  #   2. Tool execution is automatic - no approval required from user
  #   3. Session is fresh each time - no persistence between conversations
  #   4. Provider switching is supported mid-conversation via session.switchProvider()
  #   5. Provider auto-detection selects highest priority available provider (Claude > Gemini > Codex > OpenAI)
  #   6. Streaming responses are displayed in real-time as text chunks arrive
  #
  # EXAMPLES:
  #   1. User opens agent modal with hotkey, types prompt, sees streaming response in real-time
  #   2. Agent receives tool call, executes automatically, displays result without user approval
  #   3. User switches provider from Claude to Gemini mid-conversation using provider selector
  #   4. Modal displays token usage and current provider in header/status area
  #   5. User closes modal, opens again later - previous conversation is gone, fresh session starts
  #   6. No credentials found - modal shows error message with setup instructions
  #
  # ========================================
  Background: User Story
    As a developer using fspec TUI
    I want to interact with AI agent in the terminal
    So that I can get AI assistance without leaving my workflow

  Scenario: Open agent modal and send prompt with streaming response
    Given I am viewing the TUI board
    And at least one AI provider is configured
    When I press the agent modal hotkey
    Then an agent modal overlay should appear
    And the modal should show the current provider name
    When I type a prompt and press Enter
    Then the response should stream in real-time
    And I should see text appearing character by character

  Scenario: Auto-execute tool calls without approval
    Given I have the agent modal open
    And I send a prompt that triggers a tool call
    When the AI responds with a tool call
    Then the tool should execute automatically
    And the tool result should display in the conversation
    And no approval prompt should appear

  Scenario: Switch provider mid-conversation
    Given I have the agent modal open with an active conversation
    And multiple providers are available
    When I select a different provider from the provider selector
    Then the provider should switch successfully
    And the modal should display the new provider name
    And I can continue the conversation with the new provider

  Scenario: Display token usage and provider status
    Given I have the agent modal open
    When I send a prompt and receive a response
    Then the modal header should show token usage
    And the token count should include input and output tokens
    And the current provider name should be visible

  Scenario: Fresh session on modal reopen
    Given I have the agent modal open with an active conversation
    When I close the modal with Escape key
    And I reopen the agent modal
    Then the conversation history should be empty
    And a fresh session should be initialized
    And token usage should be reset to zero

  Scenario: Handle missing credentials gracefully
    Given no AI provider credentials are configured
    When I open the agent modal
    Then an error message should display
    And the error should explain no providers are available
    And setup instructions should be shown
