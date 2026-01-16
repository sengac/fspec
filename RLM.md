# RLM Integration Analysis for codelet/fspec

## Executive Summary

This document analyzes **Recursive Language Models (RLM)** from MIT CSAIL and explores how this paradigm could be integrated into codelet/fspec to dramatically enhance long-context handling and complex reasoning capabilities.

**Key Finding:** RLM offers a fundamentally different approach to context management that could replace or significantly augment codelet's current compaction strategy, enabling near-infinite context handling at comparable or lower cost.

---

## Part 1: Understanding RLM

### 1.1 Core Insight

The central innovation of RLM is simple but powerful:

> "Long prompts should not be fed into the neural network directly but should instead be treated as **part of the environment** that the LLM can **symbolically interact with**."

Instead of stuffing everything into the context window, RLM:
1. Loads the input prompt as a **variable** in a REPL environment
2. Lets the LLM write **code** to examine, filter, and decompose the context
3. Enables **recursive sub-LM calls** over programmatic snippets
4. Returns a final answer when ready

### 1.2 Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        RLM Interface                         │
│                   (same as LLM: prompt → response)           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                     Root LLM (GPT-5/Claude)                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ System Prompt: You have access to a REPL environment │    │
│  │ with `context` variable and `llm_query()` function   │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    REPL Environment (Python)                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ context = <loaded prompt - can be 10M+ chars>       │    │
│  │ llm_query(prompt) → sub-LM call                     │    │
│  │ llm_query_batched(prompts) → concurrent sub-calls   │    │
│  │ print() → observe truncated output                  │    │
│  │ FINAL(answer) / FINAL_VAR(var_name) → return        │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    LM Handler (Socket Server)                │
│  Routes sub-LM calls to appropriate model backend           │
│  Tracks token usage across all calls                        │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 Key Components (from source code analysis)

#### RLM Core (`rlm/core/rlm.py`)
- **`completion(prompt)`**: Main entry point - replaces standard LLM call
- **`max_depth`**: Controls recursion depth (default: 1)
- **`max_iterations`**: Limits REPL interaction rounds (default: 30)
- Spawns fresh environment per completion call (or reuses with `persistent=True`)

#### REPL Environment (`rlm/environments/local_repl.py`)
- **`context` variable**: The prompt loaded as a string/dict/list
- **`llm_query(prompt)`**: Make sub-LM call, returns response string
- **`llm_query_batched(prompts)`**: Concurrent sub-calls for efficiency
- **`FINAL_VAR(var_name)`**: Return a variable as final answer
- Safe builtins (blocks `eval`, `exec`, `input`)
- Temp directory for file operations

#### LM Handler (`rlm/core/lm_handler.py`)
- Multi-threaded TCP socket server
- Routes requests based on depth (root LM vs sub-LM)
- Supports different models for different depths (e.g., GPT-5 for root, GPT-5-mini for sub-calls)
- Tracks token usage across all calls

### 1.4 Performance Results (from paper)

| Task | GPT-5 Base | RLM (GPT-5) | Improvement |
|------|-----------|-------------|-------------|
| S-NIAH (131K tokens) | 92% | 96% | +4% |
| S-NIAH (262K tokens) | 74% | 96% | +22% |
| BrowseComp+ (1K docs, 6-11M tokens) | 0%* | 91% | N/A (beyond context) |
| OOLONG (131K tokens) | 44% | 56% | +28% |
| OOLONG-Pairs (32K tokens) | 0.04% | 58% | +57.96% |
| CodeQA (23K-4.2M tokens) | 24%* | 62% | +38% |

*Base model failed due to context limits

**Cost Analysis:** RLM median cost is comparable to (or cheaper than) base model calls, with high variance for complex tasks.

---

## Part 2: Current codelet/fspec Architecture

### 2.1 Context Management Today

codelet currently uses **anchor-based context compaction**:

```rust
// codelet/core/src/compaction/compactor.rs
pub struct ContextCompactor {
    confidence_threshold: f64,      // Anchor detection threshold
    min_compression_ratio: f64,     // Target compression
    strategy: CompactionStrategy,   // AnchorBased, SimpleTruncate, None
}
```

**How it works:**
1. Detect "anchor points" in conversation (file writes, user checkpoints)
2. Summarize older turns, preserve recent turns from anchor
3. Generate template-based summary (NO LLM call for summary)
4. Reconstruct message history

**Limitations:**
- Still bounded by model context window after compaction
- Lossy - information is permanently lost in summarization
- Cannot handle truly massive inputs (codebases, document corpuses)
- Summary quality degrades with conversation complexity

### 2.2 Tool Execution Framework

```rust
// Tools are implemented via Rig's Tool trait
pub struct ReadTool;
pub struct WriteTool;
pub struct EditTool;
pub struct BashTool;
pub struct GrepTool;
pub struct AstGrepTool;
// etc.
```

Tools execute atomically and return results to the LLM. There's no concept of "the LLM programming its own context exploration."

### 2.3 Multi-Provider Architecture

```rust
// codelet/providers/src/manager.rs
pub struct ProviderManager {
    claude: Option<ClaudeProvider>,
    openai: Option<OpenAIProvider>,
    gemini: Option<GeminiProvider>,
    codex: Option<CodexProvider>,
}
```

This is a strength - RLM is model-agnostic and would work with any provider.

---

## Part 3: Integration Strategy

### 3.1 Integration Options

#### Option A: RLM as a New Tool ("Recursive Analysis")
**Effort:** Low  
**Impact:** Medium

Add RLM as a tool that the agent can invoke for complex analysis tasks:

```typescript
interface RecursiveAnalysisTool {
  name: "RecursiveAnalysis";
  description: "Analyze large amounts of data using recursive decomposition";
  parameters: {
    context: string | string[];  // The data to analyze
    query: string;               // What to analyze/find
  };
}
```

**Pros:**
- Minimal changes to existing architecture
- Agent chooses when to use RLM
- Can be added incrementally

**Cons:**
- Doesn't solve the fundamental context limit problem
- Two-level decision making (agent decides to use RLM, RLM decides how to analyze)

#### Option B: RLM as Alternative Context Strategy
**Effort:** Medium  
**Impact:** High

Replace/augment the compaction system with RLM's environment-based approach:

```rust
pub enum ContextStrategy {
    /// Current anchor-based compaction
    Compaction(CompactionConfig),
    /// RLM environment-based (context as variable)
    Recursive(RLMConfig),
    /// Hybrid: use compaction normally, switch to RLM for large contexts
    Hybrid { compaction_threshold: usize, rlm_config: RLMConfig },
}
```

**Pros:**
- Solves context limits fundamentally
- Preserves information (no lossy summarization)
- Better for code analysis tasks

**Cons:**
- More complex implementation
- Requires REPL environment (Python or Rust-based)
- Higher latency for simple tasks

#### Option C: Full RLM Mode (Recommended)
**Effort:** High  
**Impact:** Very High

Implement RLM as a first-class operating mode alongside the current agent loop:

```rust
pub enum AgentMode {
    /// Standard multi-turn agent with compaction
    Standard,
    /// RLM mode: context as environment variable
    Recursive {
        max_depth: usize,
        max_iterations: usize,
        environment: REPLEnvironment,
    },
}
```

When enabled, the entire conversation paradigm shifts:
- User prompt becomes `context` variable
- Agent writes code to explore context
- Sub-agent calls handle complex subtasks
- No compaction needed (context never enters LLM directly)

### 3.2 Recommended Implementation Path

#### Phase 1: Rust REPL Environment (2-3 weeks)
Create a sandboxed code execution environment in Rust:

```rust
// codelet/core/src/rlm/environment.rs
pub struct RustREPL {
    context: ContextPayload,
    locals: HashMap<String, Value>,
    llm_handler: LMHandlerClient,
}

impl RustREPL {
    pub fn execute_code(&mut self, code: &str) -> REPLResult;
    pub fn llm_query(&self, prompt: &str) -> String;
    pub fn llm_query_batched(&self, prompts: &[String]) -> Vec<String>;
}
```

Options for execution:
1. **Rhai scripting**: Embedded Rust scripting language
2. **Python via PyO3**: Call Python REPL from Rust
3. **WASM sandbox**: Compile Python/JS to WASM for isolation

**Recommendation:** Start with Python via PyO3 (matches RLM exactly), migrate to Rhai for performance later.

#### Phase 2: LM Handler Service (1 week)
Port the socket-based LM handler pattern:

```rust
// codelet/core/src/rlm/handler.rs
pub struct LMHandler {
    root_client: Box<dyn LLMProvider>,
    sub_client: Option<Box<dyn LLMProvider>>,
    usage_tracker: UsageTracker,
}

impl LMHandler {
    pub async fn completion(&self, prompt: &str, depth: usize) -> Result<String>;
    pub async fn completion_batched(&self, prompts: &[String], depth: usize) -> Result<Vec<String>>;
}
```

#### Phase 3: RLM Orchestrator (1-2 weeks)
Main completion loop matching RLM behavior:

```rust
// codelet/core/src/rlm/orchestrator.rs
pub struct RLMOrchestrator {
    config: RLMConfig,
    handler: LMHandler,
}

impl RLMOrchestrator {
    pub async fn completion(&self, prompt: ContextPayload) -> RLMResult {
        let env = RustREPL::new(prompt, self.handler.clone());
        let mut history = self.setup_system_prompt(&env);
        
        for iteration in 0..self.config.max_iterations {
            let response = self.handler.completion(&history, 0).await?;
            
            // Extract and execute code blocks
            for code_block in extract_code_blocks(&response) {
                let result = env.execute_code(&code_block);
                history.push(format_result(&result));
            }
            
            // Check for final answer
            if let Some(answer) = extract_final_answer(&response, &env) {
                return Ok(RLMResult { answer, usage: self.handler.usage() });
            }
        }
        
        // Fallback: ask for final answer
        self.default_answer(&history).await
    }
}
```

#### Phase 4: Integration with Existing Tools (1 week)
Make existing tools available within the REPL environment:

```rust
impl RustREPL {
    fn setup_tools(&mut self) {
        // Expose codelet tools as functions
        self.register_function("read_file", |path| ReadTool::execute(path));
        self.register_function("grep", |pattern, path| GrepTool::execute(pattern, path));
        self.register_function("ast_grep", |pattern, lang| AstGrepTool::execute(pattern, lang));
        // etc.
    }
}
```

This allows RLM to leverage codelet's existing tools while maintaining the recursive context management benefits.

#### Phase 5: NAPI Bindings & TUI Integration (1 week)
Expose RLM mode to the TypeScript TUI:

```typescript
// codelet/napi/src/rlm.rs → index.d.ts
export interface RLMConfig {
  maxDepth: number;
  maxIterations: number;
  subModel?: string;  // Optional different model for sub-calls
}

export class CodeletSession {
  // Existing methods...
  
  /** Enable RLM mode for next prompt */
  setRLMMode(config: RLMConfig): void;
  
  /** Check if response is from RLM (has REPL iterations) */
  isRLMResponse(): boolean;
}
```

### 3.3 System Prompt Adaptation

The RLM system prompt needs adaptation for codelet's use case:

```rust
const CODELET_RLM_SYSTEM_PROMPT: &str = r#"
You are a coding assistant with access to a REPL environment. Your task context 
is loaded as the `context` variable - this could be a codebase, documentation, 
or conversation history too large to fit in your context window.

Available functions:
- `context` - The loaded context (string, list, or dict)
- `llm_query(prompt)` - Query a sub-LLM for analysis
- `llm_query_batched(prompts)` - Concurrent sub-LLM queries
- `read_file(path)` - Read a file from disk
- `grep(pattern, path)` - Search file contents
- `ast_grep(pattern, language)` - AST-based code search
- `print()` - Output for observation (truncated)

Strategy for code analysis:
1. First, explore the context structure (print first/last lines, count lines)
2. Use grep/ast_grep to find relevant code patterns
3. Use llm_query to analyze specific code sections
4. Build up your answer incrementally using variables
5. Return FINAL(answer) or FINAL_VAR(variable_name) when done

Remember: sub-LLMs can handle ~500K characters each, so batch intelligently.
"#;
```

---

## Part 4: What Gets Replaced vs Enhanced

### 4.1 Components to Replace

| Component | Current | RLM Replacement | Reason |
|-----------|---------|-----------------|--------|
| **Context Compaction** | Anchor-based summarization | Environment-based context | No information loss, handles arbitrary size |
| **Token Tracking** | Per-message estimation | Per-call tracking across depths | More accurate cost accounting |
| **Large Context Handling** | Truncation/summarization | Symbolic access via REPL | Fundamental improvement |

### 4.2 Components to Enhance

| Component | Current | Enhancement | Benefit |
|-----------|---------|-------------|---------|
| **Tool Execution** | Direct tool calls | Tools available in REPL | LLM can combine tools programmatically |
| **Multi-Turn Conversations** | Sequential messages | Persistent REPL state | Variables persist across turns |
| **Provider Selection** | Static per-session | Dynamic per-depth | Cost optimization (cheaper model for sub-calls) |

### 4.3 Components to Keep

| Component | Reason |
|-----------|--------|
| **Rig.rs Integration** | Proven, well-tested LLM abstraction |
| **NAPI Bindings** | TUI integration already works |
| **Provider Manager** | Multi-provider support is valuable |
| **Session Persistence** | Conversation history still needed |

---

## Part 5: Risks and Mitigations

### 5.1 Latency Concerns

**Risk:** RLM adds latency due to multiple iterations and sub-calls.

**Mitigations:**
1. Use cheaper/faster model for sub-calls (e.g., claude-3-haiku for subs)
2. Implement batched queries for parallelism
3. Add "simple mode" bypass for short prompts (< 50K tokens)
4. Cache REPL results for repeated patterns

### 5.2 Cost Variance

**Risk:** RLM costs are high-variance (some tasks much more expensive).

**Mitigations:**
1. Set cost budgets per task
2. Implement iteration limits with graceful degradation
3. Track and report cost before committing
4. Allow user to abort expensive operations

### 5.3 Security (Code Execution)

**Risk:** REPL executes LLM-generated code.

**Mitigations:**
1. Sandboxed environment (no network, limited filesystem)
2. Safe builtins only (no eval, exec)
3. Timeout on execution
4. Resource limits (memory, CPU)
5. Consider WASM isolation for production

### 5.4 Debugging Complexity

**Risk:** Recursive calls are hard to debug.

**Mitigations:**
1. Implement trajectory logging (like RLM's visualizer)
2. Add debug mode with step-by-step execution
3. Export trajectories as JSON for analysis
4. Build TUI integration for trajectory visualization

---

## Part 6: Implementation Checklist

### Phase 1: Foundation
- [ ] Create `codelet/core/src/rlm/` module structure
- [ ] Implement `REPLEnvironment` trait
- [ ] Port Python REPL via PyO3 (or implement Rhai alternative)
- [ ] Add safe builtin restrictions
- [ ] Write unit tests for code execution

### Phase 2: LM Handler
- [ ] Implement socket-based LM handler
- [ ] Add support for multiple clients (root vs sub)
- [ ] Implement usage tracking across depths
- [ ] Add batched request support
- [ ] Write integration tests

### Phase 3: Orchestrator
- [ ] Implement main completion loop
- [ ] Add code block extraction (```repl``` parsing)
- [ ] Implement FINAL/FINAL_VAR detection
- [ ] Add iteration and depth limits
- [ ] Integrate with existing provider system

### Phase 4: Tool Integration
- [ ] Expose existing tools as REPL functions
- [ ] Add tool result formatting
- [ ] Test tool combinations
- [ ] Document available functions

### Phase 5: TUI Integration
- [ ] Add NAPI bindings for RLM mode
- [ ] Update TypeScript types
- [ ] Add RLM mode toggle to TUI
- [ ] Implement trajectory visualization
- [ ] Add cost display

### Phase 6: Documentation & Testing
- [ ] Write user documentation
- [ ] Add feature specification (Gherkin)
- [ ] Write comprehensive integration tests
- [ ] Performance benchmarking
- [ ] Cost analysis vs compaction

---

## Part 7: Conclusion

RLM represents a paradigm shift in how LLMs handle long contexts. Instead of fighting the context window limit with lossy compression, RLM treats the context as external data that the LLM can programmatically explore.

**For codelet/fspec, this means:**
1. **No more context rot** - Information is never lost to summarization
2. **Truly massive context** - Analyze entire codebases (10M+ tokens)
3. **Better reasoning** - LLM can verify its own analysis via sub-calls
4. **Cost efficiency** - Selective context access vs full ingestion

**Recommended approach:** Implement RLM as an optional mode alongside the existing agent, allowing users to choose based on task complexity. Start with Phase 1-3 to validate the approach, then expand.

---

## References

- [RLM Paper (arXiv:2512.24601)](https://arxiv.org/abs/2512.24601)
- [RLM GitHub Repository](https://github.com/alexzhang13/rlm)
- [RLM Minimal Implementation](https://github.com/alexzhang13/rlm-minimal)
- [Blogpost: Recursive Language Models](https://alexzhang13.github.io/blog/2025/rlm/)

---

*Analysis prepared for codelet/fspec integration planning*
*Based on RLM source code at /tmp/rlm and codelet source at /Users/rquast/projects/fspec/codelet*
