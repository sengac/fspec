# AST Research — Deep Search Tool Implementation Points

**Work Unit:** RLM-001  
**Date:** 2026-03-11  
**Tool:** AST code analysis  

## 1. Tool Trait Implementations (Pattern to Follow)

All tools in `codelet/tools/src/` implement `rig::tool::Tool` with a `session_id: Uuid` field:

| Tool | File | Pattern |
|------|------|---------|
| SessionSearchTool | `session_search/mod.rs:47` | `pub struct SessionSearchTool { session_id: Uuid }` |
| InjectSummaryTool | `inject_summary.rs:112` | `pub struct InjectSummaryTool { session_id: Uuid }` |
| ReadTool | `read.rs:59` | `pub struct ReadTool { session_id: Uuid }` |
| GrepTool | `grep.rs:56` | `pub struct GrepTool { session_id: Uuid }` |
| AstGrepTool | `astgrep.rs:30` | `pub struct AstGrepTool { session_id: Uuid }` |
| GlobTool | `glob.rs:29` | `pub struct GlobTool { session_id: Uuid }` |
| LsTool | `ls.rs:42` | `pub struct LsTool { session_id: Uuid }` |
| BashTool | `bash.rs` | `pub struct BashTool { session_id: Uuid }` |

**DeepSearch will follow this same struct pattern.**

## 2. Provider Agent Construction (Wiring Points)

`create_rig_agent()` exists on all 5 providers:

| Provider | File:Line | Tool Count |
|----------|-----------|-----------|
| Claude | `claude.rs:492` | 15 tools |
| OpenAI | `openai.rs:305` | 13 tools |
| Gemini | `gemini.rs:101` | 14 tools |
| Codex | `codex/mod.rs:300` | 13 tools |
| ZAI | `zai.rs:192` | 13 tools |

Claude's tool chain (line 517-531):
```
ReadTool, WriteTool, EditTool, BashTool, GrepTool, GlobTool, LsTool,
AstGrepTool, AstGrepRefactorTool, FspecTool, BridgeTool, WebSearchFacade,
ConnectMcpTool, SessionSearchTool, InjectSummaryTool
```

**DeepSearchTool will be added after InjectSummaryTool in each provider.**

## 3. SessionSearch Handler Pattern

Handler lifecycle in `session_manager.rs`:
- **Register**: line 5380-5384 — `create_handler(project_path, compaction_trimming)` + `set_session_search_handler(session.id, Some(handler))`
- **Cleanup**: line 5648 — `set_session_search_handler(session.id, None)`

Handler factory in `session_search_handler.rs`:
- `create_handler(project_path: PathBuf, compaction_trimming: Arc<AtomicBool>) -> SessionSearchHandler`
- Returns `Arc<dyn Fn(SessionSearchAction, Uuid) -> SessionSearchResult + Send + Sync>`

**For DeepSearch**: `create_handler(project_path, Arc::new(AtomicBool::new(false)))` — always false for ephemeral sub-agent.

## 4. RigAgent Non-Streaming Path

`codelet/core/src/rig_agent.rs:60`:
```rust
pub async fn prompt(&self, prompt: &str) -> Result<String> {
    self.agent.prompt(prompt).multi_turn(self.max_depth).await
}
```

Constructor: `RigAgent::new(agent, max_depth)` — DeepSearch uses `max_depth=50` (not `DEFAULT_MAX_DEPTH = usize::MAX - 1`).

## 5. LlmProvider Trait

`providers/src/lib.rs:82`:
```rust
pub trait LlmProvider: Send + Sync {
    fn model(&self) -> &str;
}
```

Public accessor `provider.client()` returns the rig client for building agents externally.

## 6. Module Exports (lib.rs)

`codelet/tools/src/lib.rs` exports all tools. DeepSearch needs:
- `pub mod deep_search;` added to module declarations
- `pub use deep_search::DeepSearchTool;` added to re-exports
