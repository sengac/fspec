# AST Research — RLCD-002 Decision() first-class rig tool

Date: 2026-09-24 (specifying phase)
Method: AstGrep structural search (rust) + targeted reads.

## Findings

### 1. `impl Tool for $NAME` — every rig tool impl in codelet-tools

22 impls in `rust/tools/src`. The pattern RLCD-002 mirrors is
`impl Tool for DeepSearchTool` at `rust/tools/src/deep_search/mod.rs:397`:

```rust
impl Tool for DeepSearchTool {
    const NAME: &'static str = "DeepSearch";
    type Error = ToolError;
    type Args = DeepSearchArgs;
    type Output = String;
    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition { /* hand-written json! schema */ }
    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-013/017: pre_tool_hook_check(session_id, &self.name(), args json)
        //   -> Err => ToolError::Blocked
        // arg validation -> ToolError::Validation{tool, message} (message via
        //   codelet_common::tool_usage::append_usage_to_message for empty fields)
        // dispatch -> ToolError::Execution on failure
    }
}
```

Full inventory (file:line): done.rs:271, request_user_input.rs:318,
facade/wrapper.rs:77/230/417/1116/1244/1383/1600/1799/2016, fspec.rs:66,
web_search.rs:364, inject_summary.rs:126, deep_search/mod.rs:397,
schedule/mod.rs:88, agent_manager/mod.rs:71, generate_compaction/mod.rs:157,
unified_exec/tool.rs:78, session_search/mod.rs:66, bridge.rs:241,
graph_search/mod.rs:43.

### 2. `ToolError` — `rust/tools/src/error.rs:15`

thiserror enum: Timeout / Execution / File / Validation / Pattern / NotFound /
StringNotFound / Language / TokenLimit / Blocked. RLCD-002 uses
`Blocked` (pre-tool hook), `Validation` (arg validation), `Execution`
(classify errors + fail-open unreachable).

### 3. `create_rig_agent` registration sites — `rust/providers/src`

8 impls; 7 real providers (stub excluded):

| Site | create_rig_agent | Current tool-chain tail |
| --- | --- | --- |
| claude.rs | 506 | … `.tool(DeepSearchTool::new(session_id))` (553) … `.tool(ScheduleTool::new(session_id))` (557) |
| openai.rs | 424 | DeepSearch/Schedule anchors ~465/469 |
| gemini.rs | 130 | anchors ~211/215 |
| codex/mod.rs | 331 | anchors ~423/427 |
| zai.rs | 218 | anchors ~281/285 |
| custom/custom_provider.rs | 119 | TWO builder variants, anchors ~305/309 + 330/334 |
| copilot/rig_agent.rs | 56 | anchors ~102/106 |

All seven sites import the tools via `use codelet_tools::{ ...,
DeepSearchTool, ..., ScheduleTool, ... }` and chain `.tool(X::new(session_id))`
on the rig agent builder. RLCD-002 adds `DecisionTool::new(session_id)` at each
site (alphabetical import position: after `ConnectMcpTool`, before
`DeepSearchTool`).

### 4. DeepSearch sub-agent list (must stay unchanged)

`rust/tools/src/deep_search/mod.rs`: `SUB_AGENT_TOOL_NAMES: [&str; 7]`
(Read, Grep, AstGrep, Glob, Ls, Bash, SessionSearch) + `SUB_AGENT_TOOL_COUNT`
compile-time constant. RLCD-002's integration scenario asserts
`SUB_AGENT_TOOL_COUNT == 7` and that "Decision" is not in the list.

### 5. Precedent: handler/registry pattern (DeepSearch) vs direct tool

DeepSearch delegates execution to a per-session handler registry
(`set_deep_search_handler`). RLCD-002 does NOT need that: classify is a plain
HTTP call against the shared supervisor — the tool executes directly
(schedule/mod.rs is the direct-execution precedent: `impl Tool for ScheduleTool`
at schedule/mod.rs:88 calls its own handler inline).

### 6. RLCD-001 module already landed (test surface)

`rust/tools/src/rlcd/` (mod/error/config/service/spawn/pidfile):
`load_rlcd_config()`, `RlcdSupervisor::new(cfg)` / `ensure_ready(budget)` /
`model()` / `reachable()` / `unreachable_reason()` / `effective_base_url()`,
`RlcdConfig`/`RlcdSpawnConfig`, `RlcdError` (thiserror: Unreachable/Health/
NoFreePort/BinaryNotFound/Io). RLCD-002 adds `client.rs` (RlcdClient trait +
HttpRlcdClient + RlcdResponse + new RlcdError variants Validation/Busy/Server)
and `decision.rs` (DecisionTool/DecisionArgs).

## Decisions informed

- DecisionArgs.questions stays `serde_json::Value` (map qid -> typed spec
  object with optional `type`) rather than `BTreeMap` — preserves insertion
  order for tie-breaking (JEV-003: insertion order assigns labels/breaks ties)
  and matches the protocol's object shape exactly.
- Integration scenario proven by source-scan test (providers crate depends on
  tools; a tools test binary cannot link `codelet_providers`).
- Files kept <300 lines: decision.rs ~280, client.rs ~180.
