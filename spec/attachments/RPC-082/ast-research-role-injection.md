# RPC-082 AST Research: Role Injection Sites

**Task:** Verify all per-turn agent dispatch sites read `session.get_role()` and pass it as the `preamble` argument to `create_rig_agent`.

**Method:** AST search across `codelet/agent-loop/src/`, `codelet/sessions/src/`, and `codelet/providers/src/`.

---

## 1. Role read sites — `session.get_role()` bindings in the agent loop

```
AstGrep pattern: let role_preamble = session.get_role();
path: codelet/agent-loop/src/
```

Matches (2):
- `codelet/agent-loop/src/agent_loop.rs:878` — **OpenAI inlined arm**
- `codelet/agent-loop/src/agent_loop.rs:965` — **Custom-provider fallthrough arm (`_ =>`)**

The third site (the `run_with_provider!` macro body covering claude/gemini/zai/codex/copilot/github-copilot) lives in the macro definition at `codelet/agent-loop/src/dispatch.rs:56` and is matched textually rather than via AST because the `$session` meta-variable in a macro pattern is not parseable as a regular Rust expression by the ast-grep Rust parser. Verified via the `dispatch.rs:53-62` excerpt:

```rust
// BUG-120: Read session role and pass as preamble so it becomes
// part of the system prompt. All providers handle preamble via
// SystemPromptFacade — the role text is prepended to fspec guidance.
let role_preamble = $session.get_role();
// TOOL-012: Pass session.id as first parameter so tools store it at construction
let agent = provider.create_rig_agent(
    $session.id,
    role_preamble.as_deref(),
    $thinking.clone(),
);
```

---

## 2. BackgroundSession getter/setter

```
AstGrep pattern: pub fn get_role(&self) -> Option<String> { $$$BODY }
path: codelet/sessions/src/
```

Match (1):
- `codelet/sessions/src/background_session.rs:839` — canonical implementation reading `self.role.read().expect("role lock poisoned").clone()`.

Companion methods at adjacent lines:
- `set_role` at `:832`
- `clear_role` at `:846`

---

## 3. Per-provider `create_rig_agent` signature sites

```
AstGrep pattern: pub fn create_rig_agent($$$ARGS) -> $RET { $$$BODY }
path: codelet/providers/src/
```

Matches (7):
- `codelet/providers/src/claude.rs:507`
- `codelet/providers/src/openai.rs:410`
- `codelet/providers/src/gemini.rs:130`
- `codelet/providers/src/zai.rs:218`
- `codelet/providers/src/codex/mod.rs:331`
- `codelet/providers/src/copilot/rig_agent.rs:56`
- `codelet/providers/src/custom/custom_provider.rs:110`

This is the canonical 7-provider surface. The agent loop dispatches all of them through the same `(session_id, preamble: Option<&str>, thinking)` shape. Compile-time closure assertions in the test suite will pin this signature for every provider.

---

## 4. Existing parity test pattern (precedent)

`codelet/agent-loop/src/dispatch.rs:198-214` defines:

```rust
#[test]
fn copilot_create_rig_agent_signature_matches_dispatch_macro_contract() {
    use codelet_providers::copilot::CopilotProvider;
    let _create_rig_agent_ref = |provider: &CopilotProvider,
                                 session_id: uuid::Uuid,
                                 preamble: Option<&str>,
                                 thinking: Option<serde_json::Value>| {
        provider.create_rig_agent(session_id, preamble, thinking)
    };
    // ...
}
```

RPC-082 generalises this pattern to all 7 providers — claude/openai/gemini/zai/codex/copilot/custom — proving each can absorb a `preamble: Option<&str>` and that the dispatch macro's substitution is type-safe everywhere.

---

## 5. Conclusion / test plan

| Test                                                          | Source under test                                          | Mechanism                       |
| ------------------------------------------------------------- | ---------------------------------------------------------- | ------------------------------- |
| `set_role / get_role / clear_role round-trip`                 | `codelet/sessions/src/background_session.rs:832-848`        | Behavioral unit test on `BackgroundSession` |
| `run_with_provider! macro reads session.get_role()`           | `codelet/agent-loop/src/dispatch.rs` macro body            | String-based source structural test |
| `OpenAI inlined arm reads session.get_role()`                 | `codelet/agent-loop/src/agent_loop.rs:867-913`              | String-based source structural test |
| `Custom-provider fallthrough reads session.get_role()`        | `codelet/agent-loop/src/agent_loop.rs:953-...`              | String-based source structural test |
| `Every provider's create_rig_agent accepts Option<&str>`      | All 7 providers in `codelet/providers/src/`                | Compile-time closure assertion (7 cases) |

All required sites already exist in source — RPC-082 is a coverage card landing the structural parity tests that pin the BUG-120 contract.
