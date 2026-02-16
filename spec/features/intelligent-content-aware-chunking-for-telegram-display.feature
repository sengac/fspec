@wip
@critical
@chunking
@telegram
@bridge
@BRIDGE-006
Feature: Intelligent Content-Aware Chunking for Telegram Display

  """
  Implementation should be a content-aware buffer that accumulates streaming data and flushes at detected boundaries
  Use a boundary detector with priority: code block > heading > paragraph > sentence > max size
  Content type handlers: ThinkingHandler (summarize), ToolCallHandler (format), ToolResultHandler (summarize), TextHandler (chunk)
  CURRENT STATE: telegram-endpoint.ts has simple idle-based buffering (800ms idle or 3500 char limit triggers flush). NO content-aware boundary detection exists yet.
  CURRENT STATE: truncateMessage() in telegram-formatting.ts handles oversized messages by preserving first/last 1500 chars with omission indicator. Properly handles code block fence boundaries.
  CURRENT STATE: formatForTelegram() passes thinking blocks and tool results verbatim with emoji prefixes (💭, 🔧, ❌). NO summarization exists yet.
  NEW CODE LOCATION: Create bridge/telegram-content-chunker.ts for boundary detection logic. Integrate into handleStreamChunk() in telegram-endpoint.ts.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Messages must never break mid-sentence - chunks should end at sentence boundaries (., !, ?) when possible
  #   2. Code blocks (``` ... ```) must be sent as complete units, never split across messages
  #   3. Thinking blocks should be summarized or condensed, not streamed raw - show '🤔 Analyzing...' style indicators
  #   4. Tool output (especially file reads) should be summarized with key info, not sent verbatim - e.g., '📄 Read file.ts (245 lines)'
  #   5. Each message must have valid, complete markdown - no unclosed backticks, bold markers, etc.
  #   6. Messages must respect Telegram's 4096 character limit - split at nearest logical boundary before limit
  #   7. Paragraph breaks (double newlines) are preferred chunking points
  #   8. Headings (# ## ###) should start new chunks, not appear mid-chunk
  #   9. Base64 images are too large for Telegram (4096 char limit). Show placeholder like '📷 [image referenced]' or skip entirely. Current code already handles photo uploads separately.
  #
  # EXAMPLES:
  #   1. When Claude streams 'I will analyze this...' the full sentence arrives in one message, not 'I will ana' then 'lyze this...'
  #   2. When Claude outputs a 50-line code block, Telegram receives it as a single message with proper ```language``` formatting
  #   3. When Claude reads a 500-line file, Telegram shows '📄 Read src/auth.ts (500 lines)' instead of the full file content
  #   4. When Claude thinks for 30 seconds, Telegram shows '🤔 Analyzing implementation approach...' rather than streaming raw thinking
  #   5. When a message would be 5000 chars, it splits at the paragraph break before 4096 chars, sending two complete messages
  #   6. When Claude writes '## Problem\n\nThe issue is...' the heading starts a new message, not appearing mid-chunk
  #   7. When a tool call is made, Telegram shows '🔧 Running: Fspec(create-story) with args...' in a formatted way
  #   8. When there's a list with 10 items, all 10 items arrive together, not split across messages
  #
  # QUESTIONS (ANSWERED):
  #   Q: How should images referenced in responses be handled - send the full base64 or just a placeholder?
  #   A: Truncate with indicator - current truncateMessage() already does this well: keeps first/last 1500 chars with '[...N chars omitted...]' in middle. Properly closes/reopens code block fences.
  #
  #   Q: Should there be a configurable 'verbosity level' so users can choose between summary vs detailed output?
  #   A: Yes - tool errors already get ❌ prefix in formatForTelegram(). Keep this pattern.
  #
  # ASSUMPTIONS:
  #   1. Not for v1. Keep it simple - default to summarized output. Verbosity control is future enhancement if users request it.
  #   2. Code blocks exceeding 4096 chars: Use existing truncateMessage() behavior - truncate in middle with '[...N chars omitted...]' indicator, properly close/reopen code fences
  #   3. Tool errors: Keep existing pattern - ❌ prefix already differentiates errors from success in formatForTelegram()
  #
  # ========================================

  Background: User Story
    As a user viewing Claude's output via Telegram
    I want to receive well-formatted, logically-chunked messages
    So that I can read the AI's responses naturally without fragmented or overwhelming content

  # ===========================================
  # BOUNDARY DETECTION SCENARIOS
  # ===========================================

  @boundary
  Scenario: Complete sentence arrives in single message
    Given the chunker receives streaming text "I will analyze this code."
    When the idle timeout triggers a flush
    Then Telegram receives "I will analyze this code." as a single message
    And the message is not split mid-word

  @boundary
  Scenario: Buffer flushes at sentence boundary when approaching limit
    Given the buffer contains 3400 characters of text
    And the next chunk would push it over 3500 characters
    When the chunker detects a sentence boundary at 3200 characters
    Then it flushes at the sentence boundary
    And the remaining text stays in the buffer for the next message

  @boundary @code-block
  Scenario: Code block arrives as complete unit
    Given the chunker receives a code block "```typescript\nconst x = 1;\n```"
    When the idle timeout triggers a flush
    Then Telegram receives the complete code block in one message
    And the code block is not split across messages

  @boundary @code-block
  Scenario: Multi-line code block stays together
    Given the chunker receives a 50-line code block
    And the code block is under 4096 characters
    When the idle timeout triggers a flush
    Then all 50 lines arrive in a single Telegram message

  @boundary @paragraph
  Scenario: Paragraph break triggers new chunk
    Given the buffer contains "First paragraph.\n\nSecond paragraph."
    When the buffer is flushed
    Then "First paragraph." becomes one message
    And "Second paragraph." becomes the next message

  @boundary @heading
  Scenario: Heading starts new message
    Given the buffer contains "Some text.\n\n## New Section\n\nMore text."
    When the buffer is flushed
    Then "Some text." is sent first
    And "## New Section\n\nMore text." starts a new message

  @boundary @list
  Scenario: List items stay together in single message
    Given the chunker receives a list with 10 items
    And the total list is under 4096 characters
    When the idle timeout triggers a flush
    Then all 10 items arrive in a single Telegram message

  @boundary @priority
  Scenario: Code block boundary takes priority over paragraph
    Given the buffer contains text followed by a code block followed by a paragraph
    When the buffer approaches the size limit inside the code block
    Then it waits for the code block to complete before flushing
    And does not split at the paragraph boundary inside the code block

  # ===========================================
  # CONTENT SUMMARIZATION SCENARIOS
  # ===========================================

  @summarization @thinking
  @summarization @thinking
  @summarization @thinking
  @summarization @tool
  Scenario: Tool call displays formatted invocation
    Given Claude invokes the Fspec tool with command "create-story"
    When the tool_call chunk is processed
    Then Telegram shows "🔧 Running: Fspec(create-story)"

  @summarization @tool
  Scenario: File read tool result shows summary with line count
    Given Claude reads a 500-line file "src/auth.ts"
    When the tool_result chunk is processed
    Then Telegram shows "📄 Read src/auth.ts (500 lines)" instead of file contents

  @summarization @tool
  Scenario: Large tool output summarized not sent verbatim
    Given a tool returns 10000 characters of output
    When the tool_result chunk is processed
    Then Telegram receives a summary under 500 characters
    And the full output is not sent

  @summarization @tool
  Scenario: Tool call with arguments shows arg summary
    Given Claude invokes Read with file_path "/home/user/file.ts"
    When the tool_call chunk is processed
    Then Telegram shows "🔧 Running: Read(file_path: /home/user/file.ts)"

  # ===========================================
  # MARKDOWN VALIDATION SCENARIOS
  # ===========================================

  @validation @limit
  Scenario: Message respects 4096 character limit
    Given the buffer contains 5000 characters of text
    When the buffer is flushed
    Then no single Telegram message exceeds 4096 characters

  @validation @limit
  Scenario: Long message splits at logical boundary before limit
    Given the buffer contains 5000 characters with a paragraph break at 3800
    When the buffer is flushed
    Then the first message ends at the paragraph break
    And the second message contains the remainder

  @validation @markdown
  Scenario: Unclosed code block closed before sending
    Given the buffer contains "```typescript\nconst x = 1;" without closing fence
    And the buffer is being force-flushed due to size limit
    When the message is prepared for sending
    Then a closing "```" is appended to make valid markdown

  @validation @markdown
  Scenario: Unclosed bold markers balanced before sending
    Given the buffer contains "This is **bold text without closing"
    And the buffer is being force-flushed
    When the message is prepared for sending
    Then a closing "**" is appended to balance the markers

  @validation @markdown
  Scenario: Inline code backticks balanced in each chunk
    Given the buffer contains "Use the `command without closing"
    And the buffer is being force-flushed
    When the message is prepared for sending
    Then a closing backtick is appended

  @validation @code-block @limit
  Scenario: Code block exceeding limit truncated with indicator
    Given a code block contains 6000 characters
    When it is processed for Telegram
    Then it is truncated to under 4096 characters
    And includes "[...N chars omitted...]" indicator

  @validation @code-block @limit
  Scenario: Truncated code block has closing fence
    Given a code block is truncated due to size
    When the truncated message is prepared
    Then it has both opening and closing "```" fences
    And the markdown is valid

  @boundary
  @table
  Scenario: Markdown table arrives as complete unit
    Given the chunker receives a markdown table with header, separator and 5 data rows
    When the idle timeout triggers a flush
    Then all table rows arrive in a single Telegram message
    And the table is not split mid-row


  @boundary
  @table
  @priority
  Scenario: Table boundary takes priority - splits before table not mid-table
    Given the buffer contains 3000 characters of text followed by a 2000 character table
    When the buffer is flushed due to exceeding the 4096 limit
    Then the first message contains the text before the table
    And the second message contains the complete table
    And no table row is split across messages


  @boundary
  @table
  Scenario: Table row never split mid-row
    Given the buffer contains a table row "| Feb 10 | $5,048.96 | +0.17% |"
    When the buffer approaches the size limit mid-row
    Then it waits for the row to complete before flushing
    And the row "| Feb 10 | $5,048.96 | +0.17% |" is never split into separate messages


  @thinking
  Scenario: Thinking content wrapped in think tags
    Given Claude sends a thinking chunk with reasoning content
    When the thinking block is processed for Telegram
    Then the first message starts with '<think>'
    And the actual thinking content flows naturally
    And the final message ends with '</think>'


  @thinking
  Scenario: Multiple thinking chunks stream as continuous content
    Given Claude sends 5 separate thinking chunks in succession
    When they are processed for Telegram
    Then the content flows between single '<think>' and '</think>' tags
    And NOT 5 separate '🤔' indicator messages

