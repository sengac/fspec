# VTCode Lifecycle Hook System — Architecture Analysis & Adaptation Proposal for fspec

## Executive Summary

VTCode implements a **declarative, shell-based lifecycle hook system** that allows users to execute arbitrary commands at 8 key points in the agent lifecycle. Hooks are configured in TOML, compiled to regex matchers at startup, executed as child processes with JSON payloads on stdin, and their stdout/stderr is interpreted as structured outcomes that can **allow, deny, modify, or inject context** into the agent loop.

This document analyzes the VTCode architecture and proposes how fspec's Rust agent core (`codelet-core` / `codelet-napi`) should adopt the same pattern.

---

## Part 1: VTCode Architecture Deep Dive

### 1.1 Configuration Layer (`vtcode-config/src/hooks.rs`)

Hooks live in a TOML config file with this structure:

```toml
[hooks.lifecycle]
quiet_success_output = false  # suppress stdout from successful hooks

session_start = [
  { hooks = [{ command = "./hooks/setup-env.sh", timeout_seconds = 30 }] }
]

pre_tool_use = [
  { matcher = "Bash", hooks = [{ command = "./hooks/security-check.sh", timeout_seconds = 10 }] },
  { matcher = "Write|Edit", hooks = [{ command = "./hooks/lint.sh" }] },
  { matcher = ".*", hooks = [{ command = "./hooks/log.sh" }] }
]

post_tool_use = [
  { matcher = "Bash", hooks = [{ command = "./hooks/log-command.sh" }] }
]
```

**Type hierarchy:**

```
HooksConfig
└── LifecycleHooksConfig
    ├── quiet_success_output: bool
    ├── session_start:       Vec<HookGroupConfig>
    ├── session_end:         Vec<HookGroupConfig>
    ├── user_prompt_submit:  Vec<HookGroupConfig>
    ├── pre_tool_use:        Vec<HookGroupConfig>
    ├── post_tool_use:       Vec<HookGroupConfig>
    ├── task_completion:     Vec<HookGroupConfig>
    ├── task_completed:      Vec<HookGroupConfig>
    └── notification:        Vec<HookGroupConfig>

HookGroupConfig
├── matcher: Option<String>          // regex pattern (None = match all)
└── hooks: Vec<HookCommandConfig>    // sequential commands

HookCommandConfig
├── kind: HookCommandKind            // currently only `Command`
├── command: String                  // shell command string
└── timeout_seconds: Option<u64>     // default: 60s
```

### 1.2 Compilation Layer (`vtcode-core/src/hooks/lifecycle/compiled.rs`)

At engine creation, raw configs are compiled into `CompiledHookGroup`:

```rust
CompiledLifecycleHooks {
    quiet_success_output: bool,
    session_start:       Vec<CompiledHookGroup>,
    session_end:         Vec<CompiledHookGroup>,
    user_prompt_submit:  Vec<CompiledHookGroup>,
    pre_tool_use:        Vec<CompiledHookGroup>,
    post_tool_use:       Vec<CompiledHookGroup>,
    notification:        Vec<CompiledHookGroup>,
}

CompiledHookGroup {
    matcher: HookMatcher,               // pre-compiled
    commands: Vec<HookCommandConfig>,
}

enum HookMatcher {
    Any,                    // empty / "*" / no matcher → always matches
    Pattern(Regex),         // compiled as ^(?:PATTERN)$
}
```

**Validation at compile time:**
- Non-empty command strings
- Positive timeout values
- Valid regex patterns (compiled with `^(?:PATTERN)$` anchoring)
- At least one hook per group

### 1.3 Execution Engine (`vtcode-core/src/hooks/lifecycle/engine.rs`)

```rust
pub struct LifecycleHookEngine {
    inner: Arc<LifecycleHookInner>,  // Arc for Clone+Send across async boundaries
}

struct LifecycleHookInner {
    workspace: PathBuf,
    session_id: String,
    trigger: SessionStartTrigger,           // Startup or Resume
    hooks: CompiledLifecycleHooks,
    state: Mutex<LifecycleHookState>,       // mutable (transcript_path)
}
```

**Command execution (`execute_command`):**

1. Spawn `sh -c <command>` in workspace directory
2. Set environment variables:
   - `VT_PROJECT_DIR` — workspace path
   - `VT_SESSION_ID` — session ID
   - `VT_HOOK_EVENT` — event name (e.g., `"PreToolUse"`)
   - `VT_TRANSCRIPT_PATH` — transcript file path
3. Pipe JSON payload to stdin
4. Capture stdout/stderr via `tokio::spawn`
5. Apply timeout (default 60s). On timeout: `start_kill()` the process
6. Return `HookCommandResult { exit_code, stdout, stderr, timed_out, timeout_seconds }`

### 1.4 JSON Payloads (Piped to Hook's Stdin)

| Event | Key Fields |
|---|---|
| `SessionStart` | `session_id`, `cwd`, `source` (startup/resume), `transcript_path` |
| `SessionEnd` | `session_id`, `cwd`, `reason`, `transcript_path` |
| `UserPromptSubmit` | `session_id`, `cwd`, `prompt`, `transcript_path` |
| `PreToolUse` | `session_id`, `cwd`, `tool_name`, `tool_input`, `transcript_path` |
| `PostToolUse` | `session_id`, `cwd`, `tool_name`, `tool_input`, `tool_response`, `transcript_path` |
| `Notification` | `session_id`, `cwd`, `notification_type`, `title`, `message`, `transcript_path` |

All payloads include `hook_event_name` for disambiguation.

### 1.5 Interpretation Layer (`interpret/` modules)

**Common infrastructure:**
- `parse_json_output(stdout)` — attempts JSON parse
- `looks_like_json(stdout)` — heuristic check (starts with `{` or `[`)
- `extract_common_fields(json)` — extracts Claude Code-compatible fields:
  - `continue` / `stopReason`
  - `suppressOutput`
  - `systemMessage`
  - `decision` / `reason`
  - `hookSpecificOutput` (keyed by `hookEventName`)

**PreToolUse interpretation:**
| Signal | Outcome |
|--------|---------|
| Exit code 2 + stderr | **Deny** (blocks tool call) |
| Exit code 2 without stderr | Warning, continues |
| Timeout | **Deny** (safety) |
| JSON `continue: false` | **Deny** |
| JSON `hookSpecificOutput.permissionDecision: "allow"` | **Allow** (auto-approve, skip permission prompt) |
| JSON `hookSpecificOutput.permissionDecision: "deny"` | **Deny** |
| JSON `hookSpecificOutput.permissionDecision: "ask"` | **Ask** (force interactive prompt) |

**PostToolUse interpretation:**
| Signal | Outcome |
|--------|---------|
| JSON `decision: "block" + reason` | Block reason injected |
| JSON `hookSpecificOutput.additionalContext` | Injected as system message |
| Timeout/errors | Non-blocking (messages only) |

**UserPromptSubmit interpretation:**
| Signal | Outcome |
|--------|---------|
| Exit code 2 + stderr | Block prompt |
| JSON `continue: false` | Block prompt |
| JSON `decision: "block" + reason` | Block prompt |
| JSON `hookSpecificOutput.additionalContext` | Injected into model context |
| Plain text stdout | Added as additional context |

**SessionStart interpretation:**
- Plain stdout or JSON `additional_context` → injected as system messages

### 1.6 Outcome Types

```rust
enum PreToolHookDecision { Continue, Allow, Deny, Ask }
enum SessionStartTrigger { Startup, Resume }
enum SessionEndReason { Completed, Exit, Cancelled, Error, NewSession }
enum NotificationHookType { PermissionPrompt, IdlePrompt }
enum HookMessageLevel { Info, Warning, Error }

struct SessionStartHookOutcome { messages, additional_context }
struct UserPromptHookOutcome { allow_prompt, block_reason, additional_context, messages }
struct PreToolHookOutcome { decision (allow/deny/ask), messages }
struct PostToolHookOutcome { block_reason, additional_context, messages }
```

### 1.7 Agent Loop Integration Points

**Session Start** (session setup):
```
LifecycleHookEngine::new(workspace, config, trigger)
→ Register globally for notification system
→ hooks.run_session_start().await
→ Inject outcome.additional_context as system messages
```

**User Prompt Submit** (before agent processes input):
```
hooks.run_user_prompt_submit(prompt).await
→ If !outcome.allow_prompt: reject, loop back to input
→ Inject outcome.additional_context as system messages
```

**Pre-Tool Use** (in permission-checking gateway):
```
hooks.run_pre_tool_use(tool_name, tool_input).await
→ Match outcome.decision:
   Allow → auto-approve (skip permission UI)
   Deny  → reject tool call
   Ask   → force interactive prompt
   Continue → proceed with normal policy
```

**Post-Tool Use** (after tool execution):
```
hooks.run_post_tool_use(tool_name, tool_input, tool_output).await
→ Inject outcome.additional_context as system messages
→ Display messages
```

**Session End** (finalization):
```
hooks.run_session_end(reason).await
→ Display messages
→ Cleanup global notification engine
```

**Notifications** (global pathway via `OnceLock<RwLock<Option<Engine>>>`):
```
run_notification(type, title, message).await
→ Display messages
```

### 1.8 Threading Model

The `LifecycleHookEngine` is threaded through the agent loop via context structs:

```
SessionUISetup.lifecycle_hooks: Option<LifecycleHookEngine>  (owned)
  → session_loop_runner (borrows)
    → TurnLoop.lifecycle_hooks: Option<&'a LifecycleHookEngine>
      → TurnProcessingContext.lifecycle_hooks
        → InteractionLoopContext.lifecycle_hooks
          → ToolPermissionsContext.hooks
```

The notification hook engine is the one exception — stored globally via `OnceLock` because notifications can fire from outside the agent loop.

### 1.9 Security Boundary

Skills (the plugin system) are **explicitly forbidden** from defining hooks. The validation layer rejects any skill manifest that contains a `hooks` field. This prevents untrusted skill packages from hijacking the lifecycle.

---

## Part 2: Current fspec Rust Hook Architecture

### 2.1 What Exists Today

The fspec Rust codebase has **no general lifecycle hook system**. The word "hook" is used in two narrow contexts:

1. **`CompactionHook`** (`codelet-core/src/compaction_hook.rs`) — implements rig's `StreamingPromptHook` trait for token tracking and compaction triggering. This is an internal framework hook, not user-configurable.

2. **`GeminiHistoryHook`** (`codelet-core/src/gemini_history_hook.rs`) — decorator around `CompactionHook` for Gemini-specific thought signature injection. Also internal.

3. **fspec-hooks.json** — This exists in the TypeScript/fspec layer (the specification tool), NOT in the Rust agent core. It's a completely separate system for fspec CLI command lifecycle events.

### 2.2 Where Hooks Would Need to Integrate

Based on the architecture analysis:

| Hook Event | Integration Point | Crate |
|---|---|---|
| `session_start` | `agent_loop()` in `session_manager.rs` after session creation, before first prompt | `codelet-napi` |
| `session_end` | `agent_loop()` cleanup path, or `destroy_session()` | `codelet-napi` |
| `user_prompt_submit` | `agent_loop()` `tokio::select!` input branch, before calling `run_agent_stream_internal()` | `codelet-napi` |
| `pre_tool_use` | Inside rig's tool execution (tricky — rig auto-executes tools internally) | `codelet-tools` or `codelet-core` |
| `post_tool_use` | Same — would need rig hook or tool wrapper | `codelet-tools` or `codelet-core` |
| `notification` | Global — similar to VTCode's `OnceLock` pattern | `codelet-napi` |

### 2.3 The Tool Call Challenge

The biggest architectural challenge is **pre/post tool use hooks**. In VTCode, the agent loop has explicit control over tool execution. In fspec, **rig handles tool execution internally** during streaming:

```
RigAgent.prompt_streaming_with_history_and_hook()
  → rig framework auto-invokes Tool::call() when LLM returns tool_use
  → The stream loop only observes ToolCall/ToolResult items
```

**Options:**
1. **Wrapper tools** — wrap each tool's `call()` method to run pre/post hooks
2. **rig StreamingPromptHook** — extend the hook trait to intercept tool calls (requires rig patches)
3. **Tool trait middleware** — inject a middleware layer between rig and actual tools
4. **Stream loop interception** — modify the multi-turn loop to intercept before tool execution (requires rig changes)

Given that fspec already patches rig-core, option 2 or 4 may be viable. Option 1 (wrapper tools) is the least invasive.

---

## Part 3: Adaptation Proposal for fspec

### 3.1 Configuration

Adopt a similar TOML structure in fspec's configuration. The config could live in:
- `~/.fspec/credentials/hooks.toml` (user-level)
- `.codelet/hooks.toml` (project-level, takes precedence)
- Or a `[hooks]` section in an existing config file

```toml
[hooks.lifecycle]
quiet_success_output = false

session_start = [
  { hooks = [{ command = ".codelet/hooks/setup.sh", timeout_seconds = 30 }] }
]

pre_tool_use = [
  { matcher = "Bash", hooks = [{ command = ".codelet/hooks/security-check.sh", timeout_seconds = 10 }] },
  { matcher = "Write|Edit", hooks = [{ command = ".codelet/hooks/lint-on-save.sh" }] }
]

post_tool_use = [
  { matcher = ".*", hooks = [{ command = ".codelet/hooks/log-tool-usage.sh" }] }
]
```

### 3.2 Proposed Module Structure

```
codelet/
├── core/src/
│   └── hooks/
│       ├── mod.rs               ← Public API: LifecycleHookEngine
│       ├── config.rs            ← HooksConfig, HookGroupConfig, HookCommandConfig (serde)
│       ├── compiled.rs          ← CompiledLifecycleHooks, CompiledHookGroup, HookMatcher
│       ├── engine.rs            ← LifecycleHookEngine: execute_command, run_* methods
│       ├── payloads.rs          ← JSON payload structs for each event
│       ├── types.rs             ← Outcome enums and structs
│       └── interpret/
│           ├── mod.rs
│           ├── common.rs        ← parse_json_output, extract_common_fields
│           ├── tool.rs          ← PreToolUse/PostToolUse interpretation
│           ├── prompt.rs        ← UserPromptSubmit interpretation
│           └── session.rs       ← SessionStart/SessionEnd interpretation
```

### 3.3 Implementation Phases

#### Phase 1: Core Engine (codelet-core)
- Config types with serde deserialization (TOML)
- Regex compilation and validation
- Shell command execution with timeout (`tokio::process::Command`)
- JSON payload serialization
- Output interpretation (exit codes, JSON parsing)
- Outcome types

#### Phase 2: Session Lifecycle Hooks (codelet-napi)
- `session_start` — inject into `agent_loop()` after session setup
- `session_end` — inject into session cleanup/destroy
- Thread `LifecycleHookEngine` through `BackgroundSession`

#### Phase 3: User Prompt Hooks (codelet-napi)
- `user_prompt_submit` — inject into `agent_loop()` input handling branch
- Support prompt blocking (reject + loop back)
- Support context injection (additional system messages)

#### Phase 4: Tool Use Hooks (codelet-tools or codelet-core)
- `pre_tool_use` — requires solving the rig auto-execution challenge
- `post_tool_use` — same challenge
- **Recommended approach**: Tool wrapper middleware that intercepts `Tool::call()`
- Support Allow/Deny/Ask decisions for pre-tool
- Support context injection for post-tool

#### Phase 5: Notification Hooks (codelet-napi)
- Global hook engine (matching VTCode's `OnceLock` pattern)
- Permission prompt and idle prompt events

### 3.4 Environment Variables

Match VTCode's convention with fspec branding:

| Variable | Value |
|---|---|
| `CODELET_PROJECT_DIR` | Workspace/project directory |
| `CODELET_SESSION_ID` | Session UUID |
| `CODELET_HOOK_EVENT` | Event name (`PreToolUse`, `SessionStart`, etc.) |
| `CODELET_TRANSCRIPT_PATH` | Session transcript file path |

### 3.5 Claude Code Compatibility

VTCode's hook system is designed to be **compatible with Claude Code hooks** (same JSON response format). fspec should maintain this compatibility:

- Same exit code semantics (0 = success, 2 = deny/block)
- Same JSON response fields (`continue`, `decision`, `reason`, `hookSpecificOutput`)
- Same `permissionDecision` values (`allow`, `deny`, `ask`)

This allows users to reuse hook scripts across Claude Code, VTCode, and fspec.

### 3.6 Key Design Decisions

| Decision | Recommendation | Rationale |
|---|---|---|
| Config format | TOML (matching VTCode) | Consistent with Rust ecosystem conventions |
| Config location | `.codelet/hooks.toml` + user-level | Project-specific overrides user-level |
| Hook execution | `sh -c` via `tokio::process::Command` | Cross-platform, async, matches VTCode |
| JSON protocol | Claude Code-compatible | Ecosystem reuse |
| Tool hooks | Wrapper middleware on `Tool::call()` | Least invasive given rig's auto-execution |
| Threading | `Arc<LifecycleHookEngine>` in `BackgroundSession` | Matches existing session ownership model |
| Notification hooks | Global `OnceLock<RwLock<Option<Engine>>>` | Matches VTCode, notifications fire outside agent loop |

### 3.7 Risks & Mitigations

| Risk | Mitigation |
|---|---|
| Pre-tool hooks in rig's auto-execution loop | Implement as Tool::call() wrappers, or patch rig to emit pre-execution events |
| Hook timeout blocking agent loop | Tokio timeout with process kill (matching VTCode) |
| Hook stdout pollution | `quiet_success_output` flag + structured JSON detection |
| Security (malicious hooks) | Project-local only, no remote execution, user explicitly configures |
| Cross-platform shell | Use `sh -c` on Unix, `cmd /c` on Windows (matching VTCode) |

---

## Appendix A: VTCode Control Flow Diagram

```
Session Start
  └─→ LifecycleHookEngine::new() → compiled hooks
  └─→ run_session_start() → inject context as system messages

User Types Prompt
  └─→ run_user_prompt_submit(prompt)
       ├─ ALLOW → continue to agent
       └─ BLOCK → reject, loop back to input

Agent Requests Tool Call
  └─→ ensure_tool_permission()
       └─→ run_pre_tool_use(tool_name, tool_input)
            ├─ Allow → skip further permission checks
            ├─ Deny → reject tool call
            ├─ Ask → force interactive prompt
            └─ Continue → proceed with normal policy

Tool Executes
  └─→ run_post_tool_hooks(tool_name, input, output)
       └─→ inject additional_context as system messages

Notification Fired (PermissionPrompt/IdlePrompt)
  └─→ run_notification(type, title, message) via global engine

Session Ends
  └─→ run_session_end(reason) → display messages → cleanup global state
```

## Appendix B: VTCode File Organization Reference

```
vtcode-config/src/
  hooks.rs            ← Config types (HooksConfig, HookGroupConfig, HookCommandConfig)

vtcode-core/src/hooks/
  lifecycle/
    mod.rs            ← Re-exports
    compiled.rs       ← CompiledLifecycleHooks, CompiledHookGroup, HookMatcher
    engine.rs         ← LifecycleHookEngine (~650 lines)
    engine/
      payloads.rs     ← JSON payload structs
    interpret/
      mod.rs          ← Interpretation module
      common.rs       ← Shared parsing (parse_json_output, extract_common_fields)
      tool.rs         ← PreToolUse/PostToolUse interpretation
      prompt.rs       ← UserPromptSubmit interpretation
      session.rs      ← SessionStart/SessionEnd interpretation
    types.rs          ← Outcome enums and structs
```

## Appendix C: Claude Code Hook Protocol (JSON Response Format)

```json
{
  "continue": true,
  "suppressOutput": false,
  "systemMessage": "Hook executed successfully",
  "decision": "allow",
  "reason": "Approved by security policy",
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow",
    "additionalContext": "Remember to follow security guidelines"
  }
}
```
