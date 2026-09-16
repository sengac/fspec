# Research: What Happens When a Tool Call Fails with Wrong or Missing Arguments

**Research date:** 2026-09-16
**Goal:** Inventory every distinct tool in the fspec agent system, document what the LLM actually receives when it calls a tool with wrong or missing arguments, and identify where the failure is *unrecoverable for the LLM* (i.e., the error message does not explain how to call the tool correctly).

---

## 1. Big picture: the error pipeline

Every LLM tool call flows through the same pipeline before the LLM sees a result:

```
LLM emits ToolCall(name, args_json)
        │
        ▼
patches/rig-core :: ToolSet::call(name, args)        # tool/server.rs + tool/mod.rs
   ├─ name lookup (case-insensitive)
   │    └─ miss → ToolSetError::ToolNotFoundError("Tool not found: {name}")
   │
   ▼
ToolDyn::call(args: String)                           # tool/mod.rs:183
   ├─ serde_json::from_str::<Args>(args)
   │    └─ parse fail → ToolError::JsonError("JSON error: {e}")   ← THE BLIND SPOT
   │
   ▼
<T> as Tool>::call(args: Args)                        # per-tool `call()`
   ├─ in-tool validation → ToolError::Validation { tool, message }
   └─ handler dispatch (session-scoped) → tool-specific error
        │
        ▼
ToolSetError → ToolServerError (channel hop) → e.to_string()
   # rig-core/agent/prompt_request/streaming.rs:620-627
   Err(e) => { tracing::debug!("Error while calling tool: {e}"); e.to_string() }
        │
        ▼
parse_tool_result_content(err_string)
   # rig-core/agent/prompt_request/streaming.rs:182
   → ToolResultContent::text(err_string)
        │
        ▼
LLM receives the plain-text error string as the tool result
```

**Key property:** tool *results* are never flagged `is_error=true` to the LLM. Errors are plain text in a `ToolResultContent::text` block. The only classification layer is the CLI's TUI (`cli/src/interactive/stream_handlers.rs::detect_tool_error`), which pattern-matches *string prefixes* to colour the card red — this is display-only and invisible to the LLM.

So the LLM's entire ability to recover from a bad call depends on the **quality of the text string** that ends up in `ToolResultContent::text`.

---

## 2. The failure categories

### 2.1 `serde_json::from_str::<Args>` failures (rig layer) — **THE BLIND SPOT**

`ToolDyn::call` (rig-core tool/mod.rs:183-195) deserializes the raw args string into each tool's `Args` type before the tool's `call()` runs. Three distinct sub-cases:

| Case | Example | LLM receives |
|---|---|---|
| Malformed JSON (unterminated string, extra comma) | `{"command": "board"` | `JSON error: expected `,` or `}` after object value at line 1 column 20` |
| Missing required field | `AgentManager{}` (needs `action`) | `JSON error: missing field `action` at line 1 column 2` |
| Wrong field name | `Grep(pattern=…)` sent as `{"pat": …}` | `JSON error: missing field `pattern` at line 1 column 1` (extra keys are silently ignored) |

**Problems:**

1. **Tool name is absent from every message.** `JSON error: missing field `action`` — which tool? The LLM has to infer from the fact that the immediately-preceding assistant turn called `AgentManager`. In a batched turn with several tool calls, or after compaction, this is a real cost.
2. **`missing field` does not distinguish *missing* from *misnamed* fields.** `Grep` with `{"pat": "x"}` reports `missing field 'pattern'` — it names the correct field, which is useful, but it does not say "you sent `pat`, which is not a recognized parameter; the accepted parameters are: pattern, path, glob, …". The LLM must re-consult the tool definition.
3. **serde's message carries no usage hint.** There is no embedded schema, no "did you mean", no pointer to `command:"<tool> --help"` for fspec.
4. **Some tools' Args are enums with `#[serde(tag = "action")]`** (AgentManager, SessionSearch, GraphSearch, WebSearch). A typo in the action value produces:
   `JSON error: unknown variant `spwan`, expected one of `spawn`, `list`, `get_status`, `close`, `message`, `set_role`, `await_idle`, `profile`` — this one is actually *good* (lists the valid variants), but again carries no tool name.
5. **`serde_coerce.rs` lenient deserializers silently swallow errors.** `deser_option_usize`, `deser_option_bool`, `deser_vec_usize` (Option fields) return `Ok(None)` for *any* unparseable value instead of failing. E.g. `AgentManager.await_idle{timeout: "abc"}` → `timeout: None` (waits forever) with **no error at all**. Same for `GraphSearch` numeric fields and `SessionSearch` numeric fields. This is worse than a bad error: it's a *silent behaviour change*. (The required-value versions `deser_usize`/`deser_bool`/`deser_vec_usize` do error, e.g. `cannot parse `"abc" as an unsigned integer`. )

### 2.2 Tool-name lookup failure

`ToolSet::call` (tool/mod.rs:475-485): `Err(ToolSetError::ToolNotFoundError(name))` → `Tool not found: {name}`.

- Case-insensitive, so casing typos are already handled (TOOL-021).
- The message names the tool the LLM tried. **No list of available tools** — the LLM must remember what was registered. Reasonable, but not ideal when the LLM hallucinates a plausible-looking name.
- For MCP tools: the qualified name is `mcp__<server>__<tool>`. `route_mcp_tool_call` (mcp.rs:506-563) has two distinct failures:
  - `Invalid MCP tool name: {qualified_name}` (malformed name)
  - `MCP server '{server}' is not connected` (stale reference) — this one *is* recoverable-ish: tells the LLM to `ConnectMCP` first.
  - `MCP tool error: {error_text}` — passthrough of the remote tool's error; quality depends entirely on the remote server.

### 2.3 In-tool validation (per-tool `ToolError::Validation`)

Each tool's `call()` re-validates its args. Quality varies a lot per tool:

**Good (names the tool, names the field, sometimes the valid set):**

| Tool | Message |
|---|---|
| `unified_exec` | `Unknown action: {action}. Valid: run, write, poll, list, close` |
| `unified_exec` | `command must be a string or array of strings` / `command array must not be empty` |
| `WebSearch` (Claude facade) | `Missing 'action_type' field` / `Unknown action type: {action_type}` |
| `Bridge` (Claude/OpenAI) | `Missing 'action' field` / `Missing 'action.type' field` / `Missing 'url' field for connect action` |
| `AstGrep` | `Error: Unsupported language '{lang}'. Supported languages include: typescript, javascript, rust, python, go, java, c, cpp, ruby, kotlin, swift, dart, solidity, nix, hcl, etc.` |
| `done` | `done() requires a non-empty summary describing what was completed` / `done() rejected: you must provide evidence and a goal_assessment for the active goal: {goal}` |
| `ConnectMCP` | `name is required for connect action` / `command is required for stdio transport` / `url is required for http transport` |
| `request_user_input` | `questions array must not be empty` / `question[2].id must not be empty` / `question[0].id "x Y" must be snake_case (lowercase letters, digits, underscores)` |

**Borderline (names the field, but not the tool, no valid-set, no example):**

| Tool | Message |
|---|---|
| Gemini file facades (`read_file`/`write_file`/`replace`) | `Missing 'file_path' field` / `Missing 'content' field` / `Missing 'old_string' field` |
| `Bash` | `command parameter is required` |
| fspec facades (Claude/OpenAI) | `Invalid arguments: {e}` — a **double-wrapped serde error** with no tool name and no command context |
| Gemini/ZAI fspec facades | `Missing 'command' field` |
| `param_extract::extract_required_string` | `Missing or empty required '{field}' field` |
| `Codex` exec facade | `Missing required parameter: command` / `command must be an array of strings` |

**Weak (opaque or misleading):**

| Tool | Message | Why it's weak |
|---|---|---|
| `Fspec` (Claude/OpenAI facade) | `Invalid arguments: {e}` where `{e}` is a raw serde message | The LLM sent e.g. `{"command": "board", "args": {…}}` (object instead of string) → `Invalid arguments: invalid type: map, expected a string at line 1 column 25`. No mention of the *Fspec* tool, no statement that `args` must be a **JSON string**, no example of the correct shape. The tool description says `args` defaults to `"{}"` but nothing about the string-vs-object gotcha, which is the #1 source of fspec tool failures. |
| `Fspec` (Gemini/ZAI facade) | `Missing 'command' field` | Missing `command` is the only thing detected; `args` and `root_dir` are silently defaulted, so a garbage `args` shape is only discovered one layer deeper (in the dispatcher, §2.4). |
| `AgentManager` | *(none at this layer)* | Args deserialization is the only gate; see §2.1. Note: for `message` action, `session_id` must be a **string** (the `Message` variant has `session_id: String`, not `SessionIdParam`) — sending an array for `message` gives the generic §2.1 serde error, while the same shape is *valid* for `await_idle`. This inconsistency is invisible in the error. |
| `DeepSearch` | `scope` accepts string/array/null via custom deserializer; unknown action-shape falls through to a `D::Error::custom` from serde | `max_depth`/`max_recursion_depth` sent as non-numeric strings → **silently None** (§2.1 item 5), no error. |
| `SessionSearch` / `GraphSearch` | same pattern as DeepSearch | flattened enum with `#[serde(tag = "action_type")]` — wrong tag → §2.1 serde error naming the tool only indirectly. |
| `AstGrepRefactor` | pattern language errors are wrapped as `ToolError::Execution { message: e.to_string() }` (line 1139-1142) | Loses the `Validation`/`Language` classification; an invalid AST pattern surfaces as a generic execution error. |

### 2.4 The Fspec tool's dispatcher layer (fspec-core)

When the facade passes through, the args hit `fspec-core::dispatch::dispatch_command`:

1. **`args` field type coercion happens at the facade layer**, not the dispatcher. For the Claude/OpenAI `Fspec` tool, `args` is a `String` (the tool description says `args` defaults to `"{}"`). If the LLM sends an **object**, the *facade* fails with `Invalid arguments: invalid type: map, expected a string` (§2.3). The dispatcher never sees it.
2. **Malformed JSON in the inner `args_json`** → every command's `run()` starts with:
   ```rust
   serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
       command: "list-work-units",
       reason: format!("failed to parse args: {e}"),
   })?;
   ```
   (This exact pattern appears in **162 command files**.) Display: `Invalid args for fspec command list-work-units: failed to parse args: expected value at line 1 column 0`.
   - **Good:** names the command.
   - **Bad:** no *usage* for the command. The LLM must call `command: "help"` or `"list-work-units --help"` — which the error does not suggest.
   - **Bad:** `failed to parse args:` is a *double* prefix over the already-wrapping `Invalid args for fspec command X:`.
   - **Bad:** a **missing required field** in the *inner* args (e.g. `show-work-unit` with `{}`) produces `missing field `workUnitId`` as a serde error — which is fine for the common case, but note the args struct for `show-event-storm` deliberately catches this and re-renders it: `failed to parse args: missing required field `workUnitId`` (show_event_storm.rs:61) — an inconsistent special case that suggests the generic path is known to be weak.
3. **Explicit missing-field errors** (hand-written per command) are generally good:
   `missing required argument: workUnitId`, `missing required 'query' argument`, `missing required field: workUnitId` — but phrasing is inconsistent across commands (`missing required argument:` vs `missing required 'x' argument` vs `missing required field `x``), and none include usage.
4. **Unknown command:** `Unknown fspec command: {name}` — no list of valid commands, no "did you mean" (e.g. `list-workunit` vs `list-work-units`). Contrast with §2.1's serde enum error, which *does* list valid variants — the LLM gets a *better* hint for a bad action-value than for a typo'd command name.
5. **`NotYetPorted`:** `Command {command} is not yet ported to Rust (tracked by {work_unit}). The standalone fspec binary cannot execute TypeScript fspec commands.` — excellent recovery guidance (the error *is* the explanation). **This is the model the other errors should follow.**
6. **`ParseJson` (corrupted state files):** `Failed to parse {file}: The file may be corrupted or contain invalid JSON.\n{caret-pointed snippet}` via the vendored `codelet_fspec_json_error` crate — **excellent** (RPC-334). Again, the model to follow.
7. **`DirectoryNotFound`:** `Directory not found: {path}` — good.
8. **Help dispatch (RPC-414):** `command == "help"`, `"<cmd> --help"`, `"<cmd> -h"` are recognized *before* canonical lookup and render the per-command help page (arguments, options, examples, prerequisites, **common_errors** section). This is the richest recovery surface in the system — but it only fires on *successful, intentional* help requests, never on failure.

### 2.5 Session-scoped handler errors (internal plumbing)

| Tool | Message | LLM-facing? |
|---|---|---|
| `Fspec` (handler absent) | `Fspec handler not configured for session {uuid} - FspecTool requires session context` | Yes — but it's an internal error, not a bad-args error. |
| `AgentManager` | `Agent manager handler not configured for session {session_id} — AgentManagerTool requires session context` (JSON: `{"error":true,"code":"internal_error","message":"…"}`) | Yes, as JSON. |
| `SessionSearch` | `Session search handler not configured for session {uuid} — SessionSearchTool requires session context` | Yes. |
| `GraphSearch` | `{"error":"No handler registered for this session. GraphSearch is not available."}` | Yes. |
| `Schedule` | `{"success":false,"error":"Schedule name is required"}` etc. (schedule_handler.rs:136-148) | Yes — **note these are *in-handler* validation messages, good quality.** |
| `DeepSearch` | `Failed to acquire deep search handlers lock` / handler-absent variants | Yes. |
| `inject_summary` | handler errors wrapped in `ToolError::Execution` | Yes. |

### 2.6 Blocklist / stage-permission / pre-tool-hook rejections

`ToolError::Blocked { tool, message }` — e.g. from `check_bash_command` ("Blocked: …") and `check_file_path`. These carry the blocklist rule's own reason. Quality is good and by design *not* about arguments.

### 2.7 Truncated tool calls (model-side)

`is_truncated_tool_call_error` (`cli/src/interactive/error_classifiers.rs:61`) matches the literal string `Tool call truncated due to output token limit`. `build_truncation_recovery_message` (`recovery_truncation.rs`) injects a structured recovery prompt: names the tool, states the cause, says *don't retry the same large call, chunk it instead*. **This is the only place in the system where a failure automatically gets a structured "how to proceed" recovery instruction — and it only covers truncation, not arg errors.** (PROV-039/PROV-040, feature: `truncated-tool-call-recovery-auto-chunk-large-writes-and-retry-on-max-tokens.feature`.)

### 2.8 Unrecoverable API errors + failed tool-call stripping

BUG-170 (`strip_failed_tool_call_tail`): after a prompt-too-long error, the failed tool-call *pair* is surgically removed from context so the next user message doesn't replay the oversized call. The tool-call error itself was already emitted; this is about *context hygiene*, not error quality.

### 2.9 TUI error detection (display only)

`detect_tool_error` (`stream_handlers.rs:30-69`) classifies results as errors for the red-card UI by:
1. JSON `{"success": false}` or `{"error": "…"}` — note the **fragility**: any *successful* tool output that happens to contain a JSON object with a truthy `error` field (or a `success: false` field that's part of *data*) is misclassified. Conversely, any error that isn't JSON and doesn't start with one of 8 hardcoded prefixes ("Command failed with exit code", "Operation timed out", "Token limit exceeded:", "Invalid pattern:", "Unsupported language:", "Tool not found:", "Toolset error:", "ToolCallError:", "ToolServerError:") is rendered as a *success* card.
2. This list is maintained by hand and **already out of sync with some error strings** (e.g. `ToolError::Validation` messages like "Missing 'file_path' field" are *not* in the prefix list, so a missing-arg failure renders as a green/success card in the TUI). The LLM is unaffected (it always gets the raw text), but humans reviewing the TUI can't tell the call failed.

### 2.10 The standalone CLI (`fspec <cmd>`) front door

`rust/fspec/src/main.rs::render_clap_error` re-renders clap errors in Commander.js byte-parity form: `error: missing required argument '<name>'`, `error: unknown option '<flag>'`, `error: unknown command '<name>'` — single-line, exit 1. Good for shell users; note these are a **separate** front door (shell), not the LLM tool path. The LLM path is §2.1–§2.4.

---

## 3. Tool-by-tool inventory

Legend: **S**chema quality (what the LLM is *told* up front), **E**rror quality (what it gets when wrong), **R**ecovery surface (help/retry affordances).

### Built-in agent tools (rig `Tool` impls in `codelet-tools`)

| Tool | S | E | Notes |
|---|---|---|---|
| `Read` | schemars-derived (`ReadArgs`) | Validation: "Error: file_path must be absolute"; "Error: File not found: {path}"; TokenLimit: "Token limit exceeded: {file} has ~{n} tokens (limit: {max})" + suggestions; image size/dim errors with suggestions | Good. Token-limit and image errors include *suggestions*. Path errors do not suggest "check with Glob/Ls". |
| `Write` | schemars | "Error: file_path must be absolute" | Good. |
| `Edit` | schemars | Validation "Error: file_path must be absolute"; StringNotFound variants | Good. `old_string` not found → error shows the search context. |
| `Bash` | schemars (`BashArgs`) | "command parameter is required"; blocklist `Blocked`; exit-code errors "Command failed with exit code N" + stderr | Good. |
| `Glob` | schemars | "Invalid pattern:" via ToolOutput | OK. |
| `Grep` | schemars | "Error: Invalid regex pattern: {e}" (regex crate message — can be terse, e.g. `regex parse error: ...`) | **Weak spot:** ripgrep/regex error text is not always LLM-friendly, and the message doesn't echo the offending pattern. |
| `Ls` | schemars | path errors | OK. |
| `AstGrep` | schemars | `ToolError::Language` "Unsupported language: {message}" (with supported list); multi-line pattern error with examples; execution errors | Good. |
| `AstGrepRefactor` | schemars | path validation; execution errors wrapped in `Execution` (loses Validation classification, §2.3) | **Weak:** error-classification loss. |
| `apply_patch` (Codex) | codex schema | "Patch must start with '*** Begin Patch'" / "Patch missing '*** End Patch' marker" / per-file hunk errors | Good. |
| `WebSearch` | provider facades (Claude nested / Gemini flat) | "Missing 'action_type' field" / "Unknown action type: {x}" / action-specific missing-field errors; action-normalization shim handles provider naming drift | Good. |
| `done` | hand-written schema | Excellent: names required field, goal-mode rejection explains *which* goal and *what* is missing | **Model behaviour.** |
| `request_user_input` | hand-written schema | Excellent: per-question-index validation, snake_case rules spelled out | **Model behaviour.** |
| `Fspec` | per-provider facades | §2.3/§2.4: facade `Invalid arguments: {e}` (weak) → dispatcher `Invalid args for fspec command {cmd}: failed to parse args: {e}` (names command, no usage) | **Largest failure volume in the system** (every ACDD workflow goes through it). The `args` string-vs-object gotcha is the top cause. |
| `AgentManager` | hand-written schema | §2.1 serde only; `timeout` string silently→None; `message`-vs-`await_idle` session_id type inconsistency | **Weak.** |
| `SessionSearch` | hand-written schema | §2.1 serde; numeric fields silently→None | **Weak.** |
| `GraphSearch` | hand-written schema | §2.1 serde; numeric fields silently→None | **Weak.** |
| `DeepSearch` | schemars | §2.1 serde; `scope` lenient (string/array); numeric fields silently→None | **Weak.** |
| `Schedule` | hand-written schema | Good (in-handler validation: "Schedule name is required", "Cron expression is required", …) | Good. |
| `bridge` | provider facades (Claude nested / Gemini flat) | "Missing 'action' field" / "Missing 'action.type' field" / "Missing 'url' field for connect action" | Good. |
| `ConnectMCP` | schemars | "name is required for connect action" / "command is required for stdio transport" / "url is required for http transport" | Good. |
| `mcp__<server>__<tool>` (dynamic) | remote schema | "MCP server '{server}' is not connected" / "MCP tool error: {remote error}" / "Invalid MCP tool name: {name}" | Passthrough; quality depends on remote server. |
| `unified_exec` (internal, Codex `exec_command`) | provider facade | Excellent: "Unknown action: {action}. Valid: run, write, poll, list, close" | **Model behaviour.** |
| `inject_summary` | schemars | handler errors | Internal plumbing. |
| `profile` (AgentManager profile action) | hand-written | `profile_session_active` JSON error | OK. |

### Non-tool failure surfaces

| Surface | Behaviour | LLM-visible? |
|---|---|---|
| Truncated tool call (max_tokens) | Structured recovery prompt injected (PROV-040) | Yes — **best-in-system** |
| Prompt-too-long terminal error | `strip_failed_tool_call_tail` removes the failed pair from context (BUG-170) | Partially (error text, then context surgery) |
| Provider API errors (rate limit, 5xx) | transient-error retry / stall detection | Yes, as stream errors |
| Pre-tool-hook / blocklist / stage-permission | `ToolError::Blocked` with rule reason | Yes |
| TUI red/green card | `detect_tool_error` prefix-matching | **No (display only)** — and out of sync with current error strings |
| Standalone CLI clap errors | Commander.js byte-parity re-render | Shell users only |

---

## 4. Systemic gaps (synthesis)

1. **The rig-layer `serde_json::from_str` failure is the worst offender.** It is the *first* failure that fires for any arg mistake, it has the broadest blast radius (every tool, every provider), and its messages are:
   - missing the tool name,
   - missing the accepted-parameters list,
   - missing an example of a correct call,
   - missing a pointer to per-tool documentation (fspec has one: `"<cmd> --help"`; other tools have none at all),
   - sometimes *silently swallowed* by the `serde_coerce` Option-deserializers (wrong type → `None`, no error).
2. **No "did you mean" for fspec command names** (`Unknown fspec command: {x}`), while the same system *does* list valid enum variants for a bad action value. The asymmetry is accidental (serde vs. hand-written).
3. **The fspec `args` string-vs-object gotcha** is the single most common LLM mistake for the Fspec tool (LLMs trained on OpenAI/Anthropic function-calling emit objects; the tool schema requires `args` to be a *string* of JSON on the Claude/OpenAI facades). The error for this is the weakest in the whole system: `Invalid arguments: invalid type: map, expected a string at line 1 column N` — no tool name, no statement of the correct shape, no example. The tool *description* even says `args` defaults to `"{}"`, which teaches the LLM the string form, but the error on violation does not reinforce it.
4. **Error-string → error-classification coupling in the TUI** (`detect_tool_error`) means the *human-visible* classification of tool failures is a hand-maintained prefix list that is already out of sync (Validation errors render as success). This is not an LLM problem but it hides LLM-relevant failures from the operator.
5. **Recovery affordances exist but are not connected to failures.** The system has:
   - per-command `--help` pages with a **`COMMON_ERRORS` section** (fspec-core help configs — each command documents its own common errors + fixes, e.g. add_attachment's "Work unit 'AUTH-001' does not exist / Fix: …"),
   - a structured truncation-recovery prompt (PROV-040),
   - the `NotYetPorted` error (which embeds recovery guidance in the error itself),
   - the `ParseJson` caret-snippet diagnostics (RPC-334).
   None of these fire *automatically* on a failed call. The LLM must *know to ask for help* after failing. The `NotYetPorted` message proves the pattern works: it tells the LLM exactly what to do next.
6. **Inconsistent error phrasing for the same failure class** (missing required field): `Missing 'x' field` (facades) / `Missing or empty required 'x' field` (param_extract) / `missing required argument: x` / `missing required 'x' argument` / `missing required field `x`` / serde's `missing field `x``. An LLM that pattern-learns from one tool's error style is not helped by the next tool's different style.
7. **Double-wrapping** in the fspec path: `Invalid args for fspec command X: failed to parse args: {serde}` — two wrappers, no information in either.

---

## 5. Recommended direction (input for the implementation card)

A single, central **"invalid arguments" envelope** at the boundary where raw LLM args meet each tool, containing:

1. **The tool name** (and, for fspec, the sub-command).
2. **What went wrong**, classified: `malformed_json` | `missing_required_field` | `unknown_field` (with the suspect field name) | `wrong_type` (with expected type + received type) | `unknown_action` (with the valid-action list) | `out_of_range`.
3. **The accepted parameter schema** for that tool (compact: name, type, required, default) — rendered from the same source as the tool definition (schemars / hand-written `parameters` JSON), so it can't drift.
4. **One canonical example** of a correct call, taken from the tool's own `examples`/`definition` where one exists (fspec has them per command; AgentManager/GraphSearch schemas already embed per-field usage notes).
5. **A help pointer** where one exists: fspec → `command: "<cmd> --help"`; others → "see the tool description".
6. **Silent-coercion guard**: `serde_coerce` Option-deserializers should not swallow *type* errors for *required semantics* — at minimum, the known-optional-but-typed fields (timeouts, depths, limits) should surface a *warning* suffix on the tool result when a value was sent but unparseable ("note: `timeout` was sent as "abc" and ignored; expected a number"), so the LLM sees its mistake instead of the tool quietly changing behaviour.
7. **Fuzzy match on action/field names** ("did you mean `spawn`?" / "did you mean `list-work-units`?") — the infrastructure for fspec command names exists (162 canonical names), and serde's enum errors already show that listing valid values is cheap and useful.
8. **Repair the TUI classifier** to classify by *structured* error data (have tools emit a stable machine-readable error marker, e.g. a leading `ToolError{validation|execution|…}` tag or a JSON envelope `{"__error":true,…}`) instead of the drifting string-prefix list.

Implementation seams identified during research:

- `patches/rig-core/src/tool/mod.rs` `ToolDyn::call` (lines 183-195) — the single choke point for all arg deserialization failures; the tool *name* is available one frame up (`ToolSet::call`) and can be threaded down.
- `codelet-tools` `ToolError` (`tools/src/error.rs`) — already carries `tool: &'static str` on every variant; adding a `UsageHint`/`SchemaRef` payload here is a natural extension, and every `ToolError` already flows to the LLM via `Display`.
- `fspec-core` `FspecCoreError::InvalidArgs` + the 162 `serde_json::from_str(args_json).map_err(...)` sites — a shared `parse_args::<T>(command, args_json)` helper would centralise the envelope for the whole fspec surface.
- `tools/src/facade/*` `map_params` implementations — the hand-written missing-field errors should delegate to the shared helper so phrasing becomes uniform.
- `serde_coerce.rs` — the silent-swallow Option deserializers need a diagnostics channel (or per-field `#[serde(deny_unknown_attributes)]`-style strictness where the field is semantically required).

---

## 6. Source references (files inspected)

| Area | Files |
|---|---|
| Rig tool pipeline | `rust/patches/rig-core/src/tool/mod.rs`, `rust/patches/rig-core/src/tool/server.rs`, `rust/patches/rig-core/src/agent/prompt_request/streaming.rs` (`parse_tool_result_content`, tool-error→string at lines 620-627), `rust/patches/rig-core/src/agent/prompt_request/mod.rs` (non-streaming path) |
| Unified tool error | `rust/tools/src/error.rs` (`ToolError`), `rust/tools/src/lib.rs` (`ToolOutput`) |
| Lenient coercion | `rust/tools/src/serde_coerce.rs`, `rust/tools/src/facade/param_extract.rs` |
| Fspec tool | `rust/tools/src/fspec.rs`, `rust/tools/src/fspec_handler.rs`, `rust/tools/src/facade/fspec_facade.rs`, `rust/tools/src/facade/wrapper.rs` (FspecToolFacadeWrapper), `rust/napi/src/agent_loop.rs` (fspec handler registration) |
| fspec dispatcher | `rust/fspec-core/src/dispatch.rs`, `rust/fspec-core/src/error.rs`, `rust/fspec-core/src/canonical.rs`, `rust/fspec-core/src/commands/*.rs` (162 command modules), `rust/fspec-core/src/help_dispatch.rs`, `rust/fspec-core/src/help/mod.rs` + `help/configs/*` |
| JSON diagnostics | `rust/fspec-json-error/src/lib.rs`, `rust/fspec-core/src/io/json_error.rs` |
| Per-tool validation | `rust/tools/src/{read,write,edit,bash,grep,glob,ls,astgrep,astgrep_refactor,web_search,bridge,mcp,unified_exec,apply_patch,done,request_user_input,deep_search,graph_search,session_search,agent_manager,schedule,inject_summary}.rs` |
| Recovery pipelines | `rust/cli/src/interactive/{error_classifiers,recovery_truncation,recovery_unrecoverable,stream_handlers}.rs` |
| TUI classification | `rust/cli/src/interactive/stream_handlers.rs` (`detect_tool_error`) |
| CLI front door | `rust/fspec/src/main.rs` (`render_clap_error`) |
| Prior specs | `spec/features/fspec-tool-rust-dispatcher.feature`, `spec/features/truncated-tool-call-recovery-auto-chunk-large-writes-and-retry-on-max-tokens.feature`, `spec/features/stop-reason-lost-in-streaming-output-truncation-silently-treated-as-normal-completion.feature`, `spec/features/strip-failed-tool-call-tail-on-unrecoverable-api-error.feature`, `spec/features/research-tools-fail-when-invoked-via-fspec-tool-reads-process-argv-instead-of-commander-args.feature` |
