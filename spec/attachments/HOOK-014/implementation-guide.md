# HOOK-014: Hook Config Data Model — Loading, Merging & Compilation

## What This Card Delivers

The foundational data layer that all other hook cards build upon. After this card is complete, the Rust codebase can:
- Parse agent lifecycle hook entries from fspec-hooks.json
- Load from both user-level and project-level config files
- Merge hooks from both levels (concatenate, user-first)
- Compile regex matchers into efficient pre-compiled patterns
- Distinguish agent lifecycle events from fspec CLI command events
- Return `None` when no agent lifecycle events are configured

## Config Format

Agent lifecycle hooks live in the **same `fspec-hooks.json`** file as existing fspec CLI hooks. The key difference is the format of the hook entries:

### fspec CLI events (existing, unchanged):
```json
{
  "hooks": {
    "pre-update-work-unit-status": [
      { "name": "lint", "command": "npm run lint", "blocking": true, "timeout": 30 }
    ]
  }
}
```
These are `HookDefinition[]` — flat arrays of `{ name, command, blocking?, timeout?, condition? }`.

### Agent lifecycle events (NEW — this card):

**Non-tool events** use the same `HookDefinition[]` format (no matcher needed):
```json
{
  "hooks": {
    "session_start": [
      { "name": "setup-env", "command": "./hooks/setup.sh", "timeout": 30 }
    ],
    "user_prompt_submit": [
      { "name": "policy-check", "command": "./hooks/policy.sh", "blocking": true }
    ]
  }
}
```

**Tool events** use `HookGroup[]` format (with optional matcher):
```json
{
  "hooks": {
    "pre_tool_use": [
      { "matcher": "Bash", "hooks": [{ "command": "./hooks/security.sh", "timeout": 10 }] },
      { "matcher": "Write|Edit", "hooks": [{ "command": "./hooks/lint.sh" }] },
      { "hooks": [{ "command": "./hooks/log.sh" }] }
    ]
  }
}
```

### Agent lifecycle event keys (6 total):
- `session_start` — HookDefinition[]
- `session_end` — HookDefinition[]
- `user_prompt_submit` — HookDefinition[]
- `notification` — HookDefinition[]
- `pre_tool_use` — HookGroup[]
- `post_tool_use` — HookGroup[]

### Non-agent event keys (ignored by Rust engine):
Any key containing a hyphen (e.g. `pre-update-work-unit-status`, `post-implementing`) is a fspec CLI command event and should be skipped.

## Config Paths

1. **User-level**: `~/.fspec/fspec-hooks.json`
2. **Project-level**: `spec/fspec-hooks.json` (relative to project root)

Both are optional. If neither exists, return `None` engine.

## Merge Strategy

- Load user-level config first
- Load project-level config second
- For each agent lifecycle event key, **concatenate** the arrays: user-level hooks first, then project-level hooks appended after
- This means both execute, with user-level running first

## Regex Compilation

For `pre_tool_use` and `post_tool_use` hook groups:
- If `matcher` is absent or empty string → match ALL tool names
- If `matcher` is present → compile as `^(?:PATTERN)$` (full-match anchoring)
- Invalid regex → **error** that prevents engine creation (fail fast at startup)

## Rust Types to Implement

```rust
// Config types (serde deserialization)
struct FspecHooksConfig {
    global: Option<GlobalConfig>,
    hooks: HashMap<String, serde_json::Value>,  // polymorphic — parsed per event type
}

struct GlobalConfig {
    timeout: Option<u64>,
    shell: Option<String>,
}

struct HookDefinition {
    name: String,
    command: String,
    blocking: Option<bool>,
    timeout: Option<u64>,
    // condition field exists in schema but NOT used for agent hooks (rule 31)
}

struct HookGroupConfig {
    matcher: Option<String>,
    hooks: Vec<HookCommandConfig>,
}

struct HookCommandConfig {
    command: String,
    timeout: Option<u64>,
}

// Compiled types
struct CompiledLifecycleHooks {
    global_timeout: u64,  // default 60
    session_start: Vec<CompiledHookDefinition>,
    session_end: Vec<CompiledHookDefinition>,
    user_prompt_submit: Vec<CompiledHookDefinition>,
    notification: Vec<CompiledHookDefinition>,
    pre_tool_use: Vec<CompiledHookGroup>,
    post_tool_use: Vec<CompiledHookGroup>,
}

struct CompiledHookDefinition {
    name: String,
    command: String,
    blocking: bool,
    timeout: u64,
}

struct CompiledHookGroup {
    matcher: HookMatcher,
    commands: Vec<HookCommandConfig>,
}

enum HookMatcher {
    Any,                    // empty/absent matcher
    Pattern(regex::Regex),  // compiled ^(?:PATTERN)$
}
```

## Scenarios (13)

All tagged `@HOOK-014` in `spec/features/agent-lifecycle-hooks.feature`:
- Config Loading & Merging (8)
- Hook Group Format — pre_tool_use/post_tool_use (3)
- HookDefinition Format — session/prompt/notification (2)
