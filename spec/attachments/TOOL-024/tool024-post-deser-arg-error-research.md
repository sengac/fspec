# TOOL-024 Research: Post-deserialization argument errors lack recovery guidance

**Date:** 2026-09-16
**Method:** Live non-destructive probing of every agent-exposed tool (error-only calls, no state changes) + source verification of each observed bare error + full TOOL-023 test-suite regression run.
**Trigger:** Live review of TOOL-023 (done) revealed that its universal recovery only covers the serde *deserialization* failure path (direct tools) and facade `map_params` failures. Errors raised *after* successful deserialization by internal empty/missing-value checks remain bare.

---

## 1. Verified working (no action needed)

TOOL-023's shipped surfaces, all confirmed live on 2026-09-16:

| Surface | Probe | Result |
|---|---|---|
| Fspec dispatcher — fuzzy match | `foundation-statues`, `list-workunit` | `Unknown fspec command: <name>` + `Did you mean: <name>` + re-call line + `--help` pointer |
| Fspec dispatcher — no close match | `zzzqqqxx` | No "Did you mean" line (correct negative case) |
| Fspec dispatcher — malformed inner args | `list-work-units` + args `{` | `Invalid args for fspec command list-work-units: failed to parse args: EOF while parsing an object at line 1 column 1` + `args must be a JSON object string — e.g. args: "{\"workUnitId\": \"AUTH-001\"}"` + `list-work-units --help` |
| Fspec dispatcher — missing inner field | `show-work-unit` without workUnitId | `missing required argument: workUnitId` + same usage-hint line |
| Fspec `--help` rendering | `list-work-units --help` | Full USAGE/OPTIONS/EXAMPLES/RELATED blocks |
| Direct tools — serde miss | `Read {}`, `Write {}`, `Edit {}`, `Done {}`, `inject_summary {}`, `AgentManager {action:get_status}` (no session_id) | `X tool: invalid arguments: missing field \`Y\` ...` + `Accepted parameters for X:` + `Example call: ...` |
| Direct tools — oneOf sub-field | `WebSearch {action:{}}` | names tool, `missing field \`type\``, lists all four variants, example call — the exact reported defect is fixed |
| Bridge invalid action | `bridge {action:{type:invalid_thing}}` | `Unknown action type: invalid_thing` + `Accepted parameters for bridge:` (all 3 variants) |
| ConnectMCP bad action | `ConnectMCP {action:{connect:true}}` | serde unknown-variant error + full parameter block (see §3 artifact B) |

Test suites (all green):

```
cargo test -p codelet-fspec-core --test tool023_arg_error_recovery   # 7 passed
cargo test -p codelet-tools --test tool023_universal_arg_errors     # 3 passed
cargo test -p codelet-tools --test tool023_fspec_facade_args        # 6 passed
```

## 2. Gap A — post-deserialization empty/missing-value errors are bare

These tools deserialize fine, then fail an internal check. The error the LLM sees contains the missing parameter but **no tool-name framing, no accepted-parameter reference, no example call**.

| Tool (rig NAME) | Probe | LLM-visible error (verbatim) | Source site | Error variant |
|---|---|---|---|---|
| Bash | `Bash {command: ""}` | `command parameter is required` | `tools/src/bash.rs:113` (call_with_streaming) and `:184` (call) | Validation { tool: "bash" } — also lowercase |
| Grep | `Grep {pattern: ""}` | `Error: pattern parameter is required` | `tools/src/grep.rs:178-181` (`execute()` returns `ToolOutput::error`); surfaced by `call()` at `grep.rs:462-474` | Execution { tool: "grep" } — mislabeled |
| AstGrep | `AstGrep {pattern: ""}` | `Error: pattern parameter is required` | `tools/src/astgrep.rs:181` (language check at :190) | Execution (via same `execute()` delegation shape) |
| AstGrepRefactor | `AstGrepRefactor {source_file: ""}` | `Error: source_file parameter is required` | `tools/src/astgrep_refactor.rs:150` (pattern :150, language :159, source_file :168) | Execution via delegation |
| DeepSearch | `DeepSearch {query: ""}` | `query is required and must not be empty` | `tools/src/deep_search/mod.rs:459-463` | **Execution** { tool: "DeepSearch" } — mislabeled (it's argument validation) |
| RequestUserInput | `RequestUserInput {questions: []}` | `questions array must not be empty` | `tools/src/request_user_input.rs:113` (`validate_questions`); `call()` at :334-355 | Execution { tool: "request_user_input" } via `execute_hitl` map_err at :352 |
| Schedule | `Schedule {action: add, name: "x"}` (no cron) | `"{\"success\": false, \"error\": \"Cron expression is required\"}"` — raw serialized JSON, not even a ToolError | `tools/src/schedule/mod.rs:167-173` (unconditionally serializes the handler's `ScheduleResult`); messages originate in `core/src/scheduler/crud.rs:96-99` and `agent-loop/src/schedule_handler.rs:136-140` | n/a — Ok(String) with error-shaped JSON |

### Why TOOL-023's choke point does not catch these

`patches/rig-core/src/tool/mod.rs:183-214` — the blanket `ToolDyn for T::call` only wraps the **serde failure** branch (line 199-211) with `codelet_common::tool_usage::arg_error_recovery`. The success branch (line 186-188) maps the tool's own `ToolError` verbatim into `ToolError::ToolCallError(Box::new(e))`. So any in-`call()` `ToolError::Validation`/`Execution` payload passes through un-enriched. Facade `map_params` errors were separately enriched via `enrich_facade_arg_error` in `tools/src/facade/wrapper.rs:45` — that does not touch direct tools' in-call checks.

### Constraint that blocks the "just wrap it centrally" fix

`ToolError` (tools/src/error.rs:14-62) stores `tool: &'static str` on every variant, so a per-call enrichment in the blanket impl cannot rewrite or append to an existing payload without changing the error type. `ToolError` is consumed across all providers, the napi crate, agent-loop, and many tests — changing its shape is invasive. **Recommended direction: enrich at the source sites** using the existing `codelet_common::tool_usage::append_usage_to_message(tool_name, schema, message)` helper (same module TOOL-023 introduced; its unit tests already lock the "original message stays a verbatim prefix" contract). The schema is available at each site via the same `definition()` the TOOL-023 fix used (schemars-generated for most tools; ScheduleTool's is hand-written JSON at schedule/mod.rs:93-138).

Note: the direct-tools `execute()`-delegation pattern (Grep/AstGrep/AstGrepRefactor) returns `ToolOutput` with `is_error: true` rather than `Err(ToolError)` — enrichment must happen where the `ToolOutput` error becomes the tool's returned error, keeping the `Error: ` prefix substring.

## 3. Gap B — `tool_usage.rs` schema-rendering artifacts (LLM-visible)

Observed live in the recovery blocks that DO render:

**A. oneOf with non-object variants renders as empty objects.**
Probe: `AgentManager {action: get_status}` (no session_id) →

```
  - session_id (object) — one of:
    * {}
    * {}
```

Cause: `render_parameter_reference` (common/src/tool_usage.rs:114-122) calls `synthesize_object(variant, true)` for each oneOf variant; `synthesize_object` (line 163-183) only iterates a `properties` key, so a variant that is a bare type (`{"type":"string"}` or `{"type":"array",...}`) yields `{}`. Fix: when a variant has no `properties`, render its `type` (e.g. `string`, `array of string`).

**B. anyOf properties render as bare "(any)" with no sub-values.**
Probe: `ConnectMCP` invalid action →

```
  - action (any), default: "connect"
  - transport (any)
```

Cause: `prop_type` (line 139-161) returns `"any"` when neither `type` nor `oneOf` is present — `anyOf`/`oneOf`-as-alternatives and nested object shapes are not rendered. Fix: handle `anyOf` (and object-typed variants) like `oneOf` — list each alternative's shape.

**C. `Done` names itself lowercase.**
Probe: `Done {}` → `done tool: invalid arguments: missing field \`summary\` ...`.
Cause: `tools/src/done.rs:40` `pub const DONE_TOOL_NAME: &str = "done";` and `const NAME: &'static str = DONE_TOOL_NAME` (line 267) — the *registered* name is literally `done`, and the in-call `ToolError::Validation { tool: "done", .. }` payloads (done.rs:318/336/349) match it. This is self-consistent (the LLM *does* see the tool registered as `done`), so it is a nomenclature decision rather than a bug: decide whether the registered name becomes `Done` (aligning with `Read`/`Write`/`Edit` casing and the system-prompt `done` tool description) or the lowercase name is kept and this item is dropped. **Verified safe:** no test anywhere asserts `tool: "done"` on a Validation payload (workspace grep: zero matches).

## 4. Compatibility notes (must not regress)

- `tools/src/facade/wrapper.rs:2518/2541/2560/2578/2596/2614/2635/2663` — TOOL-014 worktree tests assert `ToolError::Validation { tool: "read" | "write" | "edit" | "grep" | "glob" | "ls", .. }` (lowercase tool names on facade path). These are the FACADE path (path isolation), not the direct-tool empty-value path, but any central enrichment must keep them passing.
- `tools/src/deep_search/tests.rs:537` asserts `ToolError::Execution { message, .. }` containing "handler not configured" — that is the *no-handler* path, distinct from the empty-query path; converting the empty-query check to `Validation` must not touch the handler-dispatch mapping at deep_search/mod.rs:480.
- All TOOL-023 assertions (substrings `command parameter is required`, `pattern parameter is required`, `query is required and must not be empty`, `Cron expression is required`, `summary`-missing behavior) remain valid because enrichment is append-only (`append_usage_to_message` preserves the original message as a verbatim prefix).
- Schedule's `ScheduleResult::error` strings are also asserted in fspec-core (`add_schedule.rs:380`) and in agent-loop/napi handlers — those are the *dispatcher* path (separate surface) and must be preserved; only the Schedule *tool's* LLM-visible JSON blob changes shape.

## 5. Scope boundary

In scope: the seven probes in §2 (Bash, Grep, AstGrep, AstGrepRefactor, DeepSearch, RequestUserInput, Schedule), the Done tool-name decision, and the `tool_usage.rs` renderer fixes (A, B).
Out of scope: execution-failure errors (command exited non-zero, IO failures, timeout, blocked-by-blocklist) — those are not argument-validation failures and get no usage block.
