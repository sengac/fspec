# AST Research: SessionSearch Trimming Integration (CMPCT-010)

## 1. create_handler() — Current Signature
**File:** `codelet/napi/src/session_search_handler.rs:35`
```rust
pub fn create_handler(project_path: PathBuf) -> SessionSearchHandler
```
Needs: Add `compaction_trimming: Arc<AtomicBool>` as second parameter.

## 2. handle_show() — Content Resolution Point
**File:** `codelet/napi/src/session_search_handler.rs:268`
```rust
fn handle_show(current_session_id: Uuid, show_id: Option<&str>, user_only: Option<bool>, max_turns: Option<usize>) -> SessionSearchResult
```
Content resolved at line 317: `let mut content = resolve_message_content(msg);`
Trimming insertion point: after resolve, before reassembly/truncation.

## 3. handle_search() — Content Resolution Point
**File:** `codelet/napi/src/session_search_handler.rs:120`
```rust
fn handle_search(project_path: &Path, query: &str, ...) -> SessionSearchResult
```
Content resolved at line 207: `let content = resolve_message_content(msg);`
Trimming insertion point: after resolve, before matching and preview extraction.

## 4. resolve_message_content() — Blob Resolution
**File:** `codelet/napi/src/session_search_handler.rs:411`
```rust
fn resolve_message_content(msg: &StoredMessage) -> String
```
Handles blob dereferencing. Trimming is applied AFTER this function returns.

## 5. Trimmer — API
**File:** `codelet/core/src/compaction/trimmer.rs:34`
```rust
pub struct Trimmer { tool_registry: HashMap<String, ToolUseInfo> }
```
**Method:** `codelet/core/src/compaction/trimmer.rs:51`
```rust
pub fn trim_message(&mut self, role: &str, content: &str, metadata: &HashMap<String, Value>) -> String
```
Re-exported via `codelet_core::compaction::Trimmer` (mod.rs:52).

## 6. BackgroundSession — Target Struct
**File:** `codelet/napi/src/session_manager.rs:904`
```rust
pub struct BackgroundSession { ... }
```
Existing similar fields:
- `is_interrupted: Arc<AtomicBool>` (line 933) — same pattern
- `compaction_progress: RwLock<Option<CompactionProgress>>` (line 985) — related field

## 7. StoredMessage — Metadata for Trimmer
**File:** `codelet/napi/src/persistence/types.rs:62`
```rust
pub struct StoredMessage {
    pub role: String,            // line 70
    pub content: String,         // line 72
    pub metadata: HashMap<String, serde_json::Value>,  // line 79
}
```
Fields `role`, `content`, `metadata` map directly to `Trimmer::trim_message()` parameters.

## 8. Registration Site
**File:** `codelet/napi/src/session_manager.rs:5365`
```rust
let session_search_handler = crate::session_search_handler::create_handler(
    std::path::PathBuf::from(&session.project),
);
```
Needs: Pass `session.compaction_in_progress.clone()` as second argument.

## Key Findings
- `codelet_core` is available in `codelet-napi` Cargo.toml (line 24)
- `AtomicBool` pattern already used in BackgroundSession (is_interrupted)
- `Trimmer::new()` and `Trimmer::default()` both available for construction
- handle_show processes messages in order (natural for Trimmer state)
- handle_search also processes messages in order within each session
