@done
@napi
@persistence
@session
@session-management
@NAPI-008
Feature: Complete Session State Persistence and Restoration

  """
  Architecture:
  - Messages stored as structured JSON matching Claude Code's format
  - Each message wrapped in envelope with metadata (uuid, timestamp, model)
  - Multi-part messages preserve array structure with typed content blocks
  - Large content (>10KB) stored in blob storage with SHA-256 hash reference
  - Token tracker and turn state persisted in session manifest
  - Restored sessions reconstruct full rig::message::Message types

  Message Envelope Schema (outer wrapper per message):
    {
      "uuid": "a6bdbefb-902d-4f98-b539-8cbee91ec831",
      "parentUuid": "81dc2799-ef52-4923-aa24-5798585aae57",
      "timestamp": "2025-12-23T08:51:44.813Z",
      "type": "assistant",
      "provider": "claude",
      "message": { ... },
      "requestId": "req_011CWPLKJZcigRWVriKduWSr"
    }

  Provider Types (from ProviderType enum):
  - claude: Anthropic Claude API (includes OAuth mode)
  - openai: OpenAI API
  - codex: Codex/ChatGPT backend API
  - gemini: Google Gemini API

  Assistant Message Schema:
    {
      "role": "assistant",
      "id": "msg_01Wk7SCqoakQaEmx7FHphZRJ",
      "model": "claude-opus-4-5-20251101",
      "content": [
        {"type": "text", "text": "..."},
        {"type": "tool_use", "id": "toolu_123", "name": "read_file", "input": {"path": "/foo.ts"}},
        {"type": "thinking", "thinking": "...", "signature": "..."}
      ],
      "stop_reason": "end_turn",
      "usage": {
        "input_tokens": 100,
        "output_tokens": 50,
        "cache_read_input_tokens": 200,
        "cache_creation_input_tokens": 50
      }
    }

  User Message Schema:
    {
      "role": "user",
      "content": [
        {"type": "text", "text": "..."},
        {"type": "tool_result", "tool_use_id": "toolu_123", "content": "...", "is_error": false},
        {"type": "image", "source": {"type": "base64", "data": "...", "media_type": "image/png"}},
        {"type": "document", "source": {"type": "base64", "data": "...", "media_type": "application/pdf"}}
      ]
    }

  Tool Result Extended Metadata (stored alongside message):
    {
      "toolUseResult": {
        "stdout": "raw output...",
        "stderr": "",
        "interrupted": false,
        "isImage": false
      }
    }

  DocumentSourceKind Handling (from rig):
  - Base64: Already encoded string, store as-is (use blob if >10KB)
  - Url: External reference string, store inline (small)
  - Raw: Binary bytes, encode to base64 before storage (use blob if >10KB)
  - String: Text data, store inline

  Image/Document Storage Rules:
  - All binary data (Base64, Raw) uses blob storage if decoded size >10KB
  - URL references stored inline (just the URL string)
  - media_type MUST be preserved (required by all LLM providers)

  See: codelet/spec/research/NAPI-008/session-persistence-analysis.md
  """

  Background: User Story
    As a developer resuming a coding session
    I want to have ALL conversation context restored including tool calls, tool results, and thinking content
    So that the LLM can continue seamlessly from where we left off with full context, and compaction works correctly

  # ==========================================================================
  # MESSAGE ENVELOPE METADATA
  # Each message has envelope metadata for threading and debugging
  # ==========================================================================

  @envelope @uuid
  Scenario: Message UUID preserved for identity
    Given a session with an assistant message with uuid "a6bdbefb-902d-4f98-b539-8cbee91ec831"
    When the session is saved and restored
    Then the restored message uuid equals "a6bdbefb-902d-4f98-b539-8cbee91ec831"

  @envelope @threading
  Scenario: Parent UUID preserved for message threading
    Given a session with messages:
      | uuid   | parentUuid | role      |
      | msg-1  |            | user      |
      | msg-2  | msg-1      | assistant |
      | msg-3  | msg-2      | user      |
      | msg-4  | msg-3      | assistant |
    When the session is saved and restored
    Then each message parentUuid matches its predecessor's uuid
    And message threading can be reconstructed

  @envelope @timestamp
  Scenario: Message timestamp preserved for ordering
    Given a session with an assistant message with timestamp "2025-12-23T08:51:44.813Z"
    When the session is saved and restored
    Then the restored message timestamp equals "2025-12-23T08:51:44.813Z"

  @envelope @model
  Scenario: Model identifier preserved per message
    Given a session with an assistant message:
      | field | value                      |
      | model | claude-opus-4-5-20251101   |
      | text  | Response from Opus         |
    When the session is saved and restored
    Then the restored message model equals "claude-opus-4-5-20251101"

  @envelope @model @multi-model
  Scenario: Mixed model session preserved correctly
    Given a session with messages from different models:
      | role      | model                      | text                |
      | assistant | claude-sonnet-4-20250514   | Response from Sonnet|
      | assistant | claude-opus-4-5-20251101   | Response from Opus  |
    When the session is saved and restored
    Then message 0 model equals "claude-sonnet-4-20250514"
    And message 1 model equals "claude-opus-4-5-20251101"

  @envelope @provider
  Scenario: Provider type preserved per message
    Given a session with an assistant message:
      | field    | value                    |
      | provider | claude                   |
      | model    | claude-opus-4-5-20251101 |
      | text     | Response from Claude     |
    When the session is saved and restored
    Then the restored message provider equals "claude"

  @envelope @provider @multi-provider
  Scenario: Mixed provider session preserved correctly
    Given a session with messages from different providers:
      | role      | provider | model                      | text                 |
      | assistant | claude   | claude-opus-4-5-20251101   | Response from Claude |
      | assistant | openai   | gpt-4-turbo                | Response from OpenAI |
      | assistant | gemini   | gemini-pro                 | Response from Gemini |
    When the session is saved and restored
    Then message 0 provider equals "claude"
    And message 1 provider equals "openai"
    And message 2 provider equals "gemini"

  @envelope @request-id
  Scenario: API request ID preserved for debugging
    Given a session with an assistant message with requestId "req_011CWPLKJZcigRWVriKduWSr"
    When the session is saved and restored
    Then the restored message requestId equals "req_011CWPLKJZcigRWVriKduWSr"

  @envelope @stop-reason
  Scenario: Stop reason preserved for response analysis
    Given a session with an assistant message:
      | field       | value     |
      | stop_reason | end_turn  |
      | text        | Complete response |
    When the session is saved and restored
    Then the restored message stop_reason equals "end_turn"

  @envelope @stop-reason @max-tokens
  Scenario: Max tokens stop reason indicates truncation
    Given a session with an assistant message:
      | field       | value      |
      | stop_reason | max_tokens |
      | text        | Truncated response that was cut off mid- |
    When the session is saved and restored
    Then the restored message stop_reason equals "max_tokens"

  # ==========================================================================
  # CONTENT TYPE PERSISTENCE
  # Each content type from rig::message must be persisted and restored
  # ==========================================================================

  @content-type @text
  Scenario: Text content persisted and restored
    Given a session with an assistant message containing text "Hello, world"
    When the session is saved and restored
    Then the restored message has content type "text"
    And the restored message text equals "Hello, world"

  @content-type @message-id
  Scenario: Assistant message ID preserved
    Given a session with an assistant message:
      | field | value            |
      | id    | msg_01Abc123XYZ  |
      | text  | Here is my response |
    When the session is saved and restored
    Then the restored assistant message id equals "msg_01Abc123XYZ"

  @content-type @tool-use
  Scenario: Tool use content persisted and restored
    Given a session with an assistant message containing a tool use:
      | field | value                         |
      | id    | toolu_01AyTnm7YLfybhnhEhwwZvAY |
      | name  | read_file                     |
      | input | {"path": "/etc/config.json"}  |
    When the session is saved and restored
    Then the restored message has content type "tool_use"
    And the restored tool use id equals "toolu_01AyTnm7YLfybhnhEhwwZvAY"
    And the restored tool use name equals "read_file"
    And the restored tool use input equals {"path": "/etc/config.json"}

  @content-type @tool-result
  Scenario: Tool result content persisted and restored
    Given a session with a user message containing a tool result:
      | field       | value                            |
      | tool_use_id | toolu_01AyTnm7YLfybhnhEhwwZvAY   |
      | content     | {"port": 8080}                   |
      | is_error    | false                            |
    When the session is saved and restored
    Then the restored message has content type "tool_result"
    And the restored tool result tool_use_id equals "toolu_01AyTnm7YLfybhnhEhwwZvAY"
    And the restored tool result content equals {"port": 8080}
    And the restored tool result is_error equals false

  @content-type @tool-result @error
  Scenario: Tool error result persisted with error flag
    Given a session with a user message containing a tool result:
      | field       | value             |
      | tool_use_id | toolu_xyz789      |
      | content     | Permission denied |
      | is_error    | true              |
    When the session is saved and restored
    Then the restored message has content type "tool_result"
    And the restored tool result is_error equals true
    And the restored tool result content equals "Permission denied"

  @content-type @tool-result @raw-output
  Scenario: Tool result raw output metadata preserved
    Given a session with a user message containing a tool result:
      | field       | value                |
      | tool_use_id | toolu_abc123         |
      | content     | formatted output     |
    And the tool result has raw metadata:
      | field       | value                |
      | stdout      | raw stdout content   |
      | stderr      | warning: deprecated  |
      | interrupted | false                |
      | isImage     | false                |
    When the session is saved and restored
    Then the restored tool result has raw metadata
    And the raw metadata stdout equals "raw stdout content"
    And the raw metadata stderr equals "warning: deprecated"
    And the raw metadata interrupted equals false

  @content-type @thinking
  Scenario: Thinking content with signature persisted and restored
    Given a session with an assistant message containing thinking:
      | field     | value                                    |
      | thinking  | Let me analyze this step by step...      |
      | signature | EqsDCkYIChgCKkDLMHNM46KZ1nevdcXu9...     |
    When the session is saved and restored
    Then the restored message has content type "thinking"
    And the restored thinking text equals "Let me analyze this step by step..."
    And the restored thinking signature equals "EqsDCkYIChgCKkDLMHNM46KZ1nevdcXu9..."

  @content-type @thinking @edge-case
  Scenario: Thinking content without signature persisted
    Given a session with an assistant message containing thinking:
      | field     | value                              |
      | thinking  | Let me think about this problem... |
      | signature |                                    |
    When the session is saved and restored
    Then the restored message has content type "thinking"
    And the restored thinking text equals "Let me think about this problem..."
    And the restored thinking signature is null

  @content-type @image @base64
  Scenario: Base64 image content persisted and restored
    Given a session with a user message containing an image:
      | field       | value                    |
      | source_type | base64                   |
      | media_type  | image/png                |
      | data_size   | 50000                    |
    When the session is saved and restored
    Then the image data is stored in blob storage
    And the restored message has content type "image"
    And the restored image source type equals "base64"
    And the restored image media_type equals "image/png"
    And the restored image data matches the original bytes

  @content-type @image @url
  Scenario: URL image reference persisted inline
    Given a session with a user message containing an image:
      | field       | value                              |
      | source_type | url                                |
      | media_type  | image/jpeg                         |
      | url         | https://example.com/screenshot.jpg |
    When the session is saved and restored
    Then the image URL is stored inline in the message
    And no blob reference is created for the image
    And the restored image source type equals "url"
    And the restored image url equals "https://example.com/screenshot.jpg"

  @content-type @document @base64
  Scenario: Base64 document content persisted and restored
    Given a session with a user message containing a document:
      | field       | value           |
      | source_type | base64          |
      | media_type  | application/pdf |
      | data_size   | 100000          |
    When the session is saved and restored
    Then the document data is stored in blob storage
    And the restored message has content type "document"
    And the restored document source type equals "base64"
    And the restored document media_type equals "application/pdf"
    And the restored document data matches the original bytes

  @content-type @document @url
  Scenario: URL document reference persisted inline
    Given a session with a user message containing a document:
      | field       | value                           |
      | source_type | url                             |
      | media_type  | application/pdf                 |
      | url         | https://example.com/report.pdf  |
    When the session is saved and restored
    Then the document URL is stored inline in the message
    And the restored document source type equals "url"
    And the restored document url equals "https://example.com/report.pdf"

  # ==========================================================================
  # STRUCTURE PRESERVATION
  # Multi-part messages and ordering must be preserved
  # ==========================================================================

  @structure @multi-part
  Scenario: Multi-part assistant message preserves structure and order
    Given a session with an assistant message containing:
      | index | type     | summary                   |
      | 0     | text     | Let me check that file... |
      | 1     | tool_use | read_file(path=/foo.ts)   |
      | 2     | text     | Found it. Now searching...|
      | 3     | tool_use | grep(pattern=error)       |
      | 4     | text     | Here's what I found.      |
    When the session is saved and restored
    Then the restored message has 5 content parts
    And content part 0 has type "text"
    And content part 1 has type "tool_use"
    And content part 2 has type "text"
    And content part 3 has type "tool_use"
    And content part 4 has type "text"

  @structure @parallel-tools
  Scenario: Parallel tool uses in single message preserved
    Given a session with an assistant message containing 3 tool uses:
      | id         | name      |
      | toolu_001  | read_file |
      | toolu_002  | grep      |
      | toolu_003  | bash      |
    And a user message containing 3 tool results:
      | tool_use_id | content          |
      | toolu_001   | file contents    |
      | toolu_002   | grep output      |
      | toolu_003   | command output   |
    When the session is saved and restored
    Then the restored assistant message has 3 tool_use content parts
    And the restored user message has 3 tool_result content parts
    And each tool_result tool_use_id matches its corresponding tool_use id

  @structure @turn-sequence
  Scenario: Message sequence preserved across multiple turns
    Given a session with 5 turns of user-assistant exchanges
    And each turn has unique content
    When the session is saved and restored
    Then the restored session has 10 messages
    And message 0 has role "user"
    And message 1 has role "assistant"
    And the message sequence alternates user-assistant

  # ==========================================================================
  # BLOB STORAGE
  # Large content uses content-addressed storage with deduplication
  # ==========================================================================

  @blob @threshold
  Scenario: Content above 10KB threshold stored in blob
    Given a session with a tool result containing 15000 bytes of content
    When the session is saved
    Then the content is stored in blob storage
    And the message contains a blob hash reference
    And the blob hash is a valid SHA-256 hex string

  @blob @threshold @boundary
  Scenario: Content at exactly 10KB threshold stored inline
    Given a session with a tool result containing exactly 10240 bytes
    When the session is saved
    Then the content is stored inline in the message
    And no blob reference is created

  @blob @deduplication
  Scenario: Duplicate large content deduplicated by hash
    Given a session with two tool results containing identical 20KB content
    When the session is saved
    Then only one blob is stored
    And both messages reference the same blob hash

  @blob @restore
  Scenario: Blob content retrieved on restore
    Given a session with a tool result stored as blob reference
    When the session is restored
    Then the blob content is retrieved from storage
    And the restored tool result content matches the original

  # ==========================================================================
  # SESSION STATE RESTORATION
  # Token tracking and turn boundaries for compaction
  # ==========================================================================

  @state @tokens
  Scenario: Aggregate token tracker state restored
    Given a session with cumulative token usage:
      | field                      | value  |
      | input_tokens               | 150000 |
      | output_tokens              | 25000  |
      | cache_read_input_tokens    | 50000  |
      | cache_creation_input_tokens| 10000  |
    When the session is saved and restored
    Then the restored token tracker input_tokens equals 150000
    And the restored token tracker output_tokens equals 25000
    And the restored token tracker cache_read_input_tokens equals 50000
    And the restored token tracker cache_creation_input_tokens equals 10000

  @state @tokens @per-message
  Scenario: Per-message token usage preserved
    Given a session with an assistant message with usage:
      | field                      | value |
      | input_tokens               | 500   |
      | output_tokens              | 150   |
      | cache_read_input_tokens    | 200   |
      | cache_creation_input_tokens| 100   |
    When the session is saved and restored
    Then the restored message has usage metadata
    And the restored message usage input_tokens equals 500
    And the restored message usage output_tokens equals 150

  @state @turns
  Scenario: Turn boundaries reconstructed from message sequence
    Given a session with 10 user-assistant exchange pairs
    When the session is saved and restored
    Then the restored session has 10 turns
    And each turn contains one user message and one assistant message
    And turn boundaries align with user message indices

  @state @compaction
  Scenario: Compaction operates correctly on restored session
    Given a session with 50 turns and 180000 input tokens
    And the compaction threshold is 200000 tokens
    When the session is saved and restored
    And 5 more turns are added reaching 210000 tokens
    Then compaction is triggered
    And compaction summarizes the oldest turns
    And recent turns are preserved verbatim

  # ==========================================================================
  # FUNCTIONAL EQUIVALENCE
  # Restored sessions must behave identically to continuous sessions
  # ==========================================================================

  @equivalence @critical
  Scenario: Restored session indistinguishable from continuous session
    Given two sessions with identical initial prompts
    And session A continues without interruption for 3 turns
    And session B is saved after 3 turns and restored
    When the messages array is serialized to API format for both sessions
    Then the serialized messages are byte-identical
    And both sessions have the same message count
    And both sessions have matching token counts

  @equivalence @context
  Scenario: Restored session enables context-aware follow-up
    Given a session where read_file tool returned config with port=8080 and host=localhost
    And the assistant only mentioned the port in the response
    When the session is saved and restored
    And the user asks "what was the host value in that config?"
    Then the restored session contains the full tool result with host=localhost
    And the follow-up can be answered without re-reading the file

  # ==========================================================================
  # EDGE CASES
  # Boundary conditions and error handling
  # ==========================================================================

  @edge-case @empty
  Scenario: Session with only text messages (no tools)
    Given a session with 3 text-only exchanges
    And no tool uses or results
    When the session is saved and restored
    Then all 6 messages are restored with correct roles
    And all message text content matches original
    And no blob storage is used

  @edge-case @single-message
  Scenario: Session with single user message
    Given a session with only one user message "Hello"
    And no assistant response yet
    When the session is saved and restored
    Then the restored session has exactly 1 message
    And the restored message role equals "user"
    And the restored message text equals "Hello"

  @edge-case @large-session
  Scenario: Session with many turns restores completely
    Given a session with 100 turns
    And mixed content types across messages
    When the session is saved and restored
    Then the restored session has exactly 200 messages
    And each message content hash matches original
    And message order is preserved

  @edge-case @no-message-id
  Scenario: Assistant message without ID handled gracefully
    Given a session with an assistant message:
      | field | value               |
      | id    |                     |
      | text  | Response without ID |
    When the session is saved and restored
    Then the restored assistant message id is null
    And the restored message text equals "Response without ID"

  @edge-case @interrupted-tool
  Scenario: Interrupted tool execution preserved
    Given a session with a user message containing a tool result:
      | field       | value           |
      | tool_use_id | toolu_abc123    |
      | content     | partial output  |
      | is_error    | true            |
    And the tool result has raw metadata:
      | field       | value |
      | interrupted | true  |
    When the session is saved and restored
    Then the raw metadata interrupted equals true
    And the tool result is_error equals true
