# HOOK-017: Tool Use Hook Integration (pre/post_tool_use)

## What This Card Delivers

The hardest card — intercepts rig's auto-execution of tools to run pre_tool_use and post_tool_use hooks. After this card is complete:
- pre_tool_use hooks run BEFORE every tool execution, can Allow/Deny/Ask/Continue
- post_tool_use hooks run AFTER every tool execution, can inject context
- Short-circuit logic stops evaluation on definitive Allow/Deny decisions
- The middleware integrates with the existing blocklist/stage-permissions model

## Depends On

- **HOOK-014** — config types, compiled hooks, matchers
- **HOOK-015** — execution engine and output interpretation

## Can Run In Parallel With

- **HOOK-016** (Session & Notification Integration) — independent integration point

## The Core Challenge

Rig auto-executes tools internally during streaming. The flow is:

```
LLM emits tool_use → rig deserializes args → ToolDyn::call(args)
    → Tool::call(self, typed_args) → returns result → rig continues
```

The agent code (session_manager.rs) only observes `StreamAssistantItem::ToolCall` and `StreamUserItem::ToolResult` items from the stream. By the time you see a ToolCall, rig has already decided to execute it.

## Recommended Approach: ToolSet::call() Interception

The cleanest interception point is in rig's `ToolSet::call()` method (in the patched rig-core). This is the single chokepoint where ALL tool calls flow through:

```rust
// In rig-core/src/tool/mod.rs, ToolSet::call()
pub async fn call(&self, toolname: &str, args: String) -> Result<String, ToolSetError> {
    // === PRE-TOOL HOOK INSERTION POINT ===
    // 1. Get the lifecycle hook engine (from thread-local or global)
    // 2. Check if any pre_tool_use groups match this tool name
    // 3. Execute matching hooks sequentially
    // 4. Interpret outcome:
    //    - Allow → proceed (skip further permission checks)
    //    - Deny → return error (tool not executed)
    //    - Ask → trigger interactive prompt (use existing PauseHandler)
    //    - Continue → proceed with normal policy (blocklist, stage perms)

    let tool = self.tools.get(toolname).ok_or(...)?;
    let result = tool.call(args).await?;

    // === POST-TOOL HOOK INSERTION POINT ===
    // 1. Execute matching post_tool_use hooks with tool_name, tool_input, tool_response
    // 2. Inject additional_context as system messages
    // 3. Display messages

    Ok(result)
}
```

### Alternative: Global Hook Registry

Since tools are executed on rig's async tasks (not the agent_loop thread), a global registry may be needed:

```rust
// Similar to existing patterns in codelet-tools (tool_pause, blocklist)
static LIFECYCLE_HOOK_ENGINE: RwLock<Option<Arc<LifecycleHookEngine>>> = RwLock::new(None);

pub fn set_tool_hook_engine(engine: Option<Arc<LifecycleHookEngine>>) { ... }
pub fn get_tool_hook_engine() -> Option<Arc<LifecycleHookEngine>> { ... }
```

Set in agent_loop before each turn, cleared after.

### Hooking Into ToolSet vs Wrapping Each Tool

**Option A: Patch ToolSet::call()** (recommended)
- Single interception point
- Catches ALL tools (static, facade, MCP)
- Requires modifying the patched rig-core
- Already in `codelet/patches/rig-core/`

**Option B: Wrap individual Tool::call()**
- More granular but misses MCP/dynamic tools
- Requires wrapping every tool struct
- More code, harder to maintain

**Option C: Wrap at FacadeToolWrapper level**
- Catches facade tools only
- Misses Claude-native tools (ReadTool, BashTool direct)
- Incomplete coverage

**Recommendation: Option A** — patch ToolSet::call() in rig-core.

## pre_tool_use Short-Circuit Logic

When multiple hook groups match the same tool:

```rust
let mut final_decision = PreToolHookDecision::Continue;

for group in matching_groups {
    let outcome = execute_hook_group(group, &payload).await;
    match outcome.decision {
        PreToolHookDecision::Allow => {
            final_decision = Allow;
            break;  // Short-circuit: definitive decision
        }
        PreToolHookDecision::Deny => {
            final_decision = Deny;
            break;  // Short-circuit: definitive decision
        }
        PreToolHookDecision::Ask => {
            final_decision = Ask;
            break;  // Short-circuit: definitive decision
        }
        PreToolHookDecision::Continue => {
            // No opinion — continue to next group
        }
    }
}

match final_decision {
    Allow => { /* proceed, skip permission checks */ }
    Deny => { /* return error, tool not executed */ }
    Ask => { /* trigger interactive prompt via PauseHandler */ }
    Continue => { /* proceed with normal permission policy */ }
}
```

## Integration with Existing Permission Model

The pre_tool_use hook result feeds into the existing permission chain:

```
1. pre_tool_use hooks → Allow/Deny/Ask/Continue
2. If Allow → SKIP blocklist + stage perms, execute tool
3. If Deny → REJECT tool call, do not execute
4. If Ask → trigger PauseHandler (same as blocklist Prompt action)
5. If Continue → fall through to normal checks:
   a. Blocklist check (check_bash_command / check_file_path)
   b. Stage permissions check (check_write_permission)
   c. Execute tool
```

## post_tool_use Integration

After tool execution completes:

```rust
if let Some(engine) = get_tool_hook_engine() {
    let payload = PostToolUsePayload {
        tool_name: toolname.to_string(),
        tool_input: args_value,
        tool_response: result.clone(),
        // ... common fields
    };
    let outcome = engine.run_post_tool_use(&payload).await;
    // Inject outcome.additional_context as system messages
    // Display outcome.messages
}
```

Post-tool hooks are **non-blocking** — failures result in warnings, not tool rejection.

## Scenarios (3)

All tagged `@HOOK-017` in `spec/features/agent-lifecycle-hooks.feature`:
- pre_tool_use short-circuits on Allow (1)
- pre_tool_use short-circuits on Deny (1)
- pre_tool_use Continue does not short-circuit (1)

Note: Only 3 scenarios because the interpretation logic is tested in HOOK-015. This card focuses on the middleware integration and short-circuit behavior.
