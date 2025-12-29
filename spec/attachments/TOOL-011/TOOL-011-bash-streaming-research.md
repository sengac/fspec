# Comparative Analysis: Shell/Bash Tool Streaming in AI Coding Assistants

## Executive Summary

| Tool | Streams to UI? | Streams to LLM? | Subprocess Method | PTY Support |
|------|----------------|-----------------|-------------------|-------------|
| **opencode** | Yes (via metadata callbacks) | No (buffered) | `child_process.spawn()` | No |
| **aider** | Yes (real-time console) | No (full output on next turn) | `subprocess.Popen()` / `pexpect.spawn()` | Yes (pexpect) |
| **gemini-cli** | Yes (event-driven) | No (buffered) | `child_process.spawn()` / `node-pty` | Yes (xterm headless) |
| **letta** | No | No | `asyncio.create_subprocess_exec()` | No |
| **fspec/codelet** | No | No | `tokio::process::Command` | No |

---

## Detailed Analysis

### 1. fspec/codelet (Rust)

**Architecture**: Simplest implementation - fully synchronous buffered execution.

```rust
// codelet/tools/src/bash.rs:77-81
Command::new("sh").arg("-c").arg(&args.command).output()
```

**Key Characteristics**:
- Uses `tokio::process::Command` with `.output()` which waits for completion
- No streaming to UI during execution
- Output is truncated to `MAX_OUTPUT_CHARS` after completion
- Tool result is returned as a single string to LLM

**Strengths**:
- Simple, predictable behavior
- Clean error handling with exit codes

**Weaknesses**:
- No visual feedback during long-running commands
- User sees nothing until command completes

---

### 2. opencode (TypeScript/Node.js)

**Architecture**: Event-driven with metadata callbacks for UI updates, but LLM still receives buffered output.

```typescript
// packages/opencode/src/tool/bash.ts:198-232
const append = (chunk: Buffer) => {
  output += chunk.toString()
  ctx.metadata({
    metadata: { output, description: params.description }
  })
}
proc.stdout?.on("data", append)
proc.stderr?.on("data", append)
```

**Key Characteristics**:
- Uses `child_process.spawn()` with piped stdio
- Accumulates output in buffer, calls `ctx.metadata()` on each chunk
- Metadata updates propagate to UI via database writes
- Supports Anthropic's `fine-grained-tool-streaming` beta header
- 30KB output limit (`MAX_OUTPUT_LENGTH`)

**Strengths**:
- Users see output in real-time via metadata updates
- Database-backed state allows multiple UI clients to observe

**Weaknesses**:
- Not true streaming to LLM - full output sent after completion
- No PTY emulation - loses interactive capabilities

---

### 3. aider (Python)

**Architecture**: Dual-mode execution with console streaming and LLM message accumulation.

```python
# aider/run_cmd.py:42-86
process = subprocess.Popen(command, stdout=subprocess.PIPE, bufsize=0, ...)
while True:
    chunk = process.stdout.read(1)
    print(chunk, end="", flush=True)  # Real-time to console
    output.append(chunk)
return process.returncode, "".join(output)  # Full output returned
```

**Key Characteristics**:
- Character-by-character reading with `bufsize=0` for unbuffered output
- Prints to console in real-time as it executes
- Pexpect-based PTY support on Unix for interactive shells
- LLM streaming for **responses** (not tool output) via `show_send_output_stream()`
- Tool output added to chat on next turn after user confirmation

**Strengths**:
- True interactive PTY support (vim, git rebase -i work)
- Console sees output immediately
- Rich markdown streaming for LLM responses (20fps throttled)

**Weaknesses**:
- Shell command output is accumulated and sent to LLM only in next conversation turn
- Not integrated into the tool response itself

---

### 4. gemini-cli (TypeScript/Node.js)

**Architecture**: Most sophisticated - dual-mode with full PTY emulation and real-time streaming callbacks.

```typescript
// packages/core/src/services/shellExecutionService.ts:448-756
// PTY mode with xterm headless terminal emulation
processingChain = processingChain.then(async () => {
  const output = await terminalRenderer.render(xterm, data)
  emitOutputEvent(event, { type: 'data', chunk: output })
})
```

**Key Characteristics**:
- Two execution modes: PTY (`node-pty`) and child process fallback
- XTerm headless terminal for ANSI rendering
- Event-driven callback chain: `ShellExecutionService` -> `ShellToolInvocation` -> UI
- Debounced rendering (68ms) for smooth updates
- Binary stream detection and progress reporting
- Throttled output updates (`OUTPUT_UPDATE_INTERVAL_MS = 1000ms`)

**Strengths**:
- True interactive shell support (vim, interactive commands)
- Proper ANSI color/formatting preservation
- Smart binary detection prevents UI flooding
- Graceful cancellation with SIGTERM->SIGKILL escalation

**Weaknesses**:
- Complexity - multi-layered callback chain
- LLM still receives buffered output, not streaming chunks

---

### 5. letta (Python)

**Architecture**: Fully synchronous, marker-based output extraction, no streaming.

```python
# letta/services/tool_sandbox/local_sandbox.py:197-267
process = await asyncio.create_subprocess_exec(python_executable, temp_file_path,
    stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE)
stdout_bytes, stderr_bytes = await asyncio.wait_for(process.communicate(), timeout=...)
```

**Key Characteristics**:
- Uses `asyncio.create_subprocess_exec()` with blocking `communicate()`
- Marker-based result extraction from stdout (UUIDs demarcate result boundaries)
- E2B remote sandbox uses same pattern
- Streaming exists but only for **LLM response chunks**, not tool execution

**Strengths**:
- Deterministic result parsing with marker protocol
- Timeout handling via `asyncio.wait_for()`
- Works with remote sandboxes (E2B)

**Weaknesses**:
- Zero streaming feedback during execution
- All output buffered in memory

---

## Architectural Patterns Observed

### Pattern 1: Callback-Based UI Updates (opencode, gemini-cli)
```
stdout chunk -> callback -> database/UI update -> ... -> completion -> full result to LLM
```

### Pattern 2: Console Streaming + Accumulated Result (aider)
```
stdout chunk -> console print -> buffer accumulate -> ... -> completion -> user confirms -> LLM
```

### Pattern 3: Pure Buffered Execution (codelet, letta)
```
spawn -> wait for exit -> collect output -> return to LLM
```

---

## Key Insight: None Stream to LLM

**Critical finding**: None of these tools stream bash tool output directly to the LLM. They all:
1. Wait for command completion
2. Return the full (possibly truncated) output
3. LLM receives it as a single tool result

The "streaming" they implement is for **UI feedback** - showing users what's happening during execution - but the LLM conversation model remains request-response with complete tool results.

---

## Comparison with fspec/codelet

| Aspect | fspec/codelet | Others |
|--------|---------------|--------|
| **UI Feedback** | None during execution | opencode/aider/gemini stream to UI |
| **PTY Support** | No | aider (pexpect), gemini (node-pty) |
| **Implementation** | Simple `.output()` | Complex callback chains |
| **LLM Integration** | Same | Same (all buffer full output) |

### Opportunity for fspec/codelet

The current implementation is functionally correct but lacks user experience polish. Potential improvements:

1. **Add metadata callbacks** (like opencode) to emit partial output during execution
2. **Emit progress events** through the `StreamOutput` trait for TUI display
3. **Consider PTY mode** for interactive command support

However, the core LLM integration pattern is already optimal - streaming individual bash output characters to the LLM would be wasteful and not improve AI performance.

---

## Recommendations

**For fspec/codelet**:
1. **Keep the buffered LLM response** - this is the correct pattern
2. **Add UI streaming** - emit output chunks through existing `StreamOutput` trait
3. **Consider PTY** only if interactive shell support becomes a user requirement

**Implementation sketch**:
```rust
// Instead of:
Command::new("sh").arg("-c").arg(&args.command).output()

// Use:
let mut child = Command::new("sh")
    .arg("-c")
    .arg(&args.command)
    .stdout(Stdio::piped())
    .spawn()?;

let stdout = child.stdout.take().unwrap();
let mut reader = BufReader::new(stdout);
let mut line = String::new();
let mut output = String::new();

while reader.read_line(&mut line)? > 0 {
    output.push_str(&line);
    // Emit partial output for UI
    emit_tool_progress(&output);
    line.clear();
}
```

---

## Source Code Locations

### opencode
- Shell tool: `/tmp/opencode/packages/opencode/src/tool/bash.ts`
- Metadata handling: `/tmp/opencode/packages/opencode/src/session/prompt.ts`
- Fine-grained streaming header: `/tmp/opencode/packages/opencode/src/provider/provider.ts:77-80`

### aider
- Command execution: `/tmp/aider/aider/run_cmd.py`
- LLM streaming: `/tmp/aider/aider/coders/base_coder.py:1900-1972`
- Markdown streaming: `/tmp/aider/aider/mdstream.py`

### gemini-cli
- Shell execution service: `/tmp/gemini-cli/packages/core/src/services/shellExecutionService.ts`
- Shell tool: `/tmp/gemini-cli/packages/core/src/tools/shell.ts`
- UI processor: `/tmp/gemini-cli/packages/cli/src/ui/hooks/shellCommandProcessor.ts`

### letta
- Local sandbox: `/tmp/letta/letta/services/tool_sandbox/local_sandbox.py`
- Tool executor: `/tmp/letta/letta/services/tool_executor/tool_execution_sandbox.py`
- E2B sandbox: `/tmp/letta/letta/services/tool_sandbox/e2b_sandbox.py`

### fspec/codelet
- Bash tool: `/Users/rquast/projects/fspec/codelet/tools/src/bash.rs`
- Stream loop: `/Users/rquast/projects/fspec/codelet/cli/src/interactive/stream_loop.rs`

---

## Research Date
2025-12-29

## Codebases Analyzed
- opencode (TypeScript/Node.js)
- aider (Python)
- gemini-cli (TypeScript/Node.js)
- letta (Python)
- fspec/codelet (Rust)
