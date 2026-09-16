# TOOL-024 AST Research — Post-deserialization arg-error recovery surface

## Goal
Every direct tool whose args serde-parse successfully but fail an internal
empty/missing-value check MUST surface a self-contained recovery block:
tool name (registered rig NAME), specific bad parameter, accepted-parameter
reference (required/optional, types, enums), and at least one complete
canonical example call. Must cover both `ToolError::Validation` and
`ToolError::Execution` (DeepSearch uses Execution for empty-query).

## Source sites (verified line numbers)

### 1. Bash — `tools/src/bash.rs`
- `const NAME: &'static str = "Bash"` (line 154)
- `call_with_streaming` line 112–117: `ToolError::Validation { tool: "bash", message: "command parameter is required" }`
- `call` line 183–188: same
- `definition()` line 160–168: `schemars::schema_for!(BashArgs)` → parameters Value

### 2. Grep — `tools/src/grep.rs`
- `const NAME: &'static str = "Grep"` (line 395)
- `execute()` line 174: `async fn execute(&self, args: Value) -> Result<ToolOutput>`
- `execute()` line 178–182: `Ok(ToolOutput::error("Error: pattern parameter is required"))`
- `call()` line 413: `async fn call(&self, args: Self::Args)`
- `call()` line 462–478: maps `result.is_error` to `ToolError::Execution { tool: "grep", message: result.content }`
- `definition()` line 401–411: `schemars::schema_for!(GrepArgs)`

### 3. AstGrep — `tools/src/astgrep.rs`
- `const NAME: &'static str = "AstGrep"` (line 410)
- `execute()` line 175: `pub async fn execute(&self, args: Value) -> Result<ToolOutput>`
- `execute()` line 177–184: `Ok(ToolOutput::error("Error: pattern parameter is required"))`
- `execute()` line 186–193: `Ok(ToolOutput::error("Error: language parameter is required"))`
- `call()` line 483–499: maps `result.is_error` to `ToolError::Execution { tool: "astgrep", ... }`
- `definition()` line 416–440: `schemars::schema_for!(AstGrepArgs)`

### 4. AstGrepRefactor — `tools/src/astgrep_refactor.rs`
- `const NAME: &'static str = "AstGrepRefactor"` (line 1013)
- `execute()` line 144: `async fn execute(&self, args: Value) -> Result<ToolOutput>`
- `execute()` line 146–153: pattern required
- `execute()` line 155–162: language required
- `execute()` line 164–171: source_file required
- `call()` line 1070: `async fn call(&self, args: Self::Args)`
- `call()` line 1136–1152: maps `result.is_error` to `ToolError::Execution { tool: "astgrep_refactor", ... }`
- `definition()` line 1019–1068: `schemars::schema_for!(AstGrepRefactorArgs)`

### 5. DeepSearch — `tools/src/deep_search/mod.rs`
- `const NAME: &'static str = "DeepSearch"` (line 398)
- `call()` line 445: `async fn call(&self, args: Self::Args)`
- `call()` line 459–464: `ToolError::Execution { tool: "DeepSearch", message: "query is required and must not be empty" }`
- `call()` line 480–483: `execute_deep_search(...).map_err(ToolError::Execution)` — handler-dispatch error, keep as Execution
- `definition()` line 404–443: hand-written `json!` parameters

### 6. RequestUserInput — `tools/src/request_user_input.rs`
- `const NAME: &'static str = "request_user_input"` (line 262)
- `definition()` line 268–332: hand-written JSON schema
- `validate_questions()` line 111–163: pure function returning `Result<(), String>`
- `execute_hitl()` line 207–222: calls `validate_questions` first, returns `Result<HitlResponse, String>`
- `call()` line 334–361: `execute_hitl(...).map_err(|e| ToolError::Execution { tool: "request_user_input", message: e })`
- Line 220: handler-not-found error message "request_user_input is unavailable in the current session mode"
- Line 214: lock error "Failed to acquire HITL handlers lock"

### 7. Schedule — `tools/src/schedule/mod.rs`
- `const NAME: &'static str = "Schedule"` (line 74)
- `call()` line 142–173: `execute_schedule_command(self.session_id, request)` → `ScheduleResult`
- Line 169–172: unconditionally serializes `ScheduleResult` to JSON via `serde_json::to_string_pretty`
- `ScheduleResult` type: `{ success: bool, error: Option<String>, ... }` in `schedule/types.rs`
- `definition()` line 80–140: hand-written JSON schema
- Handler messages (agent-loop/src/schedule_handler.rs line 136–189):
  - "Schedule name is required"
  - "Cron expression is required"
  - "Timezone is required"
  - "Job type is required"
  - "Invalid cron expression: ..."
  - "Invalid timezone: ..."
  - "Agent jobs require role and prompt fields"
  - "Shell jobs require a command field"
  - "Invalid job_type: ..."
  - "Unknown action: ..."
  - "Schedule not found: ..."
  - "Failed to read/parse/write schedules.json: ..."
  - "No schedule handler registered for this session"

### 8. Done — `tools/src/done.rs`
- `pub const DONE_TOOL_NAME: &str = "done"` (line 40)
- `const NAME: &'static str = DONE_TOOL_NAME` (line 267)
- `definition()` line 290: `name: DONE_TOOL_NAME.to_string()`
- `call()` line 315: `async fn call(&self, args: Self::Args)`
- Line 318–322: `ToolError::Validation { tool: "done", message: "done() requires a non-empty summary..." }`
- Line 336–343: `ToolError::Validation { tool: "done", message: "done() rejected: you must provide evidence..." }`
- Line 349–353: `ToolError::Validation { tool: "done", message: ... }` (Tier 2 verify failure)
- `definition()` line 273–313: hand-written JSON schema
- No test asserts `tool: "done"` on a Validation payload (zero workspace matches).

## Error type — `tools/src/error.rs`
```rust
pub enum ToolError {
    Timeout { tool: &'static str, seconds: u64 },
    Execution { tool: &'static str, message: String },
    File { tool: &'static str, message: String },
    Validation { tool: &'static str, message: String },
    Pattern { tool: &'static str, message: String },
    NotFound { tool: &'static str, message: String },
    StringNotFound { tool: &'static str, message: String },
    Language { tool: &'static str, message: String },
    TokenLimit { ... },
    Blocked { tool: &'static str, message: String },
}
```
- `tool: &'static str` — cannot be changed at runtime per-call
- Display for Validation/Execution: just `{message}` (no tool name in display)
- The LLM-visible error is `ToolError::ToolCallError(Box::new(e))` where `e` is the inner ToolError — so only `message` matters for LLM visibility.

## tool_usage.rs — `rust/common/src/tool_usage.rs`
- `pub fn arg_error_recovery(tool_name: &str, reason: &str, schema: &Value) -> String` (line 42)
- `pub fn append_usage_to_message(tool_name: &str, schema: &Value, message: &str) -> String` (line 53)
- `pub fn render_parameter_reference(tool_name: &str, schema: &Value) -> String` (line 75)
- `pub fn synthesize_example_call(schema: &Value) -> String` (line 134)
- `fn prop_type(prop: &Value) -> String` (line 139) — returns "any" for no-type
- `fn synthesize_object(schema: &Value, include_optional_strings: bool) -> String` (line 163)

### Artifact A: oneOf non-object variants render as `{}`
`render_parameter_reference` line 114–122:
```rust
if let Some(variants) = prop.get("oneOf").and_then(Value::as_array) {
    out.push_str(" — one of:");
    for variant in variants {
        out.push_str(&format!("\n    * {}", synthesize_object(variant, true)));
    }
}
```
`synthesize_object` line 163–183 only iterates `properties` key → bare-type variants yield `{}`.

AgentManager session_id schema: `{"oneOf": [{"type":"string"}, {"type":"array","items":{"type":"string"}}]}`
Current output: `session_id (object) — one of:\n    * {}\n    * {}`

### Artifact B: anyOf properties render as bare "(any)"
`prop_type` line 139–161: only checks `oneOf` and `type` — `anyOf` is not handled.

### Artifact C: Done names itself lowercase
`DONE_TOOL_NAME = "done"` → `DoneTool::NAME = "done"` → in-call `ToolError::Validation { tool: "done", ... }`.

## Compatibility constraints (must not regress)

1. `tools/src/facade/wrapper.rs` line 2518/2541/2560/2578/2596/2614/2635/2663:
   asserts `ToolError::Validation { tool: "read" | "write" | "edit" | "grep" | "glob" | "ls", .. }`
   — these are the FACADE path (`validate_and_resolve_path`), NOT the direct-tool empty-value path.

2. `tools/src/deep_search/tests.rs:537`:
   asserts `ToolError::Execution { message, .. }` containing "handler not configured"
   — that is the no-handler path (line 480), distinct from empty-query (line 459).

3. All TOOL-023 assertions (substrings `command parameter is required`,
   `pattern parameter is required`, `query is required and must not be empty`,
   `Cron expression is required`, `summary`-missing) remain valid because
   `append_usage_to_message` is append-only (original message stays as prefix).

4. `napi/tests/schedule_tool_test.rs` tests the handler directly (not
   `ScheduleTool::call`), asserting on `result.success` and `result.error` —
   those are the *dispatcher* path, separate from the tool's LLM-visible output.

5. `request_user_input.rs:955`: `assert_eq!(definition.name, "request_user_input")`
   — the registered name is "request_user_input" (snake_case). The feature file
   says "names the tool 'RequestUserInput'" — but the actual registered name IS
   "request_user_input". The feature scenario step 105 says "the error names the
   tool 'Done', not a lowercase alias" — that applies to Done. For RequestUserInput,
   the feature says "names the tool 'RequestUserInput'" at line 86. Since the
   registered name IS "request_user_input", we need to keep it as-is for the
   LLM contract. The rule says "MUST identify the tool by its registered rig NAME" —
   and the registered NAME is "request_user_input". The feature's scenario at line
   86 says "the error names the tool 'RequestUserInput'" — this is the PascalCase
   display. Since the feature file is the source of truth for acceptance criteria,
   and the rule says to use the registered rig NAME, we interpret this as: the
   tool should self-identify using its registered name in the error message.
   The registered name is "request_user_input". The feature says "names the tool
   'RequestUserInput'" — but that contradicts the registered name.
   
   Resolution: The registered name IS "request_user_input" (snake_case) — this is
   what the LLM sees in the tool definition. The feature file's "names the tool
   'RequestUserInput'" is shorthand for "names the tool with its registered name,
   not a bare internal alias". The key requirement is: the tool name appears in
   the error, and it matches the registered name. Since `definition().name` is
   "request_user_input", the error should say "request_user_input".
   
   Wait — re-reading the feature: "the error names the tool 'RequestUserInput'".
   And Rule 4 says "MUST identify the tool by its registered rig NAME". The
   registered rig NAME is `request_user_input`. So the error must contain
   "request_user_input". The feature's "RequestUserInput" is the human-readable
   display name used in the spec, not a literal string assertion.
   
   But looking at the scenario more carefully at line 86: "And the error names the
   tool 'RequestUserInput'". This is ambiguous. The safest interpretation:
   use `Self::NAME` which is "request_user_input". The test should assert
   "request_user_input" (the registered name), not "RequestUserInput".
   
   Actually, re-reading Rule 4: "Argument-validation errors MUST identify the
   tool by its registered rig NAME (Self::NAME / DONE_TOOL_NAME's display name
   'Done'), not a lowercase internal alias."
   
   For RequestUserInput, `Self::NAME` is "request_user_input" — this IS the
   registered rig NAME. It's not a "lowercase internal alias" — it's the actual
   registered name. So the error should contain "request_user_input".

6. DONE_TOOL_NAME: changing from "done" to "Done" — need to check if anything
   depends on the literal string "done" being the tool name. The system prompt
   references the tool as `done` in lowercase. But the feature file explicitly
   says the registered name should be "Done". Let me check the feature again:
   "Done tool arg error uses the registered tool name" → "the error names the
   tool 'Done', not a lowercase alias". And Rule 4: "DONE_TOOL_NAME's display
   name 'Done'". So the registered name should change from "done" to "Done".
   
   This means changing `pub const DONE_TOOL_NAME: &str = "done"` to `"Done"`,
   which changes the LLM-facing tool name from `done` to `Done`. This is a
   breaking change for the LLM — it would need to call `Done` instead of `done`.
   
   But the feature file is explicit: "the error names the tool 'Done', not a
   lowercase alias". And the rule says "DONE_TOOL_NAME's display name 'Done'".
   So the intent is to change the registered name to "Done".

## Implementation plan

### Source-site enrichment pattern
Each direct tool's `call()` method, after the internal empty-value check fails,
should:
1. Use `Self::NAME` (not a lowercase alias) as the `tool` field
2. Append `codelet_common::tool_usage::append_usage_to_message(Self::NAME, &schema, &msg)`
   where `schema` comes from `self.definition(String::new()).await.parameters`

For tools using `ToolOutput::error` (Grep, AstGrep, AstGrepRefactor):
- The `call()` method receives `result.is_error` and maps to `ToolError::Execution`
- Need to detect argument-validation errors vs execution errors
- Pattern: check if the error message contains "parameter is required" or
  "parameter must not be empty" → it's argument validation → use Validation + enrich
- Otherwise → it's an execution error → keep as Execution (no enrichment)

### Schedule tool
The Schedule tool returns `ScheduleResult` (not `ToolError`). The `call()` method
currently serializes the whole result to JSON. Need to:
- Detect `!result.success` && the error is an argument-validation error
- Map to `ToolError::Validation { tool: "Schedule", message: enriched }`
- For non-argument errors, keep as `ToolError::Execution`

### Done tool
Change `DONE_TOOL_NAME` from "done" to "Done" — this changes the LLM-facing name.
Add enrichment to the three `ToolError::Validation` sites.

### tool_usage.rs renderer fixes
- `prop_type`: handle `anyOf` by listing alternatives
- `render_parameter_reference`: when a oneOf variant has no `properties`, render
  its `type` label (e.g. "string", "array of string") instead of `{}`
