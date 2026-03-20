# HOOK-013 Child Card Execution Order

## Dependency Graph

```
HOOK-014  Config & Compilation (3 pts, 13 scenarios)
    │
    ▼
HOOK-015  Execution Engine & Output Interpretation (5 pts, 19 scenarios)
    │
    ├──────────────────┐
    ▼                  ▼
HOOK-016           HOOK-017
Session &          Tool Use Hook
Notification       Integration
Integration        (8 pts, 3 scenarios)
(5 pts, 6 scenarios)
```

## Recommended Execution Order

### 1. HOOK-014 — Config & Compilation (3 points)
**No dependencies. Start here.**

Delivers the data model that everything else builds on:
- Serde types for HookGroup (with matcher) and HookDefinition (without matcher)
- Two-level config file loading (~/.fspec/fspec-hooks.json + spec/fspec-hooks.json)
- Merge logic (user-level first, project-level appended)
- Agent lifecycle event discrimination (ignore CLI events)
- Regex compilation with ^(?:PATTERN)$ anchoring
- None engine when no agent events configured

**Feature file scenarios:** 13 (tagged @HOOK-014)

### 2. HOOK-015 — Execution Engine & Output Interpretation (5 points)
**Depends on: HOOK-014**

Delivers the runtime engine that executes hooks and interprets results:
- Async shell execution via tokio::process::Command (sh -c)
- JSON payload serialization per event type piped to stdin
- FSPEC_* environment variables
- Timeout management with process kill (SIGKILL)
- Exit code interpretation (0 = success, 2 = deny/block, other = warning)
- Claude Code compatible JSON response parsing
- Context injection via additionalContext and plain text stdout

**Feature file scenarios:** 19 (tagged @HOOK-015)

### 3. HOOK-016 — Session & Notification Integration (5 points)
**Depends on: HOOK-015** (can run in parallel with HOOK-017)

Wires the engine into the agent loop for non-tool events:
- session_start: fires after session creation, before first prompt
- session_end: fires on session cleanup/destroy with reason
- user_prompt_submit: fires before agent processes input, can block
- notification: fires via global OnceLock engine outside agent loop
- BackgroundSession gains Option<LifecycleHookEngine> field
- Sequential command execution within hook groups

**Feature file scenarios:** 6 (tagged @HOOK-016)

### 4. HOOK-017 — Tool Use Hook Integration (8 points)
**Depends on: HOOK-015** (can run in parallel with HOOK-016)

The hardest card — intercepts rig's auto-execution of tools:
- Tool::call() wrapper middleware (or ToolSet::call interception)
- pre_tool_use: runs before tool executes, returns Allow/Deny/Ask/Continue
- post_tool_use: runs after tool executes, injects context
- Short-circuit logic: Allow/Deny stops evaluation, Continue passes through
- Integration with existing blocklist/stage-permissions permission model

**Feature file scenarios:** 3 (tagged @HOOK-017)

## Scenario Distribution

| Card | Scenarios | Points | Section in Feature File |
|------|-----------|--------|-------------------------|
| HOOK-014 | 13 | 3 | Config Loading & Merging, Hook Group Format, HookDefinition Format |
| HOOK-015 | 19 | 5 | Command Execution, Timeout Handling, Exit Code Interpretation, JSON Response |
| HOOK-016 | 6 | 5 | user_prompt_submit Blocking, session_end, notification, Sequential Execution |
| HOOK-017 | 3 | 8 | pre_tool_use Short-Circuit |
| **Total** | **41** | **21** | |

## Notes

- HOOK-016 and HOOK-017 are independent of each other and CAN be worked in parallel
- All scenarios live in a single shared feature file: `spec/features/agent-lifecycle-hooks.feature`
- Each scenario is tagged with its child card ID (@HOOK-014, @HOOK-015, etc.)
- The parent HOOK-013 feature file tag (@HOOK-013) covers all scenarios
