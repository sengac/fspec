# TOOL-020: Positional `_` Args Analysis

## Problem Statement

The Rust fspec tool guidance (`codelet/tools/src/fspec_workflow_guidance.rs`) teaches LLMs to use positional `_` arguments in the format:

```json
{
  "command": "update-work-unit-status",
  "args": {"_": ["AUTH-001", "specifying"]}
}
```

However, the Rust fspec-core commands expect **named keys**:

```json
{
  "command": "update-work-unit-status",
  "args": {"workUnitId": "AUTH-001", "status": "specifying"}
}
```

This creates a **deserialization mismatch** when the Rust dispatch path executes ported commands directly.

---

## Current Architecture

### Two Execution Paths

```
┌─────────────────────────────────────────────────────────────────────┐
│                     LLM Tool Call                                    │
│  command: "update-work-unit-status"                                  │
│  args: {"_": ["AUTH-001", "specifying"]}                            │
└──────────────────┬──────────────────────────────────────────────────┘
                   │
        ┌──────────┴──────────┐
        │                     │
   PATH A (TypeScript)   PATH B (Rust)
   ┌──────────────┐      ┌──────────────┐
   │ fspec-callback│      │ dispatch.rs  │
   │ .ts:956-966  │      │ run_ported() │
   │              │      │              │
   │ Extracts `_` │      │ Receives raw │
   │ Pushes to    │      │ args_json    │
   │ argv[]       │      │              │
   │ Commander.js │      │ serde_json   │
   │ parses argv  │      │ deserializes │
   │              │      │              │
   │ ✅ WORKS     │      │ ❌ FAILS     │
   │ (positional  │      │ (expects     │
   │  → named)    │      │  named keys) │
   └──────────────┘      └──────────────┘
```

### Path A: TypeScript Callback (Works)

**File:** `src/utils/fspec-callback.ts:956-966`

```typescript
// Extracts positional args from `_` key
const positionalArgs = args._ as unknown[] | undefined;
if (Array.isArray(positionalArgs)) {
  for (const arg of positionalArgs) {
    if (arg !== undefined && arg !== null) {
      argv.push(normalizeFilePath(String(arg)));
    }
  }
}
```

Commander.js then parses the argv array and matches positional arguments to the command's `.argument('<workUnitId>')` definitions. This works because Commander.js has the positional-to-named mapping built in.

### Path B: Rust Dispatch (Fails)

**File:** `codelet/fspec-core/src/commands/update_work_unit_status.rs:20-31`

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateWorkUnitStatusArgs {
    work_unit_id: String,
    status: String,
    #[serde(default)]
    blocked_reason: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    skip_temporal_validation: Option<bool>,
}

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: UpdateWorkUnitStatusArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "update-work-unit-status",
            reason: format!("failed to parse args: {e}"),
        })?;
    // ...
}
```

When `args_json` is `{"_": ["AUTH-001", "specifying"]}`, serde_json fails because:
- There's no `work_unit_id` field in the JSON
- There's no `status` field in the JSON
- The `_` key doesn't map to any struct field

**Error:** `failed to parse args: missing field `workUnitId``

---

## Impact Assessment

### Commands Affected

**ALL 162 ported commands** use named keys in their args structs. Every command that accepts positional arguments in the guidance is affected:

| Command | Guidance Format | Rust Expects |
|---------|----------------|--------------|
| `show-work-unit` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `update-work-unit-status` | `{"_": ["AUTH-001", "specifying"]}` | `{"workUnitId": "AUTH-001", "status": "specifying"}` |
| `add-rule` | `{"_": ["AUTH-001", "Password must be 8+ chars"]}` | `{"workUnitId": "AUTH-001", "rule": "Password must be 8+ chars"}` |
| `add-example` | `{"_": ["AUTH-001", "User enters valid credentials"]}` | `{"workUnitId": "AUTH-001", "example": "User enters valid credentials"}` |
| `add-question` | `{"_": ["AUTH-001", "@human: What happens?"]}` | `{"workUnitId": "AUTH-001", "question": "@human: What happens?"}` |
| `remove-rule` | `{"_": ["AUTH-001", "0"]}` | `{"workUnitId": "AUTH-001", "id": "0"}` |
| `set-user-story` | `{"_": ["AUTH-001"], "role": "user", "action": "log in"}` | `{"workUnitId": "AUTH-001", "role": "user", "action": "log in"}` |
| `create-story` | `{"_": ["AUTH", "User Login"]}` | `{"prefix": "AUTH", "title": "User Login"}` |
| `add-dependency` | `{"_": ["AUTH-002", "AUTH-001"]}` | `{"workUnitId": "AUTH-002", "dependsOn": "AUTH-001"}` |
| `delete-work-unit` | `{"_": ["AUTH-001"]}` | `{"workUnitId": "AUTH-001"}` |
| `prioritize-work-unit` | `{"_": ["AUTH-001"], "position": "top"}` | `{"workUnitId": "AUTH-001", "position": "top"}` |
| `show-coverage` | `{"_": ["user-auth"]}` | `{"feature": "user-auth"}` |
| `link-coverage` | `{"_": ["user-auth"], "scenario": "Login"}` | `{"feature": "user-auth", "scenario": "Login"}` |
| `show-feature` | `{"_": ["user-auth"]}` | `{"feature": "user-auth"}` |
| `add-rule` | `{"_": ["AUTH-001", "rule text"]}` | `{"workUnitId": "AUTH-001", "rule": "rule text"}` |

### Scope of Change

**File:** `codelet/tools/src/fspec_workflow_guidance.rs`

- **~100+ command examples** need updating
- **Every `_` pattern** must be replaced with named keys
- **No code changes** to fspec-core commands (they already expect named keys)
- **No code changes** to the dispatch layer (it already passes args_json as-is)

---

## Why This Matters

### 1. Rust Port Correctness

The Rust port is the **future** of fspec. The TypeScript CLI is being phased out. The guidance must teach the LLM to use the format that works with the Rust implementation.

### 2. Simplicity

Named keys are **more explicit** and **easier to understand** than positional indices. The LLM doesn't need to remember the order of arguments.

### 3. Type Safety

Named keys provide **compile-time validation** via serde deserialization. Positional args require runtime mapping that can break silently.

### 4. Consistency

The Rust CLI bridge layer already uses named keys. The guidance should match the implementation.

---

## Proposed Solution

Replace ALL `_` positional patterns in `fspec_workflow_guidance.rs` with named keys:

**Before:**
```
command: "update-work-unit-status", args: {"_": ["AUTH-001", "specifying"]}
```

**After:**
```
command: "update-work-unit-status", args: {"workUnitId": "AUTH-001", "status": "specifying"}
```

This is a **documentation-only change** to the Rust tool guidance. The TypeScript callback layer can continue to support `_` for backward compatibility, but the guidance should teach the named-key format that works with both paths.

---

## Next Steps

1. Create a mapping document for all 162 commands showing the named-key format
2. Update `fspec_workflow_guidance.rs` with named keys throughout
3. Verify all examples compile correctly with the Rust dispatch path
4. Add tests to prevent regression
