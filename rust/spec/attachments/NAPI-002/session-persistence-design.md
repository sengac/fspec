# Session Persistence, Fork, and Merge System

## Executive Summary

This document describes a comprehensive session management system for codelet that enables:

1. **Context Window Persistence** - Save and restore conversation context to disk
2. **Command History** - Arrow up/down navigation through previous user inputs
3. **Session Forking** - Duplicate a conversation at any point to explore alternatives
4. **Session Merging** - Import messages from one session into another
5. **Session Resume** - Continue interrupted conversations

The architecture follows a **git-like model** where messages are immutable objects stored once, and sessions are ordered views (manifests) that reference those messages.

---

## Problem Statement

### Current State

The codelet `Session` struct holds conversation state **in memory only**:

```rust
pub struct Session {
    provider_manager: ProviderManager,
    pub messages: Vec<rig::message::Message>,  // Lost on exit
    pub turns: Vec<ConversationTurn>,          // Lost on exit
    pub token_tracker: TokenTracker,           // Lost on exit
}
```

**Problems:**
- Closing the terminal loses all conversation history
- Cannot resume interrupted sessions
- Cannot explore "what if" scenarios by forking
- Cannot import useful context from other conversations
- No arrow up/down for command history

### Desired State

Users should be able to:
1. Close terminal and resume later exactly where they left off
2. Fork a conversation to try a different approach
3. Merge insights from one conversation into another
4. Navigate previous inputs with arrow keys
5. List and switch between past sessions

---

## Architecture Overview

### Core Principle: Git for Conversations

| Git Concept | Conversation Equivalent |
|-------------|------------------------|
| Commit | Message (immutable) |
| Branch | Session (ordered list of message refs) |
| Repository | Message Store (content-addressed) |
| HEAD/ref | Session Manifest |
| Fork | Create new manifest sharing messages up to point X |
| Cherry-pick | Import specific messages with context |
| Merge | Combine message references |

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                            TypeScript Layer                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │   REPL UI   │  │  Commands   │  │   Hooks     │  │   TUI       │    │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘    │
│         │                │                │                │            │
│         └────────────────┴────────────────┴────────────────┘            │
│                                   │                                      │
│                          NAPI Bindings                                   │
└──────────────────────────────────┬──────────────────────────────────────┘
                                   │
┌──────────────────────────────────┴──────────────────────────────────────┐
│                              Rust Layer                                  │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      SessionManager                              │   │
│  │  - create_session()    - fork_session()    - add_history()      │   │
│  │  - load_session()      - merge_messages()  - get_history()      │   │
│  │  - append_message()    - cherry_pick()     - search_history()   │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                          │                │                             │
│              ┌───────────┴───┐    ┌───────┴───────┐                    │
│              ▼               ▼    ▼               ▼                    │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐        │
│  │  MessageStore   │  │ ManifestStore   │  │  HistoryStore   │        │
│  │ (content-addr)  │  │ (per-session)   │  │  (global)       │        │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘        │
│           │                    │                    │                  │
└───────────┴────────────────────┴────────────────────┴──────────────────┘
            │                    │                    │
            ▼                    ▼                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           File System                                    │
│                                                                          │
│  ~/.fspec/                                                              │
│  ├── history.jsonl              # Command history (arrow up/down)       │
│  ├── messages/                  # Content-addressed message store       │
│  │   ├── a1/a1b2c3...json                                              │
│  │   └── .../                                                           │
│  ├── sessions/                                                          │
│  │   ├── index.json             # Quick session listing                 │
│  │   └── by-project/                                                    │
│  │       └── <hash>/                                                    │
│  │           └── <uuid>.json    # Session manifests                     │
│  └── blobs/                     # Large content (file reads)            │
│      └── <sha256>.dat                                                   │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Data Model

### StoredMessage

Messages are **immutable** once written. They are stored in a content-addressed store, meaning identical messages are stored only once.

```rust
/// A single message in the conversation, stored immutably
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredMessage {
    /// Unique identifier for this message
    pub id: Uuid,

    /// SHA-256 hash of content for deduplication
    pub content_hash: String,

    /// When this message was created
    pub created_at: DateTime<Utc>,

    /// Who sent this message
    pub role: MessageRole,

    /// The actual content (may reference blob hashes for large content)
    pub content: MessageContent,

    /// Estimated token count for context window tracking
    pub token_count: Option<u32>,

    /// Provider-specific metadata (model used, stop reason, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageContent {
    /// Plain text content
    Text(String),

    /// Content with tool calls/results
    Structured {
        text: Option<String>,
        tool_calls: Vec<ToolCall>,
        tool_results: Vec<ToolResult>,
    },

    /// Reference to blob for large content (file reads, etc.)
    BlobRef {
        hash: String,
        preview: String,  // First N chars for display
        size_bytes: u64,
    },
}
```

### SessionManifest

Sessions are **ordered lists of message references**. They don't store messages directly - they reference them by ID.

```rust
/// Session manifest - defines a conversation as ordered message references
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SessionManifest {
    /// Unique session identifier
    pub id: Uuid,

    /// Human-readable name (auto-generated or user-provided)
    pub name: String,

    /// Project directory this session belongs to
    pub project: PathBuf,

    /// Provider used (claude, openai, etc.)
    pub provider: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub updated_at: DateTime<Utc>,

    /// Ordered list of message references
    pub messages: Vec<MessageRef>,

    /// If this session was forked from another
    pub forked_from: Option<ForkPoint>,

    /// Record of merges from other sessions
    pub merged_from: Vec<MergeRecord>,

    /// Current compaction state (if compacted)
    pub compaction: Option<CompactionState>,

    /// Token usage statistics
    pub token_usage: TokenUsage,
}

/// Reference to a message in the store
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MessageRef {
    /// The message ID in the message store
    pub message_id: Uuid,

    /// Where this message came from
    pub source: MessageSource,
}

/// Origin of a message reference
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageSource {
    /// Created natively in this session
    Native,

    /// Inherited from fork parent
    Forked { from_session: Uuid },

    /// Imported via merge/cherry-pick
    Imported {
        from_session: Uuid,
        original_index: usize,
    },
}

/// Information about fork lineage
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ForkPoint {
    /// Session this was forked from
    pub source_session_id: Uuid,

    /// Index of last message included in fork
    pub fork_after_index: usize,

    /// When the fork occurred
    pub forked_at: DateTime<Utc>,
}

/// Record of a merge operation
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MergeRecord {
    /// Session messages were merged from
    pub source_session_id: Uuid,

    /// Which message indices were imported
    pub source_indices: Vec<usize>,

    /// Where they were inserted (None = appended)
    pub inserted_at: Option<usize>,

    /// When the merge occurred
    pub merged_at: DateTime<Utc>,
}

/// Compaction state tracking
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompactionState {
    /// Summary of compacted messages
    pub summary: String,

    /// Messages before this index were compacted
    pub compacted_before_index: usize,

    /// When compaction occurred
    pub compacted_at: DateTime<Utc>,
}

/// Token usage tracking
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct TokenUsage {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
}
```

### HistoryEntry

Command history for arrow up/down navigation.

```rust
/// Entry in the command history
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HistoryEntry {
    /// Unique identifier
    pub id: Uuid,

    /// The text the user entered
    pub text: String,

    /// When this was entered
    pub timestamp: DateTime<Utc>,

    /// Which project directory
    pub project: PathBuf,

    /// Which session this was part of
    pub session_id: Uuid,

    /// Any pasted content (stored separately if large)
    pub pasted_content: Option<PastedContent>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PastedContent {
    /// Small pasted content, stored inline
    Inline(String),

    /// Large pasted content, stored as blob
    BlobRef { hash: String, size_bytes: u64 },
}
```

---

## File System Layout

```
~/.fspec/
├── history.jsonl                         # Global command history
│   # Format: one JSON object per line
│   # {"id":"uuid","text":"user input","timestamp":"...","project":"/path","session_id":"uuid"}
│
├── messages/                             # Content-addressed message store
│   ├── a1/                              # Sharded by first 2 chars of hash
│   │   ├── a1b2c3d4e5f6...json         # Individual message
│   │   └── a1f9e8d7c6b5...json
│   ├── b2/
│   │   └── b2a1c3d4e5f6...json
│   └── .../
│
├── sessions/
│   ├── index.json                        # Quick listing for all sessions
│   │   # [{"id":"uuid","name":"...","project":"/path","updated_at":"...","message_count":42}]
│   │
│   └── by-project/
│       ├── 0a1b2c3d/                    # Hash of project path
│       │   ├── abc123.json              # Session manifest
│       │   └── def456.json              # Session manifest
│       └── 4e5f6a7b/
│           └── 789abc.json
│
└── blobs/                                # Large content storage
    ├── sha256-a1b2c3...dat              # File content, tool results, etc.
    └── sha256-d4e5f6...dat
```

---

## Operations

### 1. Create Session

**When**: User starts a new conversation (or first message in a project).

```rust
pub fn create_session(
    project: &Path,
    provider: &str,
    name: Option<&str>,
) -> Result<Uuid>;
```

**Example**:
```
User starts codelet in /Users/dev/myproject
→ create_session("/Users/dev/myproject", "claude", None)
→ Returns: "550e8400-e29b-41d4-a716-446655440000"
→ Creates: ~/.fspec/sessions/by-project/<hash>/550e8400-e29b-41d4-a716-446655440000.json
```

### 2. Append Message

**When**: User sends a message or assistant responds.

```rust
pub fn append_message(
    session_id: Uuid,
    message: &Message,
) -> Result<Uuid>;  // Returns message ID
```

**Example**:
```
User: "How do I implement a binary search?"

→ append_message(session_id, Message::User { content: "How do I implement..." })
→ Stores message in ~/.fspec/messages/a1/a1b2c3...json
→ Adds MessageRef to session manifest
→ Appends to ~/.fspec/history.jsonl
→ Returns message ID
```

### 3. Load Session

**When**: Resuming a conversation.

```rust
pub fn load_session(session_id: Uuid) -> Result<LoadedSession>;

pub struct LoadedSession {
    pub manifest: SessionManifest,
    pub messages: Vec<StoredMessage>,
}
```

**Example**:
```
User runs: codelet --resume

→ get_last_session("/Users/dev/myproject")
→ Returns: "550e8400-..."
→ load_session("550e8400-...")
→ Returns manifest + all messages
→ Session continues from where it left off
```

### 4. Fork Session

**When**: User wants to try a different approach from a specific point.

```rust
pub fn fork_session(
    source_session_id: Uuid,
    fork_after_index: usize,
    new_name: Option<&str>,
) -> Result<Uuid>;  // Returns new session ID
```

**Example**:
```
Session A has 10 messages. User wants to try a different approach from message 5.

Command: /fork 5 "Alternative approach"

→ fork_session("session-a-id", 5, Some("Alternative approach"))
→ Creates new session B with messages 0-5 (references, not copies)
→ Session B can now diverge independently
→ Returns: "new-session-b-id"

Visualization:
Session A: [M0] → [M1] → [M2] → [M3] → [M4] → [M5] → [M6] → [M7] → [M8] → [M9]
                                         ↑
                                    fork point

Session B: [M0] → [M1] → [M2] → [M3] → [M4] → [M5]  (refs to same messages)
                                               ↓
                                             [M10] → [M11]  (new, divergent)
```

### 5. Merge Messages

**When**: User wants to import useful context from another session.

```rust
pub fn merge_messages(
    target_session_id: Uuid,
    source_session_id: Uuid,
    source_indices: &[usize],
    insert_at: Option<usize>,  // None = append
) -> Result<()>;
```

**Example**:
```
Session A: Working on feature X
Session B: Previously solved authentication issue (messages 3-4 have the solution)

Command: /merge session-b 3,4

→ merge_messages("session-a", "session-b", &[3, 4], None)
→ Imports messages 3 and 4 from session B into session A
→ Messages are appended (or inserted at specified position)

Before:
Session A: [A0] → [A1] → [A2]
Session B: [B0] → [B1] → [B2] → [B3] → [B4] → [B5]
                                  ↑      ↑
                              these two

After:
Session A: [A0] → [A1] → [A2] → [B3*] → [B4*]
                                  ↑       ↑
                          (source: session-b)
```

### 6. Cherry-Pick

**When**: User wants to import a message with its preceding context.

```rust
pub fn cherry_pick(
    target_session_id: Uuid,
    source_session_id: Uuid,
    message_index: usize,
    context_count: usize,
) -> Result<()>;
```

**Example**:
```
Session B has a great answer at message 7, but it only makes sense with the question at message 6.

Command: /cherry-pick session-b 7 --context 1

→ cherry_pick("session-a", "session-b", 7, 1)
→ Imports messages 6 AND 7 (the question-answer pair)

Before:
Session B: [...] → [B6: "How do I...?"] → [B7: "Here's how..."]
                           ↑                      ↑
                       context=1              target

After Session A:
[...existing...] → [B6*] → [B7*]
```

### 7. List Sessions

**When**: User wants to see available sessions.

```rust
pub fn list_sessions(
    project: Option<&Path>,
    limit: Option<usize>,
) -> Result<Vec<SessionSummary>>;

pub struct SessionSummary {
    pub id: Uuid,
    pub name: String,
    pub project: PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: usize,
    pub forked_from: Option<Uuid>,
}
```

**Example**:
```
Command: /sessions

→ list_sessions(Some("/Users/dev/myproject"), None)
→ Returns:
  1. "Implementing auth flow" (12 messages, 2 hours ago)
  2. "Alternative approach" (8 messages, 1 hour ago) [forked from #1]
  3. "Debugging tests" (5 messages, 30 min ago)
```

### 8. Command History

**When**: User presses arrow up/down.

```rust
pub fn add_history(
    text: &str,
    project: &Path,
    session_id: Uuid,
) -> Result<()>;

pub fn get_history(
    project: Option<&Path>,
    limit: Option<usize>,
) -> Result<Vec<HistoryEntry>>;

pub fn search_history(
    query: &str,
    project: Option<&Path>,
) -> Result<Vec<HistoryEntry>>;
```

**Example**:
```
User types "implement" and presses Ctrl+R (reverse search)

→ search_history("implement", Some("/Users/dev/myproject"))
→ Returns matching history entries:
  - "How do I implement a binary search?"
  - "Implement the user authentication flow"
  - "Can you implement this interface?"
```

---

## NAPI Interface

All operations are exposed to TypeScript via NAPI bindings:

```typescript
// Type definitions
export interface StoredMessage {
  id: string;
  contentHash: string;
  createdAt: string;
  role: 'user' | 'assistant' | 'system';
  content: MessageContent;
  tokenCount?: number;
  metadata: Record<string, unknown>;
}

export interface SessionManifest {
  id: string;
  name: string;
  project: string;
  provider: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  forkedFrom?: ForkPoint;
  mergedFrom: MergeRecord[];
}

export interface SessionSummary {
  id: string;
  name: string;
  project: string;
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  forkedFrom?: string;
}

export interface HistoryEntry {
  id: string;
  text: string;
  timestamp: string;
  project: string;
  sessionId: string;
}

export interface LoadedSession {
  manifest: SessionManifest;
  messages: StoredMessage[];
}

// SessionManager class exposed via NAPI
export class SessionManager {
  // === Session Lifecycle ===

  /**
   * Create a new session for a project
   * @returns Session ID (UUID)
   */
  createSession(project: string, provider: string, name?: string): string;

  /**
   * Load a session with all its messages
   */
  loadSession(sessionId: string): LoadedSession;

  /**
   * List sessions, optionally filtered by project
   */
  listSessions(project?: string, limit?: number): SessionSummary[];

  /**
   * Get the most recent session for a project
   */
  getLastSession(project: string): string | null;

  /**
   * Delete a session and optionally its orphaned messages
   */
  deleteSession(sessionId: string, cleanupOrphans?: boolean): void;

  // === Message Operations ===

  /**
   * Append a message to a session
   * @returns Message ID (UUID)
   */
  appendMessage(sessionId: string, message: Message): string;

  /**
   * Get all messages for a session (resolved from refs)
   */
  getMessages(sessionId: string): StoredMessage[];

  // === Fork ===

  /**
   * Fork a session at a specific message index
   * @param sourceSessionId - Session to fork from
   * @param forkAfterIndex - Include messages 0..=forkAfterIndex
   * @param newName - Optional name for forked session
   * @returns New session ID
   */
  forkSession(
    sourceSessionId: string,
    forkAfterIndex: number,
    newName?: string
  ): string;

  // === Merge ===

  /**
   * Import messages from one session into another
   * @param targetSessionId - Session to import into
   * @param sourceSessionId - Session to import from
   * @param messageIndices - Which messages to import (by index)
   * @param insertAt - Where to insert (undefined = append)
   */
  mergeMessages(
    targetSessionId: string,
    sourceSessionId: string,
    messageIndices: number[],
    insertAt?: number
  ): void;

  /**
   * Import a message with preceding context
   * @param contextCount - Number of preceding messages to include
   */
  cherryPick(
    targetSessionId: string,
    sourceSessionId: string,
    messageIndex: number,
    contextCount: number
  ): void;

  // === Lineage ===

  /**
   * Get the fork/merge lineage of a session
   */
  getSessionLineage(sessionId: string): SessionLineage;

  // === History (Arrow Up/Down) ===

  /**
   * Add an entry to command history
   */
  addHistory(text: string, project: string, sessionId: string): void;

  /**
   * Get recent history, optionally filtered by project
   */
  getHistory(project?: string, limit?: number): HistoryEntry[];

  /**
   * Search history by query string
   */
  searchHistory(query: string, project?: string): HistoryEntry[];

  // === Maintenance ===

  /**
   * Clean up orphaned messages (not referenced by any session)
   */
  cleanupOrphanedMessages(): number;  // Returns count deleted

  /**
   * Get storage statistics
   */
  getStorageStats(): StorageStats;
}
```

---

## User-Facing Commands

These commands would be exposed in the REPL:

| Command | Description | Example |
|---------|-------------|---------|
| `/sessions` | List available sessions | `/sessions` |
| `/session <id>` | Switch to a session | `/session abc123` |
| `/session-info` | Show current session details | `/session-info` |
| `/fork [index] [name]` | Fork current session | `/fork 5 "Try different approach"` |
| `/merge <session> <indices>` | Import messages | `/merge abc123 3,4,5` |
| `/cherry-pick <session> <index> [--context N]` | Import with context | `/cherry-pick abc123 7 --context 2` |
| `/rename <name>` | Rename current session | `/rename "Auth implementation"` |
| `/delete-session <id>` | Delete a session | `/delete-session abc123` |
| `/history` | Show command history | `/history` |
| `/history search <query>` | Search history | `/history search "implement"` |

---

## Concrete Examples

### Example 1: Resume After Closing Terminal

```
# Day 1: Working on a feature
$ codelet
> How do I implement rate limiting in Rust?
[... conversation continues for 20 messages ...]
> ^D  # Close terminal

# Day 2: Resume
$ codelet
[Resuming session "Rate limiting implementation" (20 messages)]
> Where were we?
[Assistant has full context from Day 1]
```

### Example 2: Fork to Try Alternative Approach

```
> Let's implement this with a HashMap
[... 5 messages implementing HashMap approach ...]
> Actually, let me try a different approach

> /fork 3 "BTreeMap approach"
[Created session "BTreeMap approach" forked from message 3]

> Let's try BTreeMap instead
[Now in forked session, can explore alternative]

> /sessions
1. "Rate limiting implementation" (8 messages)
2. "BTreeMap approach" (4 messages) [forked from #1]
```

### Example 3: Merge Solution from Another Session

```
# Session A: Working on new feature
> I need to handle authentication here

# Session B (earlier): Has auth solution at messages 15-18

> /merge session-b 15,16,17,18
[Imported 4 messages from "Auth implementation"]

> Now I have that auth context available
[Assistant can reference the imported auth solution]
```

### Example 4: Cherry-Pick with Context

```
# Session B has: [... question about caching ...] → [... detailed caching solution ...]
#                       index 12                          index 13

# I want the solution, but need the question for context
> /cherry-pick session-b 13 --context 1
[Imported messages 12-13 from "Caching exploration"]

# Now session A has the Q&A pair for context
```

### Example 5: Arrow Up/Down History

```
> How do I implement a binary search?
[... response ...]

> Can you show me a recursive version?
[... response ...]

> [Press Up Arrow]
# Shows: "Can you show me a recursive version?"

> [Press Up Arrow again]
# Shows: "How do I implement a binary search?"

> [Press Ctrl+R, type "recursive"]
# Searches history for "recursive"
# Shows: "Can you show me a recursive version?"
```

---

## Integration with Existing Systems

### Compaction Integration

When context compaction occurs, the session manifest is updated:

```rust
pub fn record_compaction(
    session_id: Uuid,
    summary: &str,
    compacted_before_index: usize,
) -> Result<()>;
```

The manifest's `compaction` field tracks what was compacted, allowing intelligent context reconstruction.

### Provider Switching

When provider switches, a new session is typically created (different context window sizes, different capabilities). The fork mechanism can preserve context:

```rust
// Before switching from Claude to OpenAI
fork_session(current_session, last_message_index, "Continued on OpenAI")
```

### System Reminders

System reminders are stored as regular messages with `role: System` and appropriate metadata tags. They're preserved through fork/merge operations.

---

## Implementation Phases

### Phase 1: Core Storage
- [ ] MessageStore implementation (content-addressed)
- [ ] SessionManifest implementation
- [ ] Basic CRUD operations
- [ ] NAPI bindings for create/load/append

### Phase 2: History
- [ ] HistoryStore implementation
- [ ] Arrow up/down integration
- [ ] History search
- [ ] NAPI bindings

### Phase 3: Fork/Merge
- [ ] fork_session implementation
- [ ] merge_messages implementation
- [ ] cherry_pick implementation
- [ ] Lineage tracking

### Phase 4: Commands
- [ ] /sessions command
- [ ] /fork command
- [ ] /merge command
- [ ] /cherry-pick command

### Phase 5: Polish
- [ ] Orphan cleanup
- [ ] Storage statistics
- [ ] Session naming heuristics
- [ ] UI integration

---

## Design Decisions

### Why Content-Addressed Messages?

1. **No duplication on fork** - Forked sessions reference same message objects
2. **Efficient merge** - Just add references, don't copy content
3. **Deduplication** - Identical messages stored once
4. **Integrity** - Hash verification

### Why Separate Manifests from Messages?

1. **Lightweight operations** - Fork/merge just manipulate refs
2. **Fast listing** - Don't need to read all messages
3. **Clean lineage** - Manifests track fork/merge history
4. **Independent lifecycle** - Messages immutable, manifests mutable

### Why JSONL for History?

1. **Append-only** - Fast writes, crash-safe
2. **Streaming** - Can read without loading entire file
3. **Compatible** - Same format as Claude Code

### Why Blob Storage for Large Content?

1. **Keep messages small** - File reads don't bloat message store
2. **Deduplication** - Same file content stored once
3. **Lazy loading** - Only load blobs when needed

---

## Future Considerations

### Potential Enhancements

1. **Session search** - Full-text search across all sessions
2. **Session export** - Export to markdown/JSON for sharing
3. **Session import** - Import from Claude Code format
4. **Collaborative sessions** - Multiple users in same session
5. **Session templates** - Pre-configured starting contexts
6. **Auto-naming** - LLM-generated session names based on content

### Not In Scope (For Now)

1. Cross-device sync
2. Encryption at rest
3. Session sharing/publishing
4. Real-time collaboration
