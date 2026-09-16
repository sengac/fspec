# TOOL-024 AST Research: Post-deserialization argument error sites

## Error type
`codelet_tools::ToolError` (tools/src/error.rs): every variant carries `tool: &'static str`.
`Validation` and `Execution` both display as `{message}` (no tool name in Display).
The blanket `ToolDyn` impl wraps `call()` errors as `ToolError::ToolCallError(Box<dyn Error>)`,
so the LLM-visible text is the inner error's `Display` — i.e. the message string only.

## Source sites (verified line numbers, 2026-09-16)

### 1. Bash (tools/src/bash.rs)
- `NAME = "Bash"` (line 154)
- `call_with_streaming` line 112–117: `ToolError::Validation { tool: "bash", msg: "command parameter is required" }` — lowercase tool, no usage
- `call` line 183–188: same error
- Schema: `schemars::schema_for!(BashArgs)` in `definition()` (line 165)

### 2. Grep (tools/src/grep.rs)
- `NAME = "Grep"` (line 395)
- `execute()` line 178–181: returns `ToolOutput::error("Error: pattern parameter is required")`
- `call()` line 462–478: maps `result.is_error` to `ToolError::Execution { tool: "grep", msg: result.content }` — mislabeled Execution, lowercase tool
- Schema: `schemars::schema_for!(GrepArgs)` in `definition()` (line 408)
- Other ToolOutput::error in execute(): "Error: Invalid regex pattern" (line 85) — execution error, not arg validation

### 3. AstGrep (tools/src/astgrep.rs)
- `NAME = "AstGrep"` (line 410)
- `execute()` line 177–183: `ToolOutput::error("Error: pattern parameter is required")`
- `execute()` line 186–193: `ToolOutput::error("Error: language parameter is required")`
- `call()` line 483–499: maps `result.is_error` to `ToolError::Execution { tool: "astgrep", msg }` — lowercase tool
- Schema: `schemars::schema_for!(AstGrepArgs)` in `definition()` (line 437)
- Other ToolOutput::error: "Error: Unsupported language" (line 200), "Error: Invalid AST pattern" (line 213) — execution errors

### 4. AstGrepRefactor (tools/src/astgrep_refactor.rs)
- `NAME = "AstGrepRefactor"` (line 1013)
- `execute()` line 146–153: `ToolOutput::error("Error: pattern parameter is required")`
- `execute()` line 155–162: `ToolOutput::error("Error: language parameter is required")`
- `execute()` line 164–171: `ToolOutput::error("Error: source_file parameter is required")`
- `call()` line 1070+: maps `result.is_error` to `ToolError::Execution { tool: "astgrep_refactor", msg }` — lowercase tool
- Schema: `schemars::schema_for!(AstGrepRefactorArgs)` in `definition()` (line 1019)

### 5. DeepSearch (tools/src/deep_search/mod.rs)
- `NAME = "DeepSearch"` (line 398)
- `call()` line 459–463: `ToolError::Execution { tool: "DeepSearch", msg: "query is required and must not be empty" }` — mislabeled Execution
- Schema: hand-written JSON in `definition()` (lines 417–441)
- Handler-dispatch error at line 480: `ToolError::Execution` — correct (execution failure, not arg validation)
- `deep_search/tests.rs:537` asserts `ToolError::Execution { message, .. }` for "handler not configured" — must NOT change that path

### 6. RequestUserInput (tools/src/request_user_input.rs)
- `NAME = "request_user_input"` (line 262) — snake_case registered name
- `definition()` name: `"request_user_input"` (line 270)
- `validate_questions()` line 111–114: returns `Err("questions array must not be empty")`
- `execute_hitl()` line 207–209: calls `validate_questions` first, then dispatches to handler
- `call()` line 351–355: maps all errors to `ToolError::Execution { tool: "request_user_input", msg: e }` — validation errors mislabeled as Execution
- Validation error messages all start with "questions array" or "question["
- Execution error messages: "Failed to acquire HITL handlers lock", "request_user_input is unavailable..."
- Schema: hand-written JSON in `definition()` (lines 279–331)
- Existing test (line 777): `tool.call(RequestUserInputArgs { questions: vec![] }).await` → asserts `err_msg.contains("must not be empty")`
- Existing test (line 955): `assert_eq!(definition.name, "request_user_input")` — registered name assertion

### 7. Schedule (tools/src/schedule/mod.rs)
- `NAME = "Schedule"` (line 74)
- `call()` line 167–173: unconditionally serializes `ScheduleResult` to JSON (success OR error)
- Handler errors (from agent-loop/src/schedule_handler.rs):
  - "Schedule name is required" — arg validation
  - "Cron expression is required" — arg validation
  - "Timezone is required" — arg validation
  - "Job type is required" — arg validation
  - "Agent jobs require role and prompt fields" — arg validation
  - "Shell jobs require a command field" — arg validation
  - "Invalid job_type: ..." — arg validation
  - "Invalid cron expression: ..." — arg validation
  - "Invalid timezone: ..." — arg validation
  - "Schedule already exists: ..." — execution error
  - "Schedule not found: ..." — execution error
- Schema: hand-written JSON in `definition()` (lines 93–138)
- `ScheduleResult` type (schedule/types.rs): `success: bool`, `error: Option<String>`, etc.
- napi/tests/schedule_tool_test.rs: tests the handler directly (not ScheduleTool::call), asserts on `result.success` and `result.error` — must not break

### 8. Done (tools/src/done.rs)
- `DONE_TOOL_NAME = "done"` (line 40) — lowercase registered name
- `NAME = DONE_TOOL_NAME` (line 267)
- `definition()` name: `DONE_TOOL_NAME.to_string()` (line 290)
- `call()` line 317–322: `ToolError::Validation { tool: "done", msg: "done() requires a non-empty summary..." }` — lowercase tool
- `call()` line 336–343: Tier 1 rejection — `ToolError::Validation { tool: "done", ... }`
- `call()` line 349–353: Tier 2 rejection — `ToolError::Validation { tool: "done", ... }`
- Schema: hand-written JSON in `definition()` (lines 292–311)
- No test anywhere asserts `tool: "done"` on a Validation payload (verified: zero matches)
- `cli/tests/cont006_goal_immediate_termination.rs:254`: asserts `tc.function.name == "done"` — this is a synthetic test message, NOT a runtime tool-name lookup. Safe to change DONE_TOOL_NAME.

## tool_usage.rs renderer artifacts

### Artifact A: oneOf with non-object variants renders as empty objects
- `render_parameter_reference` (tool_usage.rs:114–122): calls `synthesize_object(variant, true)` for each oneOf variant
- `synthesize_object` (line 163–183): only iterates a `properties` key → bare-type variants yield `{}`
- AgentManager session_id schema: `{"oneOf": [{"type":"string"}, {"type":"array","items":{"type":"string"}}]}`
- Current output: `session_id (object) — one of:\n    * {}\n    * {}`
- Fix: when a variant has no `properties`, render its `type` label (e.g. "string", "array of string")

### Artifact B: anyOf/oneOf-as-alternatives renders as bare "(any)"
- `prop_type` (line 139–161): returns `"any"` when neither `type` (string) nor `oneOf` is present
- For `anyOf` properties: not handled at all → falls through to "any"
- Fix: handle `anyOf` like `oneOf` — list each alternative's shape

### Artifact C: Done names itself lowercase
- `DONE_TOOL_NAME = "done"` → change to `"Done"`
- All three Validation payloads in done.rs use `tool: "done"` → change to `tool: Self::NAME`

## Compatibility constraints (must not regress)

1. `tools/src/facade/wrapper.rs:2518/2541/2560/2578/2596/2614/2635/2663`:
   asserts `ToolError::Validation { tool: "read"|"write"|"edit"|"grep"|"glob"|"ls", .. }` — these are
   the FACADE path (path isolation via `validate_and_resolve_path`), NOT the direct-tool empty-value
   path. Must keep passing.

2. `tools/src/deep_search/tests.rs:537`:
   asserts `ToolError::Execution { message, .. }` containing "handler not configured" — that's the
   no-handler dispatch path (line 480), distinct from the empty-query check (line 459). Must not change.

3. All TOOL-023 assertions remain valid: enrichment is append-only
   (`append_usage_to_message` preserves the original message as a verbatim prefix).

4. `napi/tests/schedule_tool_test.rs`: tests the schedule handler directly (not ScheduleTool::call),
   asserts on `result.success` and `result.error` strings. Must keep passing.

5. `request_user_input.rs:955`: `assert_eq!(definition.name, "request_user_input")` — registered
   name assertion. Keep registered name as "request_user_input" (changing it would break the
   LLM-facing tool contract and the Codex facade which uses "request_user_input" as the tool name).

## Implementation plan (source-site enrichment)

Use `codelet_common::tool_usage::append_usage_to_message(Self::NAME, &schema, &msg)` at each site.
Schema obtained via `self.definition(String::new()).await` (async, but call() is already async).

| Tool | Site | Change |
|------|------|--------|
| Bash | call_with_streaming:112, call:183 | Enrich Validation msg; tool → Self::NAME |
| Grep | call:470 | If result.is_error && msg contains "parameter is required" → Validation + enrich; else Execution |
| AstGrep | call:491 | Same as Grep |
| AstGrepRefactor | call:~1095 | Same pattern |
| DeepSearch | call:459 | Execution → Validation; enrich msg |
| RequestUserInput | call:351 | If msg starts with "questions array"/"question[" → Validation + enrich; else Execution |
| Schedule | call:167 | If !success && is_arg_validation_error → Validation + enrich; if !success → Execution; else Ok(json) |
| Done | call:317,336,349 | Enrich Validation msgs; DONE_TOOL_NAME → "Done"; tool → Self::NAME |
