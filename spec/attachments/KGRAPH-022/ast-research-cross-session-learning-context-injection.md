# AST Research: Cross-Session Learning Context Injection Infrastructure

## Integration Points for KGRAPH-022

### 1. Learnings Graph Query Functions (existing - from KGRAPH-023)

**File**: `codelet/napi/src/graph/learnings_dispatch.rs`

```rust
pub async fn dispatch_learnings_search(db: &GraphDatabase, query: &str, category: Option<&str>, limit: Option<usize>) -> String
pub async fn dispatch_learnings_decisions(db: &GraphDatabase, domain: Option<&str>, status: Option<&str>) -> String
pub async fn dispatch_learnings_stats(db: &GraphDatabase) -> String
pub async fn dispatch_learnings_related(db: &GraphDatabase, topic: &str, min_strength: Option<f32>, limit: Option<usize>) -> String
```

### 2. Learnings Extraction Function (existing - from KGRAPH-021)

**File**: `codelet/napi/src/graph/learnings_extraction.rs`

```rust
pub fn extract_learnings_from_text(source_text: &str, llm_response: Option<&str>) -> Result<LearningsExtractionResult, String>
pub const LEARNINGS_EXTRACTION_PROMPT: &str = ...
```

### 3. Graph Registry (existing - from KGRAPH-016)

**File**: `codelet/napi/src/graph/registry.rs`

```rust
pub const LEARNINGS_GRAPH: &str = "learnings";
pub async fn get_graph(name: &str) -> Result<GraphDatabase, String>
pub fn is_graph_initialized(name: &str) -> bool
```

### 4. Graph Database Entity Loading (existing - from KGRAPH-016)

**File**: `codelet/napi/src/graph/database.rs`

```rust
pub async fn load_entities(&self, entities: &[GraphEntity]) -> Result<usize, String>
pub async fn load_jsonl(&self, jsonl: &str) -> Result<(), String>
```

### 5. Session Hooks Integration (existing - from HOOK-013)

**File**: `codelet/napi/src/session_manager.rs` (lines 4271-4294)

Session start hooks fire before the main agent loop. The outcome can inject additional context as system reminders:

```rust
// HOOK-013: Fire session_start hooks
if let Some(ref hooks) = session.lifecycle_hooks {
    let ctx = session.hook_context();
    let outcome = run_session_start(hooks, &ctx, "startup").await;
    // Inject additional context as system-reminder messages
    if !outcome.additional_context.is_empty() {
        let mut inner = session.inner.lock().await;
        let combined_context = outcome.additional_context.join("\n");
        inner.add_system_reminder(SystemReminderType::FspecWorkflow, &combined_context);
    }
}
```

### 6. DeepSearch Context Injection Pattern (existing - from KGRAPH-024)

**File**: `codelet/napi/src/deep_search_handler.rs` (lines 188-225)

Shows the existing pattern for querying learnings and injecting into system prompt:

```rust
if graph_available {
    let graph_context = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            match crate::graph::registry::get_graph(LEARNINGS_GRAPH).await {
                Ok(db) => {
                    let result = dispatch_learnings_search(&db, &query, None, Some(10)).await;
                    if result.contains("\"count\":0") { None }
                    else { Some(format!("## Relevant Learnings\n\n{}\n", result)) }
                }
                Err(_) => None,
            }
        })
    });
    if let Some(context) = graph_context { system_prompt.push_str(&context); }
}
```

### 7. Work Unit Context (existing - from TUI-059)

**File**: `codelet/napi/src/session_manager.rs` (lines 123-153)

```rust
struct WorkUnitContext { id: Option<String>, title: Option<String>, status: Option<String> }
pub fn get_work_unit_context(&self) -> Option<WorkUnitContext>
pub fn set_work_unit_context(&self, id: Option<String>, title: Option<String>, status: Option<String>)
```

### 8. System Reminder Types (existing)

**File**: `codelet/cli/src/session/system_reminders.rs`

SystemReminderType enum includes FspecWorkflow, Environment, ClaudeMd, etc.

## New Files to Create

### `codelet/napi/src/graph/learnings_context.rs`

Standalone module implementing:

1. `build_learnings_context(query: &str) -> Option<String>` — Non-blocking query that returns formatted learnings context
2. `format_learnings_for_injection(decisions: &[Value], warnings: &[Value], learnings: &[Value]) -> String` — Formats query results into a structured system-reminder-compatible format
3. Token estimation and truncation logic (cap at ~2000 tokens)

### Integration Points to Modify

1. **session_manager.rs**: After session_start hooks fire, call `build_learnings_context` with the work unit domain/epic and inject the result as a system reminder
2. **session_manager.rs**: In subordinate session creation path, inject learnings from supervisor's domain
3. **session_manager.rs**: In session_end path, trigger learnings extraction from compaction DAG
