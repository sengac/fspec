# BUG-120: Role Injection Gap Analysis

## Summary

The `/role` command and `set_role` AgentManager action store a role string in memory but **never inject it into the LLM conversation**. The role is described everywhere as a "system prompt overlay" but has zero effect on AI behavior.

## Root Cause

In `codelet/napi/src/session_manager.rs`, the `run_with_provider!` macro (line 3836) always passes `None` as the preamble:

```rust
let agent = provider.create_rig_agent($session.id, None, $thinking.clone());
//                                               ^^^^
//                                    preamble is ALWAYS None
```

The role is never read from `$session.get_role()` and never forwarded to `create_rig_agent()`.

## Full Trace

### Storage Layer ✅ (Works)

- **File:** `codelet/napi/src/session_manager.rs`
- `BackgroundSession.role: RwLock<Option<String>>` (line 511)
- `set_role(&self, role: String)` → writes `Some(role)` (line 916)
- `get_role(&self) -> Option<String>` → reads (line 924)
- `clear_role(&self)` → writes `None` (line 932)

### NAPI Bindings ✅ (Works)

- `session_set_role(session_id, role_name, ...)` (line 5531)
- `session_get_role(session_id)` → `Option<SupervisorRoleInfo>` (line 5547)

### AgentManager Handler ✅ (Works for metadata)

- `handle_list`: Reports role in session entries
- `handle_get_status`: Reports role in status response
- `handle_set_role`: Calls `session.set_role()` / `session.clear_role()`
- `handle_message`: Uses sender's role for message attribution

### TUI `/role` Command ✅ (Works for dialog)

- **File:** `src/tui/components/AgentView.tsx`
- Opens `RoleDialog` component
- On submit: calls `sessionSetRole(currentSessionId, role)`
- On empty: calls `sessionSetRole(currentSessionId, '')`

### LLM System Prompt ❌ (THE GAP)

- `run_with_provider!` macro: `preamble` is hardcoded `None`
- `create_rig_agent()`: Receives `preamble: Option<&str>`, passes through `SystemPromptFacade` to build system prompt. When `None`, only facade defaults are used.
- No `SystemReminderType::Role` variant exists
- No code anywhere reads `BackgroundSession.role` and injects it into messages

## Two Possible Fix Approaches

### Approach A: Inject as Preamble (System Prompt) ← CHOSEN

Read `session.get_role()` in the `run_with_provider!` macro and pass it as the preamble:

```rust
let role = $session.get_role();
let preamble = role.as_deref();
let agent = provider.create_rig_agent($session.id, preamble, $thinking.clone());
```

Each provider handles preamble differently — the role is incorporated into the system prompt
through each provider's existing system prompt pipeline:

| Provider | Preamble Handling | Role Effect |
|----------|------------------|-------------|
| Claude OAuth | `SystemPromptFacade.transform_preamble()` | Role embedded in Claude Code prefix + fspec guidance |
| Claude API Key | Same facade path | Same as OAuth |
| Gemini | `build_gemini_system_prompt(model, preamble)` | Role appended as Project-Specific Instructions |
| OpenAI | `prepend_fspec_guidance(preamble)` | Role appended after fspec guidance |
| ZAI | Same as OpenAI | Same as OpenAI |
| Codex | `format!("{CODEX_BASE_INSTRUCTIONS}\n\n{role}")` | Role appended after base instructions (base always preserved) |

**Pros:** Cleanest — role becomes part of the system prompt on every turn. Each provider handles
it through its established pipeline. No new infrastructure needed.

### Approach B: Inject as System Reminder (NOT CHOSEN)

Add a new `SystemReminderType::Role` variant and inject the role as a system reminder message.

**Not chosen because:** Approach A is simpler — it reuses existing preamble infrastructure that
all providers already support. The preamble is rebuilt fresh each turn from `get_role()`, so
role changes are automatically reflected without managing system reminder state.

## Files Modified

1. **`codelet/napi/src/session_manager.rs`** — `run_with_provider!` macro reads `session.get_role()` and passes as preamble
2. **`codelet/providers/src/openai.rs`** — Use `prepend_fspec_guidance()` to always include fspec guidance (with or without role)
3. **`codelet/providers/src/zai.rs`** — Same as OpenAI: always include fspec guidance
4. **`codelet/providers/src/codex/mod.rs`** — Append role after `CODEX_BASE_INSTRUCTIONS` instead of replacing them
