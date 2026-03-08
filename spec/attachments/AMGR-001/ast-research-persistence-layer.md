# AST Research: Persistence Layer & Tool Pattern Analysis for SessionSearch

**Date:** 2026-03-08
**Work Unit:** AMGR-001

## 1. Persistence Layer API (codelet/napi/src/persistence/)

### Module Structure
```
codelet/napi/src/persistence/
├── mod.rs           — High-level API functions (global singletons)
├── storage.rs       — MessageStore, SessionStore (file-backed with in-memory cache)
├── history.rs       — HistoryStore (JSONL append-only, in-memory search)
├── blob.rs          — BlobStore (SHA-256 content-addressed storage)
├── blob_processing.rs — Blob reference resolution (BLOB_REF_PREFIX = "blob:sha256:")
├── types.rs         — SessionManifest, StoredMessage, HistoryEntry, etc.
├── message_envelope.rs — MessageEnvelope format for structured messages
├── napi_bindings.rs — NAPI exports to TypeScript
└── tests.rs         — 90KB of tests
```

### Key Public APIs for SessionSearch

#### Session Access
- `load_session(id: Uuid) -> Result<SessionManifest, String>` — Load by UUID
- `list_sessions(project: &Path) -> Result<Vec<SessionManifest>, String>` — List for project (pub(crate))
- `SessionStore::list_all() -> Vec<&SessionManifest>` — All sessions across all projects
- `SessionStore::list_for_project(project: &Path) -> Vec<&SessionManifest>` — Filter by project

#### Message Access
- `get_message(id: Uuid) -> Result<Option<StoredMessage>, String>` — Single message by ID
- `get_session_messages_full(session: &SessionManifest) -> Result<Vec<StoredMessage>, String>` — All messages ignoring compaction
- `get_session_messages(session: &SessionManifest) -> Result<Vec<StoredMessage>, String>` — Respects compaction

#### History Search
- `search_history(query: &str, project: Option<&Path>) -> Result<Vec<HistoryEntry>, String>` — Text search (case-insensitive contains)
- `get_history(project: Option<&Path>, limit: Option<usize>) -> Result<Vec<HistoryEntry>, String>` — Recent entries

#### Blob Resolution
- `get_blob(hash: &str) -> Result<Vec<u8>, String>` — Fetch blob by SHA-256 hash
- `is_blob_reference(s: &str) -> bool` — Check if string is "blob:sha256:..."
- `extract_blob_hash(s: &str) -> Option<&str>` — Extract hash from reference
- `rehydrate_envelope_blobs(envelope_json: &str) -> Result<String, String>` — Full blob resolution

### Key Types

#### SessionManifest
```rust
pub struct SessionManifest {
    pub id: Uuid,
    pub name: String,
    pub project: PathBuf,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<MessageRef>,
    pub compaction: Option<CompactionState>,
    pub token_usage: TokenUsage,
    // ... forked_from, merged_from, anchor_points
}
```

#### StoredMessage
```rust
pub struct StoredMessage {
    pub id: Uuid,
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
    pub role: String,       // "user" or "assistant"
    pub content: String,    // text or blob reference
    pub token_count: Option<u32>,
    pub blob_refs: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}
```

#### HistoryEntry
```rust
pub struct HistoryEntry {
    pub display: String,
    pub timestamp: DateTime<Utc>,
    pub project: PathBuf,
    pub session_id: Uuid,
    pub pasted_content: Option<PastedContent>,
}
```

### Global Singletons Pattern
```rust
lazy_static! {
    static ref MESSAGE_STORE: Mutex<Option<MessageStore>> = Mutex::new(None);
    static ref SESSION_STORE: Mutex<Option<SessionStore>> = Mutex::new(None);
    static ref BLOB_STORE: Mutex<Option<BlobStore>> = Mutex::new(None);
    static ref HISTORY_STORE: Mutex<Option<HistoryStore>> = Mutex::new(None);
}
```
All accessed via `init_stores()` + lock pattern.

## 2. Tool Registration Pattern

### How Tools are Wired (claude.rs example)
```rust
pub fn create_rig_agent(&self, session_id: Uuid, ...) -> Agent<...> {
    let mut agent_builder = self.rig_client
        .agent(&self.model_name)
        .tool(ReadTool::new(session_id))
        .tool(WriteTool::new(session_id))
        .tool(EditTool::new(session_id))
        .tool(BashTool::new(session_id))
        // ... all tools take session_id
        .tool(ConnectMcpTool::new(session_id));
}
```

### Simple Tool Pattern (ReadTool)
```rust
pub struct ReadTool { session_id: Uuid }
impl ReadTool { pub fn new(session_id: Uuid) -> Self { Self { session_id } } }
```

### Action-Based Tool Pattern (BridgeTool)
```rust
#[derive(Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeAction {
    Connect { url: String },
    Disconnect { url: String },
    List,
}

#[derive(Deserialize, Serialize)]
pub struct BridgeToolArgs { pub action: BridgeAction }

impl Tool for BridgeTool {
    const NAME: &'static str = "Bridge";
    type Args = BridgeToolArgs;
    type Output = BridgeResult;
    // ...
}
```

## 3. lib.rs Exports (codelet/tools/src/lib.rs)

All tools are declared as `pub mod` and re-exported with `pub use`:
- Modules: `astgrep`, `bash`, `bridge`, `edit`, `fspec`, `glob`, `grep`, `ls`, `mcp`, `read`, `web_search`, `write`
- Tool types: `AstGrepTool`, `BashTool`, `BridgeTool`, `EditTool`, `FspecTool`, `GlobTool`, `GrepTool`, `LsTool`, `ReadTool`, `WriteTool`, `ConnectMcpTool`, `WebSearchTool`

## 4. Provider Registration (5 providers)

All providers follow the same pattern in `create_rig_agent()`:
- `codelet/providers/src/claude.rs`
- `codelet/providers/src/openai.rs`
- `codelet/providers/src/gemini.rs`
- `codelet/providers/src/codex/mod.rs`
- `codelet/providers/src/zai.rs`

SessionSearch tool needs to be added to all 5.

## 5. Streaming Chunk Reassembly (from session-search.sh)

The Python logic in session-search.sh handles:
- `[Thinking: <text>...]` — thinking chunks (may lack closing bracket)
- `[Tool: <name>]` — tool invocations
- `[tool_result: ...]` — tool results (skipped)
- Raw text fragments — response text split by SSE streaming

Reassembly algorithm:
1. Tokenize each line looking for `[Thinking:`, `[Tool:`, `[tool_result:` patterns
2. Merge adjacent same-type tokens (concatenate thinking fragments, concatenate text fragments)
3. Output sections as (type, content) tuples

This is ~200 lines of Python that needs porting to Rust.
