@agent-integration
@tui
Feature: Proper TUI Integration Using Existing Codelet Rust Infrastructure

  """
  Architecture notes:
  - TODO: Add key architectural decisions
  - TODO: Add dependencies and integrations
  - TODO: Add critical implementation requirements
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Implementation must be almost entirely in Rust - the JavaScript/React component should only render what Rust sends via callbacks
  #   2. Must use existing stream_loop.rs and stream_handlers.rs patterns for proper message history management
  #   3. Must support Esc key interruption via an interrupt() method exposed through NAPI
  #   4. Must properly update message history using handle_text_chunk, handle_tool_call, handle_tool_result, handle_final_response patterns
  #   5. Must track tokens via CompactionHook like the CLI does
  #   6. Must load .env files for API credentials using dotenvy
  #   7. The agent modal must expand to full screen size (not a small overlay)
  #
  # EXAMPLES:
  #   1. User presses Esc during agent execution → agent stops immediately and shows partial response
  #   2. Agent calls Read tool → tool result is properly added to message history → agent can reference file content in next response
  #   3. Agent streams text → text is accumulated → final response added to messages as AssistantContent::Text
  #   4. Multi-turn conversation → each turn properly persists in message history → agent maintains context across prompts
  #   5. Session created → dotenvy loads .env → ANTHROPIC_API_KEY available → Claude provider works
  #   6. User opens agent modal → modal takes full terminal width and height → user has maximum space for conversation
  #
  # ========================================

  Background: User Story
    As a developer using fspec TUI
    I want to interact with the codelet AI agent using the same infrastructure as the codelet CLI
    So that I get the full agent experience including Esc key interruption, proper message history, token tracking, and tool execution
