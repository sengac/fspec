# BUG-344 root cause: rpc082/rpc083 read provider arms that moved

## Symptom

`cargo test --profile ci-test -p codelet-agent-loop --test rpc082_role_injection` (3 of 4 fail)
and `--test rpc083_tool_registry` (3 of 7 fail):

- `openai_inlined_arm_*` / `openai_inlined_arm_contains_five_canonical_calls` panic at
  `src.find("\"openai\" => {")` in `agent_loop.rs` (rpc082:92, rpc083:83).
- `custom_provider_fallthrough_*` panic at `src.find("CustomProvider::create_rig_agent")` in
  `agent_loop.rs` (rpc082:124, rpc083:115).
- `run_with_provider_macro_*` fail (rpc082:251, rpc083:164): the extractor takes only the body after
  the macro's FIRST `=> {` — the `(provider $inner:expr, $getter:ident, $session:expr)` arm — while
  the agent build (`gather_mcp_tool_wrappers`, `role_preamble`, `create_rig_agent`) is in a later
  arm (rust/agent-loop/src/dispatch.rs:37-80).

## Cause

f0eb0759 (2026-10-25, "harden transcript persistence and recovery") moved the OpenAI turn into
`rust/agent-loop/src/agent_loop/provider_openai.rs` (`run_openai_turn`) and the custom/stub turns
into `agent_loop/provider_custom.rs` (`run_custom_turn`, `run_stub_turn`); `agent_loop/providers.rs`
now only dispatches to them, and `run_with_provider!` gained arms. The source-shape tests kept
reading `agent_loop.rs` and the first macro arm.

Not caused by REFAC-031: `git merge-base --is-ancestor f0eb0759 f167f1f9` holds, and
rust/agent-loop/src has not changed since 0f1472c5; REFAC-031 only touched rust/providers.

## Product check

The wiring itself is intact (fspec astgrep, ast-research.md): `run_openai_turn` binds
`role_preamble = session.get_role()` and calls `provider.create_rig_agent(session.id,
role_preamble.as_deref(), thinking_config_value)`, then adds MCP wrappers, stores the tool server
handle and wraps the agent in `RigAgent::with_default_depth`; `run_custom_turn` does the same around
`CustomProvider::create_rig_agent(.., session.id, role_preamble.as_deref(), ..)`; the macro does it
for every macro-driven provider. Only the tests are wrong.

## Fix

- New shared reader `rust/agent-loop/tests/provider_sources/support.rs`: `read_source`,
  `macro_body` (the whole macro, every arm), `function_body` and `assert_in_order`; a missing marker
  panics naming it (never a vacuous pass).
- rpc082/rpc083 read `dispatch.rs` (whole macro), `agent_loop/provider_openai.rs`
  (`run_openai_turn`) and `agent_loop/provider_custom.rs` (`run_custom_turn`); both files shrink
  under 300 lines (AGENTS.md) by using the shared reader and a compact signature witness.
- The two features name the real files; the OpenAI step now quotes the real call
  (`thinking_config_value`, not `.clone()`).

## Alternatives rejected

- Behavioural tests through a live turn: only the stub path can run offline; the macro-driven and
  OpenAI arms need real providers, so a structural check of their source is the available evidence.
- Moving the code back into agent_loop.rs: reverts a deliberate decomposition.

## Wire / API changes

None.
