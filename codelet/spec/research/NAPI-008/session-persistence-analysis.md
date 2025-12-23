# Session Persistence Analysis - NAPI-008

## Executive Summary

The current `/resume` implementation has a critical flaw: **only text content is persisted and restored**. Tool calls, tool results, thinking content, turn boundaries, and token state are all lost. This means:

1. **Resumed sessions have incomplete LLM context** - The AI sees gaps where tools were used
2. **Compaction cannot work properly** - Turn boundaries are not restored
3. **Token tracking resets** - Compaction decisions based on token counts are wrong

This document provides a comprehensive analysis of what needs to be captured, where the current implementation fails, and what the fix requires.

---

## Current Data Flow

### Streaming Phase (Working)

```
LLM Response
    ↓
StreamEvent (Text | ToolCall | ToolResult | Thinking | Done | Error)
    ↓
NapiOutput.emit() - converts to StreamChunk
    ↓
JavaScript Callback in AgentModal.tsx
    ↓
UI Display (✅ works - shows tool calls in conversation)
    ↓
Persistence (❌ BROKEN - only saves text)
```

### Restore Phase (Broken)

```
SQLite (text-only messages)
    ↓
persistenceGetSessionMessages() → NapiStoredMessage { role, content }
    ↓
handleResumeSelect() in AgentModal.tsx
    ↓
sessionRef.current.restoreMessages(messages)
    ↓
session.rs restore_messages() - creates text-only rig::Message
    ↓
LLM Context (❌ BROKEN - missing tool calls/results)
    ↓
Compaction (❌ BROKEN - no turn boundaries, wrong token counts)
```

---

## What Gets Lost: Detailed Analysis

### 1. Message Content Types

The `rig` library uses rich message structures that are currently flattened to text:

#### Assistant Messages (`rig::message::AssistantContent`)

| Content Type | Description | Currently Persisted |
|-------------|-------------|---------------------|
| `Text(TextContent)` | Plain text response | ✅ Yes |
| `ToolCall(ToolCall)` | Tool invocation with id, name, args | ❌ **NO** |
| `Thinking(ThinkingContent)` | Claude's reasoning (extended thinking) | ❌ **NO** |

#### User Messages (`rig::message::UserContent`)

| Content Type | Description | Currently Persisted |
|-------------|-------------|---------------------|
| `Text(TextContent)` | Plain text from user | ✅ Yes |
| `ToolResult(ToolResult)` | Tool execution result with id, content, is_error | ❌ **NO** |
| `Image(ImageContent)` | Images (base64 or URL) | ❌ **NO** |
| `Document(Document)` | Documents (PDFs, etc.) | ❌ **NO** |

### 2. Multi-Part Messages

A single assistant response can contain multiple content parts:

```
Assistant Message:
  [0] Text: "Let me read that file..."
  [1] ToolCall: { id: "tc_123", name: "read_file", args: { path: "/foo.ts" } }
  [2] Text: "I see the issue. Let me also check..."
  [3] ToolCall: { id: "tc_124", name: "grep", args: { pattern: "error" } }
```

Currently, this becomes: `"Let me read that file...I see the issue. Let me also check..."`

The tool calls are **completely lost**.

### 3. Tool Results

When a tool executes, the result comes back as a user message with `ToolResult` content:

```
User Message (Tool Result):
  [0] ToolResult: { tool_call_id: "tc_123", content: "file contents...", is_error: false }
  [1] ToolResult: { tool_call_id: "tc_124", content: "grep output...", is_error: false }
```

Currently, this becomes: `"[non-text content]"` or is skipped entirely.

### 4. Turn Boundaries

The session tracks "turns" for compaction:

```rust
// codelet-cli/src/session.rs
pub struct Session {
    pub messages: Vec<Message>,
    pub turns: Vec<Turn>,  // ← NOT PERSISTED OR RESTORED
    pub token_tracker: TokenTracker,
    // ...
}
```

A `Turn` represents a user→assistant exchange. Compaction uses turns to:
- Decide what can be summarized (old turns)
- Decide what must be kept verbatim (recent turns)
- Track boundaries for coherent summaries

Without turn restoration, compaction after resume will:
- Not know where exchanges begin/end
- Potentially summarize mid-conversation
- Produce incoherent summaries

### 5. Token Tracking

The `TokenTracker` is reset to zero on restore:

```rust
// session.rs restore_messages() - line 386
session.token_tracker = codelet_core::compaction::TokenTracker {
    input_tokens: 0,           // ❌ Should restore actual count
    output_tokens: 0,          // ❌ Should restore actual count
    cache_read_input_tokens: Some(0),
    cache_creation_input_tokens: Some(0),
};
```

This breaks compaction decisions because:
- Compaction triggers based on token thresholds
- A resumed session appears to have 0 tokens used
- Could lead to premature or delayed compaction

---

## Source Code Analysis

### Where Persistence Happens (Saving)

**File:** `src/tui/components/AgentModal.tsx`

```typescript
// Line 787-788: User message - text only
persistenceAppendMessage(currentSessionId, 'user', userMessage);

// Line 977-978: Assistant message - accumulated text only
persistenceAppendMessage(currentSessionId, 'assistant', fullAssistantResponse);
```

**Problem:** `fullAssistantResponse` only accumulates `chunk.text` (line 840). Tool calls received at line 853 and tool results at line 891 are displayed in UI but **never persisted**.

### Where Reading Happens (messages getter)

**File:** `codelet/napi/src/session.rs` (lines 269-316)

```rust
pub fn messages(&self) -> Result<Vec<Message>> {
    session.messages.iter().map(|msg| {
        match msg {
            rig::message::Message::User { content, .. } => {
                // Only extracts Text, everything else → "[non-text content]"
                content.iter().filter_map(|c| match c {
                    rig::message::UserContent::Text(t) => Some(t.text.clone()),
                    _ => None,  // ToolResult LOST
                })
            }
            rig::message::Message::Assistant { content, .. } => {
                // Only extracts Text, everything else → "[non-text content]"
                content.iter().filter_map(|c| match c {
                    rig::message::AssistantContent::Text(t) => Some(t.text.clone()),
                    _ => None,  // ToolCall, Thinking LOST
                })
            }
        }
    })
}
```

### Where Restoration Happens

**File:** `codelet/napi/src/session.rs` (lines 377-414)

```rust
pub fn restore_messages(&self, messages: Vec<Message>) -> Result<()> {
    // Resets token tracker to zero
    session.token_tracker = TokenTracker {
        input_tokens: 0, output_tokens: 0, ...
    };

    for msg in messages {
        let rig_msg = match msg.role.as_str() {
            "user" => RigMessage::User {
                // Creates TEXT-ONLY message
                content: OneOrMany::one(UserContent::text(&msg.content)),
            },
            "assistant" => RigMessage::Assistant {
                // Creates TEXT-ONLY message
                content: OneOrMany::one(AssistantContent::text(&msg.content)),
            },
            // Line 403-404: SKIPS tool messages entirely
            _ => continue,
        };
        session.messages.push(rig_msg);
    }
    // NOTE: session.turns is NOT restored - remains empty
}
```

---

## Impact Analysis

### 1. LLM Context Degradation

When a session is resumed, the LLM sees:

**Before (what actually happened):**
```
User: "Read the config file and tell me what port it uses"
Assistant: "Let me read the config file."
         [ToolCall: read_file("/etc/config.json")]
         [ToolResult: {"port": 8080, "host": "localhost"}]
         "The config uses port 8080."
```

**After Resume (what LLM sees):**
```
User: "Read the config file and tell me what port it uses"
Assistant: "Let me read the config file.The config uses port 8080."
```

The LLM has no idea a tool was called or what it returned. If the user asks "what else was in the config?", the LLM cannot answer accurately.

### 2. Compaction Failure

**Scenario:** Session has 50 turns, resumes, then compaction triggers.

**Expected behavior:** Compaction summarizes turns 1-40, keeps turns 41-50 verbatim.

**Actual behavior:** `session.turns` is empty after restore. Compaction may:
- Fail entirely (no turns to summarize)
- Treat all messages as a single turn
- Produce nonsensical summaries

### 3. Token Tracking Errors

**Scenario:** Session used 150k tokens before save. User resumes and continues.

**Expected:** Token tracker shows ~150k, compaction may trigger soon.

**Actual:** Token tracker shows 0. User continues until hitting context limit unexpectedly, with no warning or automatic compaction.

---

## Proposed Solution Architecture

### Design Goal: Match Claude Code's Storage Format

After analyzing Claude Code's session storage in `~/.claude/projects/{project-path}/{session-uuid}.jsonl`, we've designed a schema that captures ALL metadata needed for complete session restoration.

### 1. Message Envelope Wrapper

Every message MUST be wrapped in an envelope with metadata:

```json
{
  "uuid": "a6bdbefb-902d-4f98-b539-8cbee91ec831",
  "parentUuid": "81dc2799-ef52-4923-aa24-5798585aae57",
  "timestamp": "2025-12-23T08:51:44.813Z",
  "type": "assistant",
  "provider": "claude",
  "message": { ... },
  "requestId": "req_011CWPLKJZcigRWVriKduWSr"
}
```

**Envelope fields:**
- `uuid`: Unique identifier for this message
- `parentUuid`: UUID of the message this responds to (enables threading)
- `timestamp`: ISO 8601 timestamp when message was created
- `type`: Message type ("user", "assistant", "tool_result", "summary")
- `provider`: LLM provider that generated/received this message (see below)
- `message`: The actual message payload (see below)
- `requestId`: API request ID for debugging/tracing (format varies by provider)

**Provider types** (from `ProviderType` enum in `codelet/providers/src/manager.rs`):
- `claude`: Anthropic Claude API (includes OAuth mode)
- `openai`: OpenAI API
- `codex`: Codex/ChatGPT backend API
- `gemini`: Google Gemini API

Note: Provider is critical for multi-provider sessions where the user might switch providers mid-conversation. Each provider has different API response formats, and knowing the provider helps correctly interpret restored messages.

### 2. Assistant Message Schema

Assistant messages include rich metadata beyond just content:

```json
{
  "role": "assistant",
  "id": "msg_01Wk7SCqoakQaEmx7FHphZRJ",
  "model": "claude-opus-4-5-20251101",
  "content": [
    {"type": "text", "text": "Let me read that file..."},
    {"type": "tool_use", "id": "toolu_123", "name": "Read", "input": {"file_path": "/foo.ts"}},
    {"type": "text", "text": "I see the issue..."}
  ],
  "stop_reason": "end_turn",
  "stop_sequence": null,
  "usage": {
    "input_tokens": 15847,
    "output_tokens": 423,
    "cache_read_input_tokens": 12500,
    "cache_creation_input_tokens": 3000,
    "service_tier": "standard"
  }
}
```

**Key fields:**
- `id`: Anthropic message ID (msg_...) - CRITICAL for context continuity
- `model`: Exact model ID used (enables multi-model session analysis)
- `stop_reason`: "end_turn", "max_tokens", "stop_sequence", "tool_use"
- `usage`: Complete token breakdown including cache metrics

### 3. Content Type Schemas

#### Text Content
```json
{"type": "text", "text": "Plain text response"}
```

#### Tool Use (not "tool_call" - matches Anthropic API naming)
```json
{
  "type": "tool_use",
  "id": "toolu_01ABC123",
  "name": "Read",
  "input": {"file_path": "/path/to/file"}
}
```

#### Tool Result
```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_01ABC123",
  "content": "file contents here",
  "is_error": false
}
```

#### Tool Result with Raw Output Metadata
For tool results, we capture additional execution metadata:
```json
{
  "toolUseResult": {
    "stdout": "raw stdout from tool execution",
    "stderr": "raw stderr if any",
    "interrupted": false,
    "isImage": false
  }
}
```

#### Thinking Content
```json
{
  "type": "thinking",
  "thinking": "reasoning process here",
  "signature": "optional_signature_for_verification"
}
```

Note: `signature` is optional - thinking content without signatures must still be persisted.

#### Image Content
```json
{
  "type": "image",
  "source": {
    "type": "base64",
    "media_type": "image/png",
    "data": "base64-encoded-data"
  }
}
```

Or URL-based:
```json
{
  "type": "image",
  "source": {
    "type": "url",
    "url": "https://example.com/image.png"
  }
}
```

#### Document Content
```json
{
  "type": "document",
  "source": {
    "type": "base64",
    "media_type": "application/pdf",
    "data": "base64-encoded-data"
  },
  "title": "document.pdf",
  "context": "optional context about the document"
}
```

**DocumentSourceKind variants (from rig):**
- `Base64`: Binary data encoded as base64 (most common)
- `Url`: Remote URL reference
- `String`: Plain text content
- `Raw`: Raw bytes (must be encoded to Base64 for API)
- `Unknown`: Unrecognized format (preserve as-is)

### 4. Turn Boundaries

Turns can be derived from the message sequence:
- Each `user→assistant` exchange = 1 turn
- Tool results are part of the assistant turn that invoked them

Alternatively, store explicit turn boundaries:
```json
{
  "turns": [
    {"start_index": 0, "end_index": 1},
    {"start_index": 2, "end_index": 5},
    {"start_index": 6, "end_index": 7}
  ]
}
```

### 5. Token State Persistence

Token usage is tracked per-message via `usage` field. Session-level aggregates:
- Restore token tracker from sum of all message usage
- Or persist cumulative state in session manifest

---

## Files Requiring Changes

### Rust (codelet-napi)

1. **`codelet/napi/src/session.rs`**
   - `messages()` getter: Return structured content, not just text
   - `restore_messages()`: Parse structured content, restore all content types
   - `restore_messages()`: Restore token tracker state
   - New: `restore_turns()` or derive turns from messages

2. **`codelet/napi/src/types.rs`**
   - Update `Message` type to support structured content
   - Add types for `ToolCallContent`, `ToolResultContent`, etc.

3. **`codelet/napi/src/persistence/napi_bindings.rs`**
   - May need updates for structured message storage

### TypeScript (TUI)

1. **`src/tui/components/AgentModal.tsx`**
   - Persist tool calls when streaming (lines ~853-890)
   - Persist tool results when streaming (lines ~891-930)
   - Update restore handling for structured messages

### Rust (codelet-cli) - Potential

1. **`codelet-cli/src/session.rs`**
   - May need serialization helpers for Message types
   - Turn reconstruction logic

---

## Testing Strategy

### Unit Tests

1. **Serialization round-trip:** Message → JSON → Message for all content types
2. **Turn derivation:** Given message sequence, correctly identify turn boundaries
3. **Token restoration:** Verify token tracker state matches after restore

### Integration Tests

1. **Full session lifecycle:**
   - Start session
   - Make requests with tool calls
   - Save session
   - Create new session
   - Restore saved messages
   - Verify LLM has full context
   - Continue conversation referencing tools

2. **Compaction after restore:**
   - Create session with many turns
   - Save and restore
   - Trigger compaction
   - Verify compaction works correctly

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Storage size increase | Medium | Low | Use blob storage for content >10KB |
| Performance impact | Low | Medium | Benchmark serialization; optimize if needed |

---

## Appendix: rig Message Type Reference

From `rig` crate source (codelet/patches/rig-core/src/completion/message.rs):

```rust
pub enum Message {
    User { content: OneOrMany<UserContent> },
    Assistant {
        id: Option<String>,  // Anthropic message ID (msg_...) - MUST be persisted
        content: OneOrMany<AssistantContent>
    },
}

pub enum UserContent {
    Text(TextContent),
    ToolResult(ToolResult),
    Image(ImageContent),
    Document(Document),
}

pub enum AssistantContent {
    Text(TextContent),
    ToolCall(ToolCall),
    Thinking(ThinkingContent),
}

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}

pub struct ToolResult {
    pub id: String,           // tool_call_id this responds to
    pub content: String,      // result content
    pub is_error: bool,       // whether this was an error
}

pub struct ThinkingContent {
    pub thinking: String,
    pub signature: Option<String>,  // Optional - not all thinking has signatures
}

// Image handling
pub struct ImageContent {
    pub image: Image,
}

pub struct Image {
    pub source: ImageSource,
    pub media_type: ImageMediaType,
}

pub enum ImageSource {
    Base64(String),
    Url(String),
}

pub enum ImageMediaType {
    Jpeg,
    Png,
    Gif,
    Webp,
}

// Document handling
pub struct Document {
    pub source: DocumentSource,
    pub media_type: DocumentMediaType,
    pub title: Option<String>,
    pub context: Option<String>,
}

pub enum DocumentSourceKind {
    Base64,   // Binary data as base64 string
    Url,      // Remote URL reference
    Raw,      // Raw bytes (Vec<u8>) - must be encoded before API
    String,   // Plain text content
    Unknown,  // Unrecognized format - preserve as-is
}

pub enum DocumentMediaType {
    Pdf,
    PlainText,
    Markdown,
    Csv,
    Docx,
    Xlsx,
    Pptx,
}
```

### Key Implementation Notes

1. **Assistant Message ID**: The `id` field in `Message::Assistant` holds the Anthropic message ID (`msg_...`). This MUST be captured during streaming and persisted. Current implementation sets this to `None` on restore.

2. **DocumentSourceKind::Raw**: Raw bytes are not directly supported by the Anthropic API. When encountering Raw source, it must be base64-encoded before sending to API or persisting.

3. **Thinking Signatures**: The `signature` field in `ThinkingContent` is optional. Thinking content without signatures (e.g., from older API versions or certain model configurations) must still be persisted.

4. **Media Types**: Both images and documents require explicit media types. The rig library provides enum variants that map to MIME types (e.g., `ImageMediaType::Png` → `image/png`).
