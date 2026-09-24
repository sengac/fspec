# AST Research: RLCD-004 — semantic security layer over the regex blocklist

Research date: 2026-09-24. Scope: `rust/tools/src/blocklist/`, `rust/tools/src/{bash,read,write,edit,unified_exec,apply_patch}/`, `rust/tools/src/rlcd/`, `rust/tools/src/tool_pause.rs`.

## 1. Regex blocklist call surface (where the stage hooks in)

| Call site | Function | Async context | Notes |
|---|---|---|---|
| `tools/src/bash.rs:138` | `check_bash_command(&args.command, session_id)` | in `async fn call` (rig Tool) | main Bash path; `cwd` available via `resolve_cwd(args.cwd)` / `get_effective_cwd` |
| `tools/src/bash.rs:201` | `check_bash_command` (streaming path) | in `async fn call_with_streaming` | delegates to `call` when no stream callback (single choke point) |
| `tools/src/unified_exec/tool.rs:197` | `check_bash_command(&check_str, session_id)` | in `async fn handle_run` | has `skip_blocklist` flag (TOOL-022 P4 double-check guard) |
| `tools/src/read.rs:294` | `check_file_path(&file_path_str, session_id)` | in `async fn call` | path already resolved absolute |
| `tools/src/write.rs:91` | `check_file_path` | in `async fn call` | same |
| `tools/src/edit.rs:94` | `check_file_path` | in `async fn call` | same |
| `tools/src/apply_patch/mod.rs:39` | `check_file_path(&p, session_id)` inside sync `validate_patch_path` | called from `async fn call` (3 PatchOp arms) | needs `.await` at call sites when becoming async |
| `tools/src/blocklist/middleware.rs` | `check_command_raw` (NAPI) | sync | returns `CheckResult`; NOT a tool path — unchanged |

`check_bash_command` / `check_file_path` are SYNC functions (middleware.rs). All rig-tool call sites are already in async contexts, so an async semantic entry point for file ops (`check_file_path_semantic`) fits without changing tool signatures; the Bash path needs a sync→async bridge (`block_in_place` + `Handle::block_on` guarded by `Handle::try_current()` flavor==MultiThread — the `codelet_sessions::profile_sections::probe_profile_models` convention, `sessions/src/profile_sections.rs:598`).

## 2. `CheckResult` / `BlocklistAction` (matcher.rs)

```rust
pub enum BlocklistAction { Block, Allow, Prompt }  // serde lowercase

pub struct CheckResult {
    pub allowed: bool,
    pub blocked: bool,
    pub reason: Option<String>,
    pub guidance: Option<String>,
    pub matched_rule_id: Option<String>,   // None when no rule matched
}
```

Semantics of `check_command`: first matching rule wins.
- `Block` -> `CheckResult::blocked(rule)` (blocked=true, matched_rule_id=Some)
- `Allow` -> `CheckResult::allowed()` (**matched_rule_id=None** — cannot be distinguished from "no rule matched"!)
- `Prompt` -> `{allowed:false, blocked:false, matched_rule_id:Some}` (the middleware pauses for Triple)
- no match -> `CheckResult::allowed()` (matched_rule_id=None)

**Gap found (RLCD-004 requirement):** an explicit `Allow` rule is indistinguishable from "no rule matched" in `CheckResult`. The security layer MUST never override an explicit Allow rule (deterministic intent), but "no rule matched" IS stage-eligible in checkMode `all`. FIX: add `matched_action: Option<BlocklistAction>` to `CheckResult` (set on Block/Allow/Prompt arms; None on no-match).

## 3. Session-allowance API (middleware.rs:355-375)

```rust
pub fn allow_for_session(pattern: &str)          // insert into SESSION_ALLOWANCES
pub fn is_session_allowed(pattern: &str) -> bool
pub fn clear_session_allowances()                // TUI restart path
```

`SESSION_ALLOWANCES: LazyLock<RwLock<HashSet<String>>>`. The RLCD stage reuses this with the single pseudo-pattern `"rlcd-check"` (RLCD-004 contract) — when set, ALL later RLCD security checks in the session skip until TUI restart. `clear_session_allowances` on restart covers it unchanged.

## 4. `BlockedError` (middleware.rs:27)

```rust
pub struct BlockedError { pub reason: String, pub guidance: Option<String>, pub rule_id: String }
```

Display: `"Blocked: {reason} {guidance?}"`. Every RLCD stage hard-reject/deny must carry `rule_id: "rlcd-security"` and a reason starting `"RLCD security:"` so `ToolError::Blocked { message: blocked.to_string() }` works unchanged at the 7 call sites.

## 5. Pause API (tool_pause.rs:38-86)

```rust
pub enum PauseResponse { Resumed, Approved, Denied, Interrupted, AllowOnce, AllowSession }
pub fn pause_for_user(session_id: Uuid, request: PauseRequest) -> PauseResponse  // Resumed when no handler
pub fn set_pause_handler(session_id, handler)  // test seam (also used by blocklist tests)
```

Stage mapping: `AllowOnce | Resumed` -> Ok (execute once); `AllowSession` -> `allow_for_session("rlcd-check")` + Ok; `Approved` (legacy non-triple response) -> Ok (execute — the regex prompt branch treats it the same way, `_ => Ok(())`); `Denied | Interrupted` -> `BlockedError { reason: "RLCD security: user denied access" }`.

## 6. RLCD client + config available for reuse (RLCD-001/002)

- `rlcd::client::{RlcdClient, HttpRlcdClient, RlcdQuestion{qid, type, instructions, criteria}}` — noul question: `criteria=None`.
- `rlcd::service::health_check(url, timeout) -> Result<String/*model*/, RlcdError>` — single /health probe; model name ALWAYS from the response (protocol rule, never hardcoded).
- `rlcd::config::load_rlcd_config()` — user config; `rlcd.security` section added by this work unit (`checkMode`, `blockThreshold`, `promptThreshold`, `maxChecksPerSession`).
- noul answer shape (RLCD-002): `answers.security = {type:"noul", noul:<f64>}`.

## 7. Latency guard design

Per-session counter `RLCD_CHECK_COUNTER: LazyLock<RwLock<HashMap<Uuid, u32>>>` in security.rs: incremented when the stage is entered (after the allowance/enabled/gate checks pass — capped sessions must not burn their counter on skipped checks… decision: increment on ENTRY of the stage evaluation, before the health probe, so a capped session deterministically stops after N engine consults). `maxChecksPerSession=0` = unlimited. One-time warn per session (separate `RLCD_SECURITY_WARNED` set, key `"<session>:cap"` / `"<session>:unreachable"`).

## 8. Failure-mode matrix (fail-open, HITL decision 2026-09-24)

| Condition | Behavior |
|---|---|
| `rlcd.enabled=false` | skip stage, Ok(()) |
| unknown `checkMode` value | skip stage, Ok(()) |
| `is_session_allowed("rlcd-check")` | skip stage, Ok(()) — no engine call |
| counter > maxChecksPerSession | skip stage, Ok(()) + one-time warn |
| no multi-thread runtime (Bash sync bridge) | skip stage, Ok(()) |
| /health unreachable / not ready | skip stage, Ok(()) + one-time warn |
| classify 4xx/5xx/timeout | skip stage, Ok(()) + one-time warn |
| missing/non-numeric `noul` field | p=0.0 -> execute |
| p >= blockThreshold | `BlockedError{reason:"RLCD security: P(risk)=…", rule_id:"rlcd-security"}` |
| p >= promptThreshold | Triple pause; Deny -> `BlockedError "RLCD security: user denied access"` |

## 9. Test seams

- blocklist config: temp `$HOME` with `.fspec/blocklist.json` (system path — `system_config_path()` reads `dirs::home_dir()`); `init_blocklist(None)` so no project rules leak in.
- RLCD user config: `FSPEC_USER_DIR` temp dir (rlcd::config::user_dir override).
- RLCD server: `rlcd_mock.rs` (included module) — /health ready + configurable /v1/classifier body.
- pauses: `set_pause_handler` stubs capturing `PauseRequest`.
- Bash tests MUST run on a multi-thread runtime (`#[tokio::test(flavor = "multi_thread")]`) for the block_in_place bridge.
