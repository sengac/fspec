# Deep Search Tool — Architecture Research

**Date:** 2026-03-05 (verified against codebase)  
**Work Unit:** RLM-001  
**Based on:** RLM Paper (MIT CSAIL, arXiv:2512.24601v2, Jan 2026)  
**Sessions:** `7e0358a4` (design discussion), `95e3f165` (architecture deep-dive + verification)

---

## 0. How We Got Here

The original RLM-001 story (January 2026) proposed a grand integration: replace codelet's compaction strategy with a full REPL-based RLM environment (Python via PyO3, socket-based LM handler, `llm_query()` recursion). That was the right *paper analysis* but the wrong *engineering instinct*.

In the March 5 session, we walked through the actual paper results and asked progressively sharper questions:

1. "So it's like agentic search on everything?"
2. "Is it really worth the full REPL environment?"
3. "You could just create some kind of RLM search function, right?"
4. **"Why isn't this just a tool call, like Grep?"**

That last question is the breakthrough. The answer: **it IS just a tool call.** Codelet already has an agent with Read/Grep/AstGrep/Bash tools. An "RLM-style" sub-agent is just *another* agent instance — with a scoped corpus and a purpose-built system prompt — running to completion and returning its answer as a tool result. No REPL, no Python, no sockets.

The architecture deep-dive confirmed this: `RigAgent::prompt()` already handles multi-turn tool execution internally and returns a `String`. The entire DeepSearch implementation collapses to building an agent with a subset of tools and calling `prompt()`.

---

## 1. What We're Building

A **DeepSearch** tool that the parent agent calls like any other tool. Internally, it spawns an **ephemeral sub-agent** that explores a user-scoped corpus using codelet's existing tools (Read, Grep, AstGrep, Bash), then returns a text answer.

```
Parent agent (interactive session)
  │
  ├── calls Read, Write, Bash, etc. as normal
  │
  └── calls DeepSearch(query, scope) ──────────────────────┐
        │                                                    │
        │  ┌─────────────────────────────────────────────┐  │
        │  │ Ephemeral sub-agent (NOT a BackgroundSession)│  │
        │  │                                             │  │
        │  │  1. Build ProviderManager + select model     │  │
        │  │  2. Build rig Agent with scoped tools        │  │
        │  │  3. Inject RLM system prompt as preamble     │  │
        │  │  4. Call agent.prompt(query).multi_turn()    │  │
        │  │  5. Rig handles tool loop internally         │  │
        │  │  6. Return final answer string               │  │
        │  │  7. Drop everything (ephemeral)              │  │
        │  └─────────────────────────────────────────────┘  │
        │                                                    │
        ◄────────────────────────────────────────────────────┘
        Parent continues with result
```

### Key Properties
- **Ephemeral** — no persistence, no session ID, no resume, no NAPI boundary
- **Blocking** — parent's tool call blocks until sub-agent finishes
- **Scoped** — caller specifies exactly what's in scope (paths, globs, conversation histories)
- **Stripped tools** — Read, Grep, AstGrep, Bash only (no Write, Bridge, Fspec, WebSearch, MCP)
- **Cheaper model** — can use sonnet/haiku for sub-agent even if parent is opus
- **Pure Rust** — called directly from within the tool execution context
- **~100-150 lines** — not 1610 like the main stream loop

---

## 2. Why NOT BackgroundSession

`BackgroundSession` (in `codelet/napi/src/session_manager.rs`) is the **wrong abstraction**. It carries:

| Feature | Needed for DeepSearch? |
|---------|----------------------|
| `output_buffer` + `watcher_broadcast` for UI streaming | ❌ No UI |
| Persistence layer (`persist_user_message`, etc.) | ❌ Ephemeral |
| Pause/resume handlers (user confirmation dialogs) | ❌ Would deadlock |
| Attach/detach lifecycle (tmux-like switching) | ❌ Not interactive |
| Anchor points, compaction progress, debug metadata | ❌ Disposable |
| Token tracking for context fill indicator | ❌ No display |
| `MAX_SESSIONS = 10` global limit | ❌ Would conflict with parent |
| NAPI boundary (Node.js ThreadsafeFunction callbacks) | ❌ Pure Rust |

---

## 3. The Critical Discovery: `RigAgent::prompt()`

While researching the streaming agent loop (`run_agent_stream_internal` in `codelet/cli/src/interactive/stream_loop.rs`, 1610 lines), we found that `RigAgent` already has a **non-streaming `prompt()` method** at `codelet/core/src/rig_agent.rs:60`:

```rust
// codelet/core/src/rig_agent.rs:60-76
pub async fn prompt(&self, prompt: &str) -> Result<String> {
    debug!(prompt = %prompt, "Starting agent execution");

    let response = self
        .agent
        .prompt(prompt)
        .multi_turn(self.max_depth)
        .await
        .map_err(|e| anyhow!("Prompt failed: {e}"))?;

    debug!(
        response_length = response.len(),
        "Agent execution completed"
    );

    Ok(response)
}
```

This does **everything**:
- Multi-turn tool calling (Read, Grep, etc.)
- Automatic depth control via rig's `max_depth`
- Returns the final text response after all tool calls complete

**Note:** `DEFAULT_MAX_DEPTH` is `usize::MAX - 1` (effectively unlimited). For DeepSearch, we should set a sensible limit (e.g., 50) to prevent runaway sub-agents.

### How rig's multi-turn loop works internally

Under the hood, rig's `PromptRequest` (at `codelet/patches/rig-core/src/agent/prompt_request/mod.rs:242`) implements `IntoFuture` with a loop that:

1. Sends prompt to model (`agent.completion()` at line 392)
2. Extracts tool calls from response (partition into `tool_calls` and `texts` at line 412)
3. Executes tools via `stream::iter(tool_calls).map(...).buffer_unordered(self.concurrency)` (lines 450-547)
   - **Default concurrency is 1** (sequential execution, set at line 80)
   - Can be overridden with `.with_tool_concurrency(n)` (line 123)
4. Appends tool results to chat history as `Message::User` (line 559)
5. Loops until no more tool calls OR `current_max_depth > self.max_depth + 1` (line 339)
6. Returns final text as `PromptResponse { output, total_usage }` (line 444)

The loop also supports `PromptHook` callbacks (`on_completion_call`, `on_completion_response`, `on_tool_call`, `on_tool_result`) and a `CancelSignal` mechanism — none of which DeepSearch needs in v1.

**This eliminates the need for any custom stream loop, `StreamOutput` implementation, or manual `MultiTurnStreamItem` handling.**

### What the main stream loop adds (and we DON'T need):

| Feature | In `run_agent_stream_internal` | Needed? |
|---------|-------------------------------|---------|
| Compaction logic (pre-prompt check, CompactionHook) | ✅ | ❌ Ephemeral, won't fill context |
| Token tracking / context fill display | ✅ | ❌ No UI |
| Interrupt handling (is_interrupted AtomicBool) | ✅ | ❌ Non-interactive |
| Debug capture events | ✅ | ❌ Disposable |
| Gemini continuation workarounds | ✅ | ❌ Not relevant |
| Tool progress streaming | ✅ | ❌ No display |
| Retry-after-compaction | ✅ | ❌ Ephemeral |

---

## 4. How Agents Are Currently Constructed

### Provider-specific `create_rig_agent()`

Each provider has its own `create_rig_agent()` method that wires all tools. The signature is identical across all 5 providers:

```rust
pub fn create_rig_agent(
    &self,
    session_id: uuid::Uuid,
    preamble: Option<&str>,
    thinking_config: Option<serde_json::Value>,
) -> rig::agent::Agent<ProviderCompletionModel>
```

**Verified locations and tool counts:**
- `codelet/providers/src/claude.rs:492` — 13 tools, returns `Agent<ClaudeCompletionModel>`
- `codelet/providers/src/openai.rs:305` — 13 tools, returns `Agent<openai::completion::CompletionModel>`
- `codelet/providers/src/gemini.rs:101` — 14 tools (extra web_fetch), returns `Agent<gemini::completion::CompletionModel>`
- `codelet/providers/src/codex/mod.rs:300` — 13 tools, returns `Agent<CodexResponsesModel>`
- `codelet/providers/src/zai.rs:192` — 13 tools, returns `Agent<openai::completion::CompletionModel>`

**Field visibility (CRITICAL for DeepSearch):**
- `rig_client` — **PRIVATE** field on all providers (e.g., `ClaudeProvider` line 166)
- `model_name` — **PRIVATE** field on all providers (e.g., `ClaudeProvider` line 169)
- `provider.client()` — **PUBLIC** accessor, returns `&ClaudeClient` (line 414)
- `provider.model()` — **PUBLIC** via `LlmProvider` trait (line 658), returns `&str`

Inside `create_rig_agent()`, the method accesses its own private fields:
```rust
// This works because it's &self inside the impl block:
let mut agent_builder = self
    .rig_client                              // private field, OK from &self
    .agent(&self.model_name)                 // private field, OK from &self
    .max_tokens(MAX_OUTPUT_TOKENS as u64)
    .tool(ReadTool::new(session_id))
    .tool(WriteTool::new(session_id))
    // ... 11 more tools ...
    .tool(ConnectMcpTool::new(session_id));
```

**For DeepSearch, external code must use public accessors:**
```rust
// Correct approach from outside the provider impl:
use rig::client::CompletionClient;  // trait at patches/rig-core/src/client/completion.rs:19
let agent_builder = provider.client()   // public accessor → &ClaudeClient
    .agent(provider.model())             // CompletionClient::agent() + LlmProvider::model()
```

The `CompletionClient` trait (at `codelet/patches/rig-core/src/client/completion.rs:19`) provides the `.agent(model)` method that returns an `AgentBuilder`.

### The CLI agent runner (`codelet/cli/src/interactive/agent_runner.rs`)

Shows the pattern: get provider → `create_rig_agent` → wrap in `RigAgent` → run stream loop.

```rust
// codelet/cli/src/interactive/agent_runner.rs (63 lines total)
macro_rules! run_with_provider {
    ($get_provider:ident, $preamble:expr) => {{
        let provider = manager.$get_provider()?;
        let rig_agent = provider.create_rig_agent(session_id, $preamble, None);
        let agent = RigAgent::with_default_depth(rig_agent);
        run_agent_stream_with_interruption(agent, prompt, session, ...).await
    }};
}

match provider_name.as_str() {
    "claude" => run_with_provider!(get_claude, None),
    "openai" => run_with_provider!(get_openai, None),
    "codex"  => run_with_provider!(get_codex, None),
    "gemini" => run_with_provider!(get_gemini, None),
    _ => Err(anyhow::anyhow!("Unknown provider")),
}
```

**Note:** ZAI is NOT in the CLI agent_runner — it's only handled in the NAPI `session_manager.rs` (lines 5539, 5655).

For DeepSearch, the pattern is simpler — non-streaming, no session:

```rust
let provider = manager.get_claude()?;
// Option A: reuse create_rig_agent() but it wires ALL 13 tools
let rig_agent = provider.create_rig_agent(session_id, Some(&system_prompt), None);

// Option B: build a custom agent with only search tools (preferred)
let rig_agent = build_search_agent(provider, session_id, &system_prompt);

let agent = RigAgent::new(rig_agent, 50);  // 50 depth, not usize::MAX
let result = agent.prompt(query).await?;    // Non-streaming, blocking
```

---

## 5. What Gets Reused vs What's Custom

### ✅ REUSE directly

| Primitive | Location | How |
|-----------|----------|-----|
| `ProviderManager` | `codelet/providers/src/manager.rs:182` | `ProviderManager::with_model_support().await?` + `select_model()` |
| Credential resolution | `codelet/providers/src/credentials.rs` | Env vars already set by parent session |
| `RigAgent::prompt()` | `codelet/core/src/rig_agent.rs:60` | Non-streaming multi-turn execution |
| `ReadTool` | `codelet/tools/src/read.rs` | `ReadTool::new(session_id)` |
| `GrepTool` | `codelet/tools/src/grep.rs` | `GrepTool::new(session_id)` |
| `AstGrepTool` | `codelet/tools/src/astgrep.rs` | `AstGrepTool::new(session_id)` |
| `BashTool` | `codelet/tools/src/bash.rs` | `BashTool::new(session_id)` |
| `GlobTool` | `codelet/tools/src/glob.rs` | `GlobTool::new(session_id)` |
| `LsTool` | `codelet/tools/src/ls.rs` | `LsTool::new(session_id)` |
| `CompletionClient` trait | `patches/rig-core/src/client/completion.rs:19` | `.agent(model)` returns `AgentBuilder` |

### 🔧 CUSTOM (new code)

| Component | Why |
|-----------|-----|
| `DeepSearchTool` struct | Implements `rig::tool::Tool` — the tool the parent agent calls |
| `build_search_agent()` | Builds a rig agent with only the search subset of tools + RLM preamble |
| Scope metadata generator | Converts `SearchScope` to system prompt description |
| System prompt template | RLM-adapted prompt describing scope and search strategy |

### ❌ NOT NEEDED

| Component | Why Not |
|-----------|---------|
| Custom stream loop | `prompt()` handles everything |
| `StreamOutput` implementation | No streaming, no UI |
| `Session` struct | No conversation persistence needed |
| CompactionHook | Ephemeral, won't fill context |
| Token tracking | No display, no cost budgeting in v1 |
| `BackgroundSession` | Wrong abstraction entirely |

---

## 6. Corpus Scoping Design

The caller specifies what's in scope. This is a **first-class design requirement** — the sub-agent must not wander outside its declared corpus.

### Scope Types

```rust
pub enum SearchScope {
    /// Specific file paths
    Files(Vec<PathBuf>),
    
    /// Glob patterns relative to project root
    Globs(Vec<String>),
    
    /// Directory tree (recursive)
    Directory(PathBuf),
    
    /// Multiple scopes combined
    Combined(Vec<SearchScope>),
    
    /// Raw text blob (for arbitrary content like conversation history)
    RawText(String),
}
```

### How Scope Works

The scope is **NOT loaded into the sub-agent's context window** — that's the whole point of RLM. Instead:

1. **System prompt describes the scope as metadata:**
   ```
   Your search scope contains:
   - 47 files matching "src/**/*.rs" (total ~2.3M chars)
   - 12 files matching "spec/features/*.feature" (total ~180K chars)
   
   Files are accessible via Read, Grep, and AstGrep tools.
   Do NOT try to read all files — explore strategically.
   ```

2. **Tools are scoped** — the sub-agent's Read/Grep tools could be restricted to the declared paths (v2 — for v1, the system prompt alone is sufficient since the sub-agent has no reason to look elsewhere)

3. **For conversation history scope**, the history is serialized to a temp file that the sub-agent can Read/Grep through

---

## 7. System Prompt Design

Adapted from RLM paper Appendix C, but using codelet's tool-based approach instead of a Python REPL:

```
You are a research assistant tasked with answering a query by exploring a scoped
corpus of files. You have access to tools for reading files, searching with regex,
and structural code search.

YOUR SEARCH SCOPE:
{scope_description}

AVAILABLE TOOLS:
- Read: Read file contents (use offset/limit for large files)
- Grep: Search file contents by regex pattern
- AstGrep: AST-based structural code search (for code files)
- Glob: Find files matching patterns
- Ls: List directory contents
- Bash: Execute shell commands for data processing

STRATEGY:
1. Start by understanding the scope — use Grep to find relevant files
2. Read targeted sections, not entire files
3. For code: use AstGrep to find structural patterns (functions, types, etc.)
4. Build up your answer incrementally
5. When you have enough information, provide your final answer

IMPORTANT:
- Do NOT try to read all files at once — explore strategically
- Use Grep/AstGrep to narrow down before reading
- Your answer should directly address the original query
- If the answer is not in scope, say so explicitly

QUERY: {query}
```

---

## 8. Tool Parameter Design

```rust
/// What the parent LLM calls
pub struct DeepSearchParams {
    /// The question to answer
    query: String,
    
    /// Paths or glob patterns defining what to search
    /// Examples: ["src/"], ["**/*.rs"], ["/path/to/specific/file.txt"]
    scope: Vec<String>,
    
    /// Optional: model to use for the sub-agent
    /// Defaults to a cheaper model (e.g., "anthropic/claude-sonnet-4")
    model: Option<String>,
    
    /// Optional: maximum tool call depth before giving up
    /// Defaults to 50
    max_depth: Option<usize>,
}
```

---

## 9. Concrete Implementation Sketch

```rust
// codelet/tools/src/deep_search.rs

use codelet_core::RigAgent;
use codelet_providers::ProviderManager;
use rig::tool::Tool;

pub struct DeepSearchTool {
    // Could hold default model preference, etc.
}

impl Tool for DeepSearchTool {
    const NAME: &'static str = "DeepSearch";
    type Error = /* ... */;
    type Args = DeepSearchParams;
    type Output = String;
    // definition() — JSON schema for the LLM
    // call() → calls deep_search()
}

/// Core function — provider-agnostic
async fn deep_search(
    query: &str,
    scope: &[String],
    model: Option<&str>,
    max_depth: usize,
) -> Result<String> {
    // 1. Create provider (credentials already in env from parent)
    let mut mgr = ProviderManager::with_model_support().await?;
    let model_str = model.unwrap_or("anthropic/claude-sonnet-4");
    mgr.select_model(model_str)?;
    
    // 2. Generate scope description for system prompt
    let scope_desc = generate_scope_description(&scope)?;
    let system_prompt = format!(RLM_SYSTEM_PROMPT_TEMPLATE, 
        scope_description = scope_desc,
        query = query);
    
    // 3. Build agent with search-only tools
    let session_id = uuid::Uuid::new_v4();  // Ephemeral, no worktree
    let rig_agent = build_search_agent(&mgr, session_id, &system_prompt)?;
    let agent = RigAgent::new(rig_agent, max_depth);
    
    // 4. Run to completion (blocking, non-streaming)
    let result = agent.prompt(query).await?;
    
    Ok(result)
}

/// Build a rig agent with only read-only search tools
fn build_search_agent(
    mgr: &ProviderManager,
    session_id: uuid::Uuid,
    preamble: &str,
) -> Result<rig::agent::Agent</* provider model type */>> {
    // VERIFIED: Must use PUBLIC accessors, not private fields
    use rig::client::CompletionClient;  // at patches/rig-core/src/client/completion.rs:19
    
    let provider = mgr.get_claude()?;
    let agent = provider.client()         // PUBLIC: &ClaudeClient (claude.rs:414)
        .agent(provider.model())          // PUBLIC: CompletionClient::agent() + LlmProvider::model()
        .max_tokens(8192_u64)             // MAX_OUTPUT_TOKENS from claude.rs:46
        .tool(ReadTool::new(session_id))
        .tool(GrepTool::new(session_id))
        .tool(AstGrepTool::new(session_id))
        .tool(GlobTool::new(session_id))
        .tool(LsTool::new(session_id))
        .tool(BashTool::new(session_id))
        .preamble(preamble)
        .build();
    
    Ok(agent)
}
```

### Provider-Agnostic Challenge

The return type of `.agent().build()` is `Agent<M>` where `M` is the provider's `CompletionModel` type:
- Claude: `Agent<anthropic::completion::CompletionModel<RefreshingClaudeClient>>`
- OpenAI: `Agent<openai::completion::CompletionModel>`
- Gemini: `Agent<gemini::completion::CompletionModel>`
- Codex: `Agent<CodexResponsesModel>`
- ZAI: `Agent<openai::completion::CompletionModel>` (shares OpenAI's type)

Options:
1. **v1: Claude-only** — just hardcode to Claude provider, add others later
2. **v2: Macro approach** — like `run_with_provider!` in `agent_runner.rs` (line 33)
3. **v3: Add `create_search_agent()` to each provider**

---

## 10. The `llm_query()` Sub-Call Question

The paper's full RLM includes `llm_query()` — letting the sub-agent spawn **another** LLM call from within code execution. This is the "symbolic recursion" that makes RLM truly powerful.

### For v1: Skip it

The paper's "RLM no sub-calls" ablation (Table 1) already beats all baselines:
- GPT-5: 58% CodeQA, 88% BrowseComp+ (vs 62% / 91% with sub-calls)
- Qwen3-Coder: **66%** CodeQA (actually BETTER without sub-calls), 46% BrowseComp+

Our sub-agent already IS an LLM with tool access — it can reason about what it reads. The "no sub-calls" version maps exactly to "agent with tools but no recursive spawning."

### For v2: Add as a tool

```rust
pub struct LlmQueryTool { model: String }
// Takes (query: string, context: string) -> string
// Spawns yet another ephemeral LLM call (no tools, just text in/out)
```

This gives full RLM with `max_depth=2` (parent → DeepSearch → llm_query leaf calls).

---

## 11. Open Questions

1. **How does the tool get credentials?** 
   - Env vars are already set by the parent's credential resolution step
   - `ProviderManager::with_model_support()` reads from env
   - **Answer: Just create a new ProviderManager — credentials are in env**

2. **Should we scope the Bash tool?**
   - Full Bash is powerful but risky in a sub-agent
   - For v1: include it with same restrictions as parent (blocklist applies)
   - For v2: consider read-only Bash wrapper

3. **Cost controls?**
   - `max_depth` parameter (hard limit on tool call rounds)
   - v1: that's sufficient
   - v2: add token budget tracking, wall-clock timeout

4. **Where does the code live?**
   - `codelet/tools/src/deep_search.rs` — the DeepSearchTool struct + Tool impl
   - Agent construction lives there too (it's tool-adjacent code)
   - If scope logic grows complex, promote to `codelet/tools/src/deep_search/`

5. **Provider-specific model types?**
   - Need to handle `Agent<ClaudeCompletionModel>` vs `Agent<OpenAICompletionModel>` etc.
   - v1: start Claude-only, same pattern as `agent_runner.rs` macro
   - v2: add `create_search_agent()` to each provider

6. **Wire into parent agent how?**
   - Add `DeepSearchTool` to each provider's `create_rig_agent()` method
   - It becomes tool #14 alongside the existing 13 (Claude) / 14 (Gemini)

7. **Session ID for sub-agent tools?**
   - Tools require `session_id: Uuid` for worktree isolation
   - Sub-agent generates a fresh `Uuid::new_v4()` — ephemeral, no worktree
   - Read/Grep will operate on the real filesystem, not a git worktree
   - That's correct: DeepSearch is read-only, no worktree needed

---

## 12. Implementation Plan

### Phase 1: Minimal viable tool (~2-3 hours, 8 story points)
- [ ] `DeepSearchTool` struct implementing `rig::tool::Tool` (trait at `patches/rig-core/src/tool/mod.rs:106`)
- [ ] `DeepSearchParams` (query, scope, model, max_depth)
- [ ] `deep_search()` core function using `RigAgent::prompt()`
- [ ] `build_search_agent()` — Claude-only, 6 search tools, using `provider.client()` + `provider.model()`
- [ ] `generate_scope_description()` — file count + size metadata
- [ ] System prompt template adapted from RLM Appendix C
- [ ] Wire into Claude's `create_rig_agent()` as tool #14
- [ ] Hardcoded to same model as parent (no separate model selection yet)

### Phase 2: Multi-provider + scope controls (~2-3 hours)
- [ ] Add `create_search_agent()` to OpenAI, Gemini, Codex, ZAI providers
- [ ] `SearchScope` enum (Files, Globs, Directory, Combined, RawText)
- [ ] Glob expansion for scope resolution
- [ ] Optional: scoped tool wrappers (restrict paths)

### Phase 3: Model selection + cost controls (~1-2 hours)
- [ ] Optional `model` parameter for cheaper sub-agent model
- [ ] `max_depth` enforcement (already in rig, just expose)
- [ ] Timeout support (tokio::time::timeout wrapper)
- [ ] Token usage reporting in tool result

### Phase 4: Conversation history scope (future)
- [ ] Session message serializer → temp file
- [ ] Cross-session scope (read other sessions' histories)
- [ ] Configurable inclusion (tool results, thinking, etc.)

### Phase 5: `llm_query()` recursion (future)
- [ ] `LlmQueryTool` for leaf-level text-in/text-out LLM calls
- [ ] Wire into sub-agent's tool set
- [ ] Recursion depth limit

---

## 13. Verification Log

All claims in this document were verified against the codebase on 2026-03-05.

| Claim | Verified At | Status |
|-------|-------------|--------|
| `RigAgent::prompt()` exists at line 60 | `codelet/core/src/rig_agent.rs:60` | ✅ |
| `DEFAULT_MAX_DEPTH = usize::MAX - 1` | `codelet/core/src/rig_agent.rs:16` | ✅ |
| `Tool` trait: `NAME`, `Args`, `Output`, `call()` | `patches/rig-core/src/tool/mod.rs:106` | ✅ |
| All tools use `new(session_id: Uuid)` | Grep across all tool files | ✅ |
| `rig_client` is PRIVATE on ClaudeProvider | `claude.rs:166` | ✅ |
| `model_name` is PRIVATE on ClaudeProvider | `claude.rs:169` | ✅ |
| `provider.client()` is PUBLIC | `claude.rs:414` | ✅ |
| `LlmProvider::model()` returns `&str` | `providers/src/lib.rs:87` | ✅ |
| `CompletionClient::agent()` returns `AgentBuilder` | `patches/.../client/completion.rs:54` | ✅ |
| `create_rig_agent()` locations | claude:492, openai:305, gemini:101, codex:300, zai:192 | ✅ |
| Claude wires 13 tools, Gemini 14 | `grep '\.tool(' <file>` | ✅ |
| Stream loop is 1610 lines | `wc -l stream_loop.rs` | ✅ |
| Multi-turn uses `buffer_unordered(concurrency)` | `prompt_request/mod.rs:546` | ✅ |
| Default tool concurrency is 1 | `prompt_request/mod.rs:80` | ✅ |
| CLI agent_runner: claude/openai/codex/gemini only | `agent_runner.rs:56-61` | ✅ |
| ZAI only in NAPI session_manager | `session_manager.rs:5539` | ✅ |
| `ProviderManager::with_model_support()` | `manager.rs:182` | ✅ |
| `MAX_OUTPUT_TOKENS = 8192` for Claude | `claude.rs:46` | ✅ |

---

## References

- RLM Paper: https://arxiv.org/abs/2512.24601
- RLM Source Code: https://github.com/alexzhang13/rlm
- RLM Blog Post: https://alexzhang13.github.io/blog/2025/rlm/
- Key codelet files:
  - `codelet/core/src/rig_agent.rs` — RigAgent with `prompt()` non-streaming path
  - `codelet/providers/src/claude.rs:492` — `create_rig_agent()` tool wiring pattern
  - `codelet/providers/src/lib.rs:82` — `LlmProvider` trait with `model()` accessor
  - `codelet/cli/src/interactive/agent_runner.rs` — CLI agent construction pattern
  - `codelet/cli/src/interactive/stream_loop.rs` — The 1610-line loop we DON'T need
  - `codelet/providers/src/manager.rs:182` — `ProviderManager::with_model_support()`
  - `codelet/tools/src/lib.rs` — All tool exports
  - `codelet/patches/rig-core/src/agent/prompt_request/mod.rs` — Rig's multi-turn internals
  - `codelet/patches/rig-core/src/tool/mod.rs:106` — `Tool` trait definition
  - `codelet/patches/rig-core/src/client/completion.rs:19` — `CompletionClient` trait
