# RLCD Integration — Research & Design (fspec)

Investigation date: 2026-09-24. Investigator: fspec agent session (supervisor: R. Quast).

## 1. Naming decision: "RLCD", not "laya"

The current decision backend is **laya-rs** (Convai's "System 1" typed-decision
engine reimplementation on candle). The product name `laya` is expected to be
renamed to **`rlcd`** in the near future (the RLCD — Reinforcement Learning
Continual Decisions — training loop is the conceptual core: REINFORCE with
Gaussian exploration + group-mean baseline over a strictly-proper scoring
rule).

**Convention for all fspec code, cards, feature files, and config:**

- Use `rlcd` for every identifier, module, config key, log line, and doc
  reference: module `rust/tools/src/rlcd/`, config section `rlcd`, work-unit
  prefix `RLCD`, status type `RlcdStatus`, client trait `RlcdClient`, log
  `rlcd-serve.log`, pidfile `rlcd.pid`.
- `laya` appears ONLY as: (a) the name of the current backend binary
  (`laya serve`), (b) the checkpoint directory (`~/.cache/laya-rs/`),
  (c) protocol-doc filenames (attached to this epic's cards).
- The HTTP client is a small trait (`RlcdClient`) with exactly one
  implementation (`HttpRlcdClient` speaking the Jev/Simple-Jev v1 protocol).
  A future renamed backend or different decision engine implements the same
  trait — no fspec code outside `rust/tools/src/rlcd/client.rs` may mention
  the backend name.
- The backend binary name is config-driven: `rlcd.spawn.binary` (default
  `"laya"`). When the upstream rename lands (`rlcd` binary), users flip one
  config key — zero fspec code changes.
- Config key `rlcd.backend = "laya"` (default) documents which
  binary/protocol family the spawn path targets; the client trait is
  protocol-stable regardless.

## 2. Current backend — laya-rs (verified live on this machine, 2026-09-24)

Facts confirmed by direct observation (not copied from docs):

- Binary: `/home/rquast/.local/bin/laya` (on PATH via `which laya`).
- Checkpoint: `~/.cache/laya-rs/laya-typed-decisions` (~842 MB, English,
  `typed-decisions` variant). `multilingual` variant exists upstream.
- `laya serve` with no args: binds `127.0.0.1:8000`, reports
  `{"status":"ready","model":"typed-decisions"}` on `GET /health` in ~2 s
  (checkpoint already cached). Model load dominates; allow ≥ 60 s budget for
  cold machines (first-run checkpoint download is ~843 MB).
- Serve flags (JEV-006, verified in README):
  ```
  laya serve [--model DIR | --model-variant KEY [--models-root DIR]]
             [--host ADDR] [--port N] [--max-model-len N]
             [--max-request-branches N] [--max-queued N]
  ```
  Defaults: host `127.0.0.1`, port `8000`, max-request-branches `100`,
  max-queued `16`. Graceful on SIGTERM/Ctrl-C.
- Endpoints: `POST /v1/classifier` (alias `POST /v1/systemone`),
  `GET /health`, `GET /openapi.json`. Full contract: attached
  `protocol-reference.md`.

### Protocol invariants the supervisor/client MUST honor

1. `request.model` must equal the loaded model's name/path (the server
   reports it, e.g. `"typed-decisions"`). **Read it from the `model` field of
   the `/health` response — never hardcode.** (Server rule: "Loaded model is
   '<name>'" 422 on mismatch.)
2. Exactly one of `state` (string | JSON object | array) or `messages`
   (non-empty text-only chat history) — XOR; both/neither → 422.
3. Questions: `choice` (criteria map, 2–50), `score` (criteria array, 2–50,
   order = rubric lowest→highest), `noul` (criteria `{true,false}` optional).
   Unknown question/option fields → 422. Max 100 questions/request (schema
   hard cap 256; server default 100), body ≤ 1 MiB.
4. Responses: choice → `{choice, confidence, probabilities}`; score →
   `{score (fractional rubric index), confidence, probabilities, legend}`;
   noul → `{noul: P(true)}`.
5. Errors: structured 422 envelope `{error: {message, type, code, param,
   details[]}}`; 429 `{"detail":"Scoring queue is full"}` + `Retry-After: 1`
   (admission queue: 1 in-flight forward + 16 queued); 499 client
   disconnected; 500 unhandled.

### Operational lessons (AUDIT-001 — laya was already run over this
codebase's 7,542 coverage mappings; see attached AUDIT_PROCESS.md)

- **CPU aarch64 speed**: ~1.5 s/question amortized over 60-question batches;
  ~12 s for 10 questions. A resident server (this design) avoids the ~842 MB
  per-invocation reload of `laya answer` CLI mode. RAM ≈ 2.5 GB per server
  process — do NOT spawn more than one.
- **Confidence semantics**: absolute confidence values run low even on
  correct calls (audit observed means ≈ 0.025 aligned / 0.006 divergent on a
  hard semantic task; simple 2–3 option questions read higher, e.g. 0.69 on
  a clean refund intent). Design all thresholds against **probability spreads
  and relative ordering**, treat absolute values as soft signals, and make
  thresholds configurable (defaults in §4/§6/§7).
- **Sequence budget** (for building good states, not for the supervisor):
  `max_len=1024`, options/instructions get ~256-token head, **state is
  placed last and its tail is truncated** on overflow. ⇒ put the most
  important content first in state strings; keep question criteria ≤ ~8 words.
- **State as plain string** when order matters (JSON objects get
  alphabetically sorted keys before tokenization).

## 3. fspec codebase integration points (verified 2026-09-24)

### 3.1 Config system (DRY/SOLID reuse — do NOT build a new one)

- User dir: `fspec_user_dir()` in `rust/sessions/src/profile_sections.rs`
  (`FSPEC_USER_DIR` env override → `dirs::home_dir()/.fspec` → `$HOME/.fspec`).
  The env override is what makes config tests hermetic (no real `~/.fspec`
  touched).
- Key-preserving read/write: `read_config_value(path)` /
  `write_config_value(path, &Value)` (same file), pretty-print + trailing
  newline, `preserve_order` serde keeps key order.
- Canonical example of the add-a-section pattern:
  `save_persisted_model_string_to()` (PROV-122) — read-merge-write of
  `tui.lastUsedModel`: missing/malformed file starts from `{}`, non-object
  root replaced with fresh object, nested object created when absent,
  unrelated keys untouched, persistence is best-effort (Err returned and
  logged, never fatal).
- **Decision (HITL, 2026-09-24): user-config ONLY** — a new top-level `rlcd`
  section in `~/.fspec/fspec-config.json`. No project-level
  `spec/fspec-config.json` merge (one machine = one RLCD server).

### 3.2 Blocklist middleware (RLCD security layer target)

`rust/tools/src/blocklist/` (`mod.rs`, `config.rs`, `matcher.rs`,
`middleware.rs`, `template.rs`):

- `check_bash_command(command, session_id)` — called by `BashTool::call`
  (`bash.rs:201`) + `call_with_streaming` (`bash.rs:138`) + `UnifiedExecTool`
  (`unified_exec/tool.rs:197`).
- `check_file_path(file_path, session_id)` — called by `ReadTool`
  (`read.rs:294`), `WriteTool` (`write.rs:91`), `EditTool` (`edit.rs:94`),
  `ApplyPatchTool` (`apply_patch/mod.rs:39`).
- Flow today: regex pass over merged `~/.fspec/blocklist.json` +
  `.fspec/blocklist.json` rules (project first, first-match-wins, Allow
  overrides Block) → `blocked` ⇒ `BlockedError` (hard reject with reason +
  guidance); `prompt` ⇒ session-allowance check (`is_session_allowed`) ⇒
  else `pause_for_user(PauseKind::Triple)` ⇒ AllowOnce / AllowSession
  (`allow_for_session(pattern)` stored in `SESSION_ALLOWANCES`, cleared on
  TUI restart) / Deny.
- `check_command_raw` (NAPI surface) uses the cached matcher;
  `check_bash_command`/`check_file_path` reload config on every call (hot
  reload by design — the RLCD stage plugs in at the same spot, after the
  regex decision is `allowed` or `prompt`).

### 3.3 Fspec workflow gate chokepoint

- Agent/TUI mode: LLM → `FspecTool` (`rust/tools/src/fspec.rs`, rig tool,
  JS-callback execution) → `FspecToolFacadeWrapper` →
  `execute_fspec_command_for_session(session_id, request)`
  (`rust/tools/src/fspec_handler.rs`) → per-session `FspecHandler` → TS.
  `FspecRequest {command, args_json, project_root, provider}` /
  `FspecResult {success, data, error, system_reminder}`.
- Standalone CLI mode: `codelet-fspec` binary → `codelet-fspec-core` command
  `run(args_json, project_root)` (two-front-doors invariant — the same
  `run()` serves the LLM dispatcher and the shell subcommand).
- **Decision: the gate lives in the agent-mode tool path** (the
  `execute_fspec_command_for_session` boundary, keyed by `session_id`).
  Rationale: the Triple-pause HITL system, session allowances, and
  block-notification callbacks exist only in the agent tool layer;
  `fspec-core` has no session context. CLI-mode fspec invocations remain
  ungated (documented in the feature file).

### 3.4 Decision tool registration

`DecisionTool` (new) is registered alongside `DeepSearchTool` /
`ScheduleTool` in every provider's `create_rig_agent` (verified line
anchors, 2026-09-24):

- `rust/providers/src/claude.rs:553/557`
- `rust/providers/src/openai.rs:465/469`
- `rust/providers/src/gemini.rs:211/215`
- `rust/providers/src/codex/mod.rs:423/427`
- `rust/providers/src/zai.rs:281/285`
- `rust/providers/src/custom/custom_provider.rs:305/309` (+ second agent at 330/334)
- `rust/providers/src/copilot/rig_agent.rs:102/106`

Tool pattern to mirror: `DeepSearchTool` (`rust/tools/src/deep_search/`) —
rig `Tool` impl, `pre_tool_hook_check` first, `ToolError` variants
(`Blocked`/`Validation`/`Execution`), hand-written JSON schema in
`definition()`. Decision needs no per-session handler registry — it is a
self-contained HTTP call (more like `WebSearchTool`), using the shared
`RlcdClient` from RLCD-001.

### 3.5 Cross-cutting constraints

- Workspace lints: no `unwrap()`/`panic!`/`unsafe`; thiserror for public
  errors; tracing for logs; files < 300 lines (refactor before continuing);
  no new tokio runtimes — `Handle::current()` only.
- Tests: scoped `cargo test -p <crate> --test <name>` (NEVER unscoped
  workspace runs); `tempfile`, `serial_test` for global-state tests
  (blocklist statics, session allowances); `FSPEC_USER_DIR` for config
  tests; a mock axum/hyper server on an ephemeral port for supervisor tests;
  the live backend binary for ONE integration test gated on the binary being
  on PATH + checkpoint present (skip silently otherwise — mirror laya-rs's
  `LAYA_TEST_MODEL`/`LAYA_TEST_BINARY` gating convention).
- `reqwest` is already a workspace dep (codelet-tools uses it for web
  search) — use it; no new deps expected.

## 4. Supervisor design (RLCD-001)

Module: `rust/tools/src/rlcd/` with `config.rs` (config load/save +
defaults), `service.rs` (status + health polling + spawn), `client.rs`
(`RlcdClient` trait + `HttpRlcdClient`), `error.rs`.

### Config schema (user-config only)

```json
"rlcd": {
  "enabled": true,
  "url": "http://127.0.0.1:8000",
  "backend": "laya",
  "spawn": {
    "bindAddress": "127.0.0.1",
    "port": 8000,
    "portScanLimit": 10,
    "binary": "laya"
  },
  "security": { "checkMode": "all", "blockThreshold": 0.9, "promptThreshold": 0.6 },
  "gate": { "commands": ["update-work-unit-status"] }
}
```

- `url` — full URL of the server to talk to. ANY host/port (remote allowed:
  connect-only, never spawned). Default `http://127.0.0.1:8000`.
- `backend` — informational tag for the protocol family (default `"laya"`;
  the Jev v1 contract).
- `spawn.bindAddress` — address to bind when auto-spawning a local server.
  Default `127.0.0.1`; set `0.0.0.0` to expose on all interfaces. Only used
  when `url` is local.
- `spawn.port` — first port tried when auto-spawning. Default `8000`.
- `spawn.portScanLimit` — number of ports to try starting at `port`
  (default 10 → port..port+9).
- `spawn.binary` — backend executable resolved on PATH (default `"laya"`;
  flip to `"rlcd"` when the upstream rename lands).
- `security.*` — thresholds + check mode for RLCD-004.
- `gate.commands` — fspec command names to gate (RLCD-003).
- All fields optional → defaults; lenient deserialization (follow
  `de_opt_u32_lenient`-style coercion so TS-written floats don't drop the
  section); unknown keys preserved verbatim on write.
- Loader: `load_rlcd_config() -> RlcdConfig` (env-resolved) +
  `load_rlcd_config_from(path)` (path-injectable core, testable with
  `tempfile` + `FSPEC_USER_DIR`).

### Status + polling

- Shared status: `RLCD_STATUS: RwLock<RlcdStatus>` where
  `RlcdStatus { reachable: bool, base_url: String, model: Option<String>,
  last_check: Instant, spawn_note: Option<String> }`.
- A background tokio task (spawned once on first use via a `OnceCell`-style
  guard on the status) loops: `GET {url}/health` (2 s timeout) → ready ⇒
  `reachable=true, model=Some(model)`; failed ⇒ local URL + spawn enabled ⇒
  ensure-spawned (below) then keep polling; **polling cadence 10 s**
  (config-able). The task never blocks tool calls — callers read the status
  snapshot and, when not ready, run a short foreground wait (up to ~10 s)
  bounded by the request timeout.
- `/health` parse: `{"status":"ready","model":...}`; treat any other shape
  as unreachable.
- One `tracing::warn`/`info` per state transition (down→up, up→down,
  spawn-fail), not per poll, to avoid log spam.

### Ensure-spawned (local URLs only)

Triggered only when the host part of `url` ∈ {`127.0.0.1`, `localhost`}
**and** the configured binary resolves on PATH (`which <binary>`).
Remote hosts: never spawn — status stays `unreachable` and the fail-open
policy applies. Windows: spawn path degrades to connect-only (documented;
the `process_group(0)` + null-stdin detach pattern is Unix-shaped).

Spawn procedure:
1. **Pidfile**: `~/.fspec/rlcd/<backend>.pid` (today `laya.pid`; dir name
   stays `rlcd`). If a live pid exists (kill(pid,0) ok) and health is down →
   it's probably still loading: wait out the poll budget instead of
   double-spawning. Stale pidfile → remove.
2. **Port scan** (on `spawn.bindAddress`, `spawn.port`..+`portScanLimit-1`):
   a. Try `TcpListener::bind((probe_addr, port))` (probe on `127.0.0.1` even
      when binding `0.0.0.0`): Ok → free, use it (drop the probe listener).
   b. Err (occupied) → `GET http://127.0.0.1:{port}/health` (500 ms): if it
      returns the ready shape, that occupant IS a compatible server →
      connect to it, no spawn.
   c. Else try next port. All ports exhausted (occupied by non-RLCD) → spawn
      fails → fail-open policy.
3. **Spawn**: `std::process::Command` —
   `<binary> serve --host <bindAddress> --port <chosen>` detached:
   stdout/err → `~/.fspec/logs/rlcd-serve.log` (append), stdin from null,
   `process_group(0)` so it survives the fspec process; write the pidfile
   afterwards.
4. Poll `/health` at the chosen port up to 60 s (model load / first-time
   checkpoint download). On success: status reachable; **the effective URL
   (127.0.0.1, chosen port) is what callers connect to** even if the
   configured `url` port differed (log the switch via `tracing::info` +
   block notification).

### Fail-open policy (HITL decision, 2026-09-24)

When RLCD is unreachable (remote down, local down, spawn failed, binary
missing): every consumer DEGRADES, never hard-blocks:
- Decision tool → structured error output with the reason + hint (start the
  backend server or set `rlcd.url`).
- Workflow gate → command executes (tracing::warn logged).
- Security layer → regex blocklist + stage permissions remain fully in
  force; RLCD stage skipped (tracing::warn).

## 5. Decision tool (RLCD-002)

`DecisionTool` in `rust/tools/src/rlcd/decision.rs`:

- `DecisionArgs { state: Value (string|object|array), questions:
  Map<String, QuestionDef> (hand-written JSON schema: type
  choice/score/noul, instructions, criteria), model: Option<String> (defaults
  to the server-reported model), url: Option<String> (override) }`.
- `call()`: `pre_tool_hook_check` → arg validation (state non-empty;
  questions 1..100; criteria sizes) → ensure status ready (bounded wait) →
  `RlcdClient::classify(model, state, questions)` → render answers compactly
  (choice + confidence + probabilities; score + legend; noul probability +
  usage).
- Error mapping: 422 → first detail(s) surfaced (schema is the agent's fault
  — actionable); 429 → "RLCD queue busy, retry in {Retry-After}s";
  500/timeout → Execution error; unreachable → fail-open error text (§4).
- Registered in the 7 `create_rig_agent` sites (§3.4). NOT added to
  sub-agent tool lists (DeepSearch sub-agents etc.) in this epic.

## 6. Workflow gate (RLCD-003)

Intercept point: `execute_fspec_command_for_session` (agent mode only —
§3.3 decision).

- `gate.commands` config list (default: `["update-work-unit-status"]`).
  Extendable to `delete-work-unit`, `restore-checkpoint`, `auto-advance` by
  config without code changes.
- On a gated command: build RLCD state from `(command, args_json,
  project_root)` — for `update-work-unit-status` include workUnitId,
  from→to status, and (best-effort) the unit's title/epic read from
  `spec/work-units.json` in the same call. Ask ONE `choice` question:
  `criteria {proceed, hold, ask_user}` with instructions "Given this ACDD
  workflow command, should it proceed?".
- Mapping (thresholds under `gate` config: `holdThreshold` default 0.65 on
  `hold`, `askThreshold` default 0.40 on `ask_user`, `proceedMargin`
  default 0.15):
  - argmax `proceed` and `P(proceed) − max(others) > proceedMargin` → execute.
  - argmax `hold` (or `P(hold) ≥ holdThreshold`) → REJECT: return
    `FspecResult { success: false, error: "RLCD gate: <reason> …" }` so the
    agent sees it and can react (retry with corrected args, or explain to
    the user).
  - argmax `ask_user` (or `P ≥ askThreshold`) → `pause_for_user(PauseKind::Triple)`
    with the command + RLCD rationale in the message; AllowOnce /
    AllowSession (allowance keyed `rlcd-gate:<command>` via the existing
    `allow_for_session` machinery) / Deny; Deny ⇒ reject.
  - Unreachable (fail-open) → execute, log warn.
- The gate NEVER mutates fspec state itself; it only allow/reject/ask.
- State-construction is best-effort: if `spec/work-units.json` can't be
  read, the gate proceeds with the raw command/args state (no failure).

## 7. Blocklist security layer (RLCD-004)

New stage inside `check_bash_command` / `check_file_path`
(`middleware.rs`), executed AFTER the regex pass resolves to `allowed` or
`prompt`:

- Gate: `rlcd.enabled && security.checkMode ∈ {"all", "prompt-only"}`
  (`all` = every regex-allowed operation; `prompt-only` = only operations
  the regex pass already flagged `prompt` — the latency-saver mode;
  default `all`) AND RLCD status reachable (fail-open otherwise).
- Session suppression: a single pseudo-pattern `rlcd-check` in
  `SESSION_ALLOWANCES` — when set (user chose AllowSession on an RLCD
  prompt), all subsequent RLCD security checks for the session are skipped
  until TUI restart (reuses `allow_for_session` / `is_session_allowed` /
  `clear_session_allowances` unchanged).
- One `noul` question per checked operation:
  - Bash: state = `tool=Bash; command=<command>; cwd=<effective cwd>`;
    instructions: "Is this shell command risky, destructive, or an
    unintended escalation of what the session has been doing?"
  - File: state = `tool=<Read|Write|Edit|ApplyPatch>; path=<abs path>;
    operation=<read|write|edit|patch>`; instructions: "Is this file access
    risky, or does it target sensitive data the session should not touch?"
- Mapping (`p = noul P(true)`; thresholds from config, defaults in §4):
  - `p ≥ blockThreshold` (0.90) → `BlockedError` (hard reject; reason =
    "RLCD security check flagged this operation") — same shape as a regex
    block so existing `ToolError::Blocked` handling works unchanged.
  - `p ≥ promptThreshold` (0.60) → `pause_for_user(PauseKind::Triple)`
    (tool_name = actual tool; message carries the RLCD suspicion; details =
    the command/path). AllowOnce / AllowSession (sets the `rlcd-check`
    allowance) / Deny — Deny ⇒ `BlockedError("User denied access")`.
  - else → allow.
- Latency guard: per-session check counter (`RLCD_CHECK_COUNTER`,
  `RwLock<HashMap<Uuid, u32>>`) with `maxChecksPerSession` (default 200) —
  above it, skip with a one-time warn (prevents a runaway agent from burning
  minutes of RLCD calls; 0 = unlimited).
- Ordering guarantee: the regex blocklist is ALWAYS evaluated first
  (deterministic rules are the hard-fact layer); RLCD only adds a semantic
  second stage. An RLCD decision can never override an `Allow` regex rule.

## 8. Testing strategy (all cards)

- Config: path-injectable cores + `FSPEC_USER_DIR`/`tempfile`; round-trip
  (write section, read back, unrelated keys preserved verbatim); malformed
  file → defaults + warn; float coercion.
- Supervisor: axum mock server on an ephemeral port for health
  ready/down shapes; port-scan logic against a real occupied port (bind a
  dummy listener in-test, plus an in-test "health-shaped" responder for the
  occupant-detection branch); spawn path: ONE integration test gated on
  `which laya` existing AND a checkpoint cache dir existing (skip silently
  otherwise — mirror laya-rs's gating convention), generous timeout;
  pidfile stale/alive branches.
- Decision tool: mock server returning fixture responses (README refund
  example), 422/429/500 mapping, unreachable fail-open text, arg
  validation.
- Gate: registered `FspecHandler` stub + registered pause handler (existing
  `set_pause_handler` pattern from blocklist tests) — proceed / hold / ask
  branches + fail-open.
- Security layer: `serial_test` (blocklist statics + session allowances),
  temp project blocklists, pause-handler stubs (pattern already in
  `middleware.rs` tests), threshold branches via a mock `RlcdClient`
  (trait injection, not HTTP, for unit tests).
- Every test file: header comment naming its feature file + `// @step`
  comments per Gherkin step (ACDD).

## 9. Deliverables map (card → files)

| Card | Feature file(s) | New/changed code |
|---|---|---|
| RLCD-001 | `rlcd-service-supervisor.feature` | `rust/tools/src/rlcd/{mod,config,service,error}.rs` (+ re-exports in `lib.rs`), tests |
| RLCD-002 | `rlcd-decision-tool.feature` | `rust/tools/src/rlcd/{client,decision}.rs`, provider registrations ×7, tests |
| RLCD-003 | `rlcd-fspec-workflow-gate.feature` | `rust/tools/src/rlcd/gate.rs` + wiring in the fspec facade wrapper path, tests |
| RLCD-004 | `rlcd-blocklist-security.feature` | `rust/tools/src/rlcd/security.rs` + `blocklist/middleware.rs` hook, tests |

Tags: register `@rlcd` (feature group) at the start of the specifying
phase; component tags `@tools` / `@agent-core` as applicable. Estimates
(post-feature-file, ACDD rule): RLCD-001 = 5, RLCD-002 = 3, RLCD-003 = 5,
RLCD-004 = 5.

## 10. Open knobs (defaults chosen, config-exposed, revisit after first use)

1. `security.checkMode` default `all` (strongest posture; `prompt-only`
   available if the agent loop feels slow).
2. Gate default command list = `["update-work-unit-status"]` only; widen via
   config as trust builds.
3. `maxChecksPerSession` default 200.
4. Health poll cadence 10 s; foreground ready-wait 10 s; spawn poll budget
   60 s.
5. Backend rename (laya → rlcd binary/protocol) lands upstream: the blast
   radius by design is `service.rs` (spawn argv, pidfile name, log path) and
   `client.rs` (protocol shape only if the protocol itself changes).
