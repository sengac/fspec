@done
@rust
@scrollback
@agent-view
@bug
@tui
@RPC-078
Feature: AgentView chunk rendering parity with TS Ink reference
  """
  Each StreamChunk variant maps to exactly the same scrollback line text + foreground colour that the TS Ink chunkProcessor / thinkingBlockManager produces. The pre-RPC-078 Rust ladder used Debug-style literals ("user>", "assistant>", "[done]", ...) — this feature pins the corrected ladder and the no-duplicate UserInput invariant.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES (exact TS Ink parity per chunk variant):
  #   1. StreamChunk::UserInput renders ONE green line "You: <text>" — never "user> <text>"
  #   2. StreamChunk::Text renders ONE white line "● <text>" (U+25CF + space) — never "assistant> <text>"
  #   3. StreamChunk::Thinking renders a yellow block starting with "[Thinking]\n<text>" — never "(thinking)"
  #   4. StreamChunk::Error renders ONE white line "API Error: <error>" — never "[error] <error>"
  #   5. StreamChunk::Done strips the trailing "..." streaming suffix but emits NO new line — never "[done]"
  #   6. StreamChunk::Interrupted renders ONE white line "⚠ Interrupted" — never "[interrupted]"
  #   7. StreamChunk::UserNotification renders the message verbatim with no prefix — never "[notice]"
  #   8. StreamChunk::IncomingMessage renders ONE magenta line "[W] <role>> <body>" (W = Worker) — never "supervisor>"
  #   9. Action::InputSubmitted MUST NOT synchronously push a "user>" line — the chunks broadcast path is the single source of truth
  #
  # ========================================
  Background: User Story
    As a user typing into a TUI session
    I want each chunk variant to render with its TS Ink prefix and colour
    So that the scrollback matches the reference Ink implementation byte-for-byte and never shows internal Debug literals

  Scenario: UserInput chunk renders as green You: prefixed line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::UserInput { text: "hi" } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "You: hi"
    Then the rendered chunk's spans carry foreground color GREEN
    Then no rendered chunk contains the substring "user> "

  Scenario: Text assistant chunk renders as white circle-bullet prefixed line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Text { text: "hello back" } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "● hello back"
    When the chunks subscriber forwards StreamChunk::Done for s-1
    Then no rendered chunk contains the substring "assistant> " or "[done]"

  Scenario: Error chunk renders as API Error prefixed status line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Error { error: "rate limit exceeded" } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "API Error: rate limit exceeded"
    Then no rendered chunk contains the substring "[error]"

  Scenario: Done chunk produces no scrollback line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Done for s-1
    Then the s-1 scrollback contains zero rendered chunks
    Then no rendered chunk contains the substring "[done]"

  Scenario: Interrupted chunk renders as warning-prefixed status line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Interrupted { queued_inputs: vec!["a", "b"] } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "⚠ Interrupted"
    Then no rendered chunk contains the substring "[interrupted]" or "queued"

  Scenario: UserNotification chunk renders the message verbatim with no extra prefix
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::UserNotification { message: "⟳ Reconnecting..." } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "⟳ Reconnecting..."
    Then no rendered chunk contains the substring "[notice]"

  Scenario: IncomingMessage chunk parses SUPERVISOR prefix and renders as magenta bracket-W line
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::IncomingMessage { text: "[SUPERVISOR: reviewer | Session: s-2]\nplease check this" } for s-1
    Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "[W] reviewer> please check this"
    Then the rendered chunk's spans carry foreground color MAGENTA
    Then no rendered chunk contains the substring "supervisor> "

  Scenario: Thinking chunk renders as yellow Thinking-prefixed block
    Given an AgentView with a fresh SessionContext for session s-1
    When the chunks subscriber forwards StreamChunk::Thinking { thinking: "considering the options" } for s-1
    Then the s-1 scrollback contains a rendered chunk whose first visible line equals "[Thinking]"
    Then the rendered chunk's spans carry foreground color YELLOW
    Then no rendered chunk contains the substring "(thinking)"

  Scenario: User input is not duplicated when the session manager broadcasts UserInput back
    Given an AgentView with an open session s-1 backed by a real session manager
    When the App dispatches Action::InputSubmitted("is this card done?")
    Then the s-1 scrollback contains exactly one rendered chunk whose visible text equals "You: is this card done?"
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::UserInput { text: "is this card done?" })
    Then the count of "You: is this card done?" lines is exactly 1
