# AMGR-013: AgentManager Spawn Provider Name Mismatch

## Root Cause

There are **two different naming conventions** for providers in this codebase:

| Layer | Anthropic | Google | OpenAI | ZAI |
|-------|-----------|--------|--------|-----|
| **models.dev registry** (external) | `"anthropic"` | `"google"` | `"openai"` | `"zai"` |
| **ProviderType enum** (internal) | `"claude"` | `"gemini"` | `"openai"` | `"zai"` |

Two providers have **mismatched names**: `anthropic↔claude` and `google↔gemini`.

## The Bug Flow

1. `session_manager.rs:4279` — reads `inner_session.current_provider_name()` → returns `"claude"` (internal name)
2. `agent_manager_handler.rs:82-91` — builds `format!("{provider}/{model}")` → `"claude/claude-opus-4-6"`
3. `session_manager.rs:3092` — `create_session_with_id()` receives `"claude/claude-opus-4-6"`
4. Inside `create_session_with_id`, `select_model()` → `parse_model_string()` splits to `("claude", "claude-opus-4-6")`
5. Registry lookup: `self.providers.contains_key("claude")` → **FALSE** — registry keys use models.dev names
6. Error: `"Unknown provider: 'claude'. Available providers: anthropic, ..."`

## Affected Providers

- **Anthropic**: `current_provider_name()` returns `"claude"`, registry expects `"anthropic"` → **BROKEN**
- **Google/Gemini**: `current_provider_name()` returns `"gemini"`, registry expects `"google"` → **BROKEN**
- **OpenAI**: returns `"openai"`, registry expects `"openai"` → OK (names match)
- **Codex**: returns `"codex"`, registry expects `"codex"` → OK (names match)
- **ZAI**: returns `"zai"`, registry expects `"zai"` → OK (names match)

## Fix Options

### Option A: Map internal names back to registry names in the handler (Recommended)
Add a reverse mapping function in `agent_manager_handler.rs`:

```rust
fn internal_to_registry_provider(name: &str) -> &str {
    match name {
        "claude" => "anthropic",
        "gemini" => "google",
        other => other, // openai, codex, zai are same in both
    }
}
```

Apply it in `handle_spawn` before building the model string:
```rust
let registry_provider = internal_to_registry_provider(provider);
let model_string = format!("{registry_provider}/{model}");
```

**Pros**: Minimal change, contained to one file, no API changes.
**Cons**: Another hardcoded mapping to maintain.

### Option B: Store the original registry provider name on the session
Add `registry_provider_name: String` field to session that preserves the original models.dev name used during model selection. Pass this through to `create_handler` instead of `current_provider_name()`.

**Pros**: Eliminates the naming ambiguity at the source.
**Cons**: Larger change, touches session state.

### Option C: Make `create_session_with_id` accept internal provider names
Add the reverse mapping inside `select_model()` so it can accept either naming convention.

**Pros**: Fix once at the API boundary.
**Cons**: Widens the API to accept two different name formats.

## Recommendation

**Option A** — simplest, lowest risk, fixes the immediate bug. The mapping function can have a comment explaining the naming asymmetry and linking to this analysis.

## Key Files

- `codelet/napi/src/agent_manager_handler.rs` — lines 82-114 (handle_spawn)
- `codelet/napi/src/session_manager.rs` — lines 4279-4287 (handler registration)
- `codelet/providers/src/manager.rs` — lines 374-376 (current_provider_name)
- `codelet/providers/src/manager.rs` — map_provider_id_to_type (forward mapping exists, no reverse)
