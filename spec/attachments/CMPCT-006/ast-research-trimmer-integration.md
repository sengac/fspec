# AST Research: Trimmer Integration Points

## StoredMessage (Input Type)
- **Location**: `codelet/napi/src/persistence/types.rs:62`
- **Fields**: `id: Uuid`, `content_hash: String`, `created_at: DateTime<Utc>`, `role: String`, `content: String`, `token_count: Option<u32>`, `blob_refs: Vec<String>`, `metadata: HashMap<String, serde_json::Value>`
- **Key insight**: `metadata` contains the full serialized `MessageEnvelope`. The envelope has the message payload with content blocks.

## Message Envelope (Metadata Structure)
- **Location**: `codelet/napi/src/persistence/message_envelope.rs`
- **MessagePayload**: Untagged enum — `User(UserMessage)` or `Assistant(AssistantMessage)`
- **Metadata keys**: `uuid`, `parentUuid`, `timestamp`, `type`, `provider`, `message`, `requestId`
- **The `message` key** contains the serialized `UserMessage` or `AssistantMessage`

## UserContent (Tool Results)
- **Location**: `codelet/napi/src/persistence/message_envelope.rs:74`
- **Variants**:
  - `Text { text: String }` — plain text
  - `ToolResult { tool_use_id: String, content: String, is_error: bool, tool_use_result: Option<ToolUseResultMetadata> }` — tool output
  - `Image { source: ImageSource }` — base64 or URL image
  - `Document { source: DocumentSource, title, context, cache_control }` — documents
- **Note**: ToolResult has `tool_use_id` but NOT the tool name. Need correlation with assistant ToolUse.
- **ToolUseResultMetadata** has: `stdout`, `stderr`, `interrupted`, `is_image`

## AssistantContent (Tool Use Requests)
- **Location**: `codelet/napi/src/persistence/message_envelope.rs:133`
- **Variants**:
  - `Text { text: String }` — reasoning text
  - `ToolUse { id: String, name: String, input: serde_json::Value }` — tool call with name and parameters
  - `Thinking { thinking: String, signature: Option<String> }` — extended thinking

## resolve_message_content (Content Resolution)
- **Location**: `codelet/napi/src/session_search_handler.rs:411`
- Resolves blob references in `content` and `blob_refs`
- For assistant messages with large tool inputs: blob_refs may contain tool use input data
- Returns the full resolved content string

## Content Summary Format
- User text: full text
- User tool result: truncated to 200 chars
- User image: `[image]`
- Assistant text: full text
- Assistant tool use: `[tool_use: {name}]`
- For blob-stored content: `resolve_message_content` joins content + resolved blobs with newlines

## Blob Processing for ToolUse
- **Location**: `codelet/napi/src/persistence/blob_processing.rs:107`
- Large tool use inputs are stored as blobs with keys like `tool_use:{idx}:{id}`
- Blob refs are stored in metadata under `_blobRefs` key

## Session Search Handler Flow (CMPCT-010 Integration Point)
- **handle_show()**: Iterates messages → `resolve_message_content()` → reassemble_content (assistant only) → truncate → SessionMessage
- **handle_search()**: Iterates messages → `resolve_message_content()` → regex match → extract preview → SearchMatch
- Trimmer will be called after `resolve_message_content()` and before reassembly/truncation

## Cross-Crate Dependency Constraint
- `codelet-core` CANNOT depend on `codelet-napi` (would create circular dependency)
- Trimmer API must use primitive types: `(role: &str, content: &str, metadata: &HashMap<String, Value>)`
- Trimmer parses metadata internally to extract tool information

## Tool Name Correlation Strategy
- Assistant messages: tool name available directly from `metadata["message"]["content"][].name`
- User tool result messages: only `tool_use_id` available, need correlation with preceding assistant message
- Stateful trimmer: register tool_use_id → tool_name when processing assistant messages, look up when processing user tool results
- Fallback: content-based heuristics for uncorrelated tool results

## Compaction Module (Target Location)
- **Location**: `codelet/core/src/compaction/mod.rs`
- Current exports: anchor, compactor, metrics, model, selector
- Trimmer will be added as `pub mod trimmer; pub use trimmer::Trimmer;`
