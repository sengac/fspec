# TOOL-026: validate attribution, update focus and delegate safely

Research date: 2026-10-22. Depends on SESS-003 and TOOL-025.
Overview: `docs/research/session-work-unit-focus.md`.

## Execution seam and coverage

`rust/patches/rig-core/src/tool/mod.rs:407` ToolType::call delegates both tool kinds.
`tool/server.rs:130` calls ToolSet::call. The typed ToolDyn blanket implementation
deserializes before Tool::call at `tool/mod.rs:183`; manually implemented ToolDyn,
including RMCP at `:272`, would bypass a change only in that blanket implementation.
Use shared, injected per-agent policy before either form of delegation.

Existing lifecycle pre_tool_use hooks are not universal enforcement: tools invoke
them individually after parsing, and handler registration at
`rust/sessions/src/session_creation_helper.rs:248` depends on configured hooks.
`rust/providers/src/custom/rig_tool.rs:99` remaps Rhai inputs; extract metadata before
mapping. `rust/tools/src/mcp.rs:1024` forwards raw args to the remote server; strip only
the harness-owned root `work_unit_id` before that boundary, retaining all nested
business arguments and `target_work_unit_id`. Existing business root `work_unit_id`
schemas are reserved-name collisions and must be rejected, never overwritten.
No wrapper compatibility or fallback is supported.

`rust/patches/rig-core/src/agent/prompt_request/streaming.rs:676` currently awaits each
local tool inline. No application multi_tool_use.parallel implementation was found in
the searched paths. Still define batch/parallel acceptance order and validate every
executable leaf if such wrappers are supported; outer metadata must not mask omissions.
Audit nonstreaming, rebuilt builders and direct provider dispatch as well as streaming.

## Proposed admission sequence, pending policy decisions

1. Identify the invoking session from trusted construction context, not an LLM-supplied
   session_id, AgentManager target, arbitrary project_root or process-global active ID.
2. Parse and validate the required top-level `work_unit_id` string. Missing/null/empty/
   unknown/cross-project IDs fail without state mutation or tool side effects.
   NONE is explicit clear.
3. Produce original audit arguments plus stripped business arguments. Preserve original
   tool-call/result pairing and correlation IDs; do not mutate persisted provider
   history into a shape inconsistent with the schema the model actually received.
4. Admit calls in deterministic invocation order and update shared session focus with
   a revision. Proposed timing is before delegation, retaining focus if execution
   later fails. Delayed completions must not restore older focus.
5. Run existing permission checks and execution on the stripped arguments. Define
   denied-call timing and do not bypass stage/worktree restrictions through NONE.
6. Reconcile successful status-command completion with BUG-231 using card/revision
   guards and approved attribution-versus-business-target precedence.

## Security and lifecycle hazards

- `rust/sessions/src/session_tool_callbacks.rs:127` treats no backend work-unit context
  as no stage gate. Explicit NONE or a different card must not grant unintended writes.
  Prefer separate activity focus and permission binding; this needs a product decision.
- `rust/sessions/src/background_session.rs:864` uses inner.try_lock for reminders;
  the agent holds inner at `rust/agent-loop/src/agent_loop.rs:580`. Queue reminder
  reconciliation instead of waiting on a lock held by the invoking agent.
- `rust/agent-loop/src/agent_manager_handler.rs:150` creates managed child sessions;
  each child needs its own middleware. A parent message targeting a child updates
  parent attribution, not the child's focus.
- DeepSearch and compaction use fresh IDs and independent tool builders at
  `deep_search_handler.rs:132` and `compaction_builder.rs:66`. Unknown-session lookup
  must not become an allow-all metadata bypass. Define isolated non-UI/owner policy.
- Codex raw/hosted-native routes bypass the standard JSON contract (see TOOL-025
  attachment). Strict mode needs adapters or explicit disabling, not inferred NONE.
- done() signals task/turn completion, not necessarily a work-unit status transition.
  Ensure its special completion handling cannot skip required admission checks.

## Regression plan

- Valid A -> B and A -> NONE updates caller only before the executor sees the call.
- Invalid/missing/null/cross-project metadata makes no focus change and invokes no
  shell, filesystem, MCP, callback, completion or provider-native side effect.
- Strict MCP server sees exactly its original business payload, including nested
  targets or `target_work_unit_id`; existing business root `work_unit_id` collisions
  reject registration. Facade and Rhai mappings behave equivalently.
- Two sessions, managed child targets and auxiliary-agent ownership stay isolated.
- Slow/failing calls, interleaved cards, duplicate results and clear/snapshot races
  obey deterministic admission order. No result-time last-writer-wins behavior.
- NONE cannot weaken write-stage or worktree constraints. Denied calls and attempts
  to select a permissive unrelated card need explicit security tests.
- Startup/mid-turn MCP registration, provider/model switches, done(), native/raw
  routes and direct ToolDyn all have enforcement tests, not just schema snapshots.

Use real session stores and shared temp-dir helpers with a deterministic executor at
the external boundary. Add narrowly scoped Rust integration targets after Gherkin;
do not claim schema-only tests demonstrate runtime enforcement.
