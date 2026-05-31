# RPC-059 — Lift loop store into `codelet-core::loops`; `/loop` subcommand handler

**Parent:** RPC-030 · **Phase:** 7.6 · **Estimate:** 5 pts · **Depends on:** RPC-058

## Goal

Lift the loop-store (session-scoped recurring prompts) from `codelet/napi/src/session_manager.rs` (`loop_register` line 7319, `loop_cancel` line 7382, `loop_list` line 7390) into `codelet-core::loops`. New RPC methods + `/loop` subcommand handler matching TS `loop-service.handleLoopCommand`.

## TS reference

`SLASH_COMMANDS[13]` syntax: `[interval] <prompt> | cancel <id> | list`. The TS service is `src/tui/services/loop-service.ts`. `requiresSession: false` (can be invoked pre-session).

## Lift target

`codelet/core/src/loops/mod.rs`:

```rust
pub struct LoopRegistry {
    loops: RwLock<Vec<RegisteredLoop>>,
    on_trigger: Arc<dyn Fn(LoopTrigger) + Send + Sync>,
}

pub struct RegisteredLoop {
    pub id: String,
    pub session_id: SessionId,
    pub interval: Duration,
    pub prompt: String,
    pub created_at: DateTime<Utc>,
    pub next_trigger: DateTime<Utc>,
}

impl LoopRegistry {
    pub fn register(&self, session_id: SessionId, interval: Duration, prompt: String) -> RegisteredLoop;
    pub fn cancel(&self, id: &str) -> Result<(), String>;
    pub fn list(&self) -> Vec<RegisteredLoop>;
    pub fn start_runner(&self);
}
```

Loops trigger every `interval` and inject the prompt into the bound session. Cancelled when session is destroyed.

## Backend trait additions

```rust
fn loop_add(&self, session_id: &SessionId, interval_secs: u64, prompt: String) -> Result<RegisteredLoop, String>;
fn loop_cancel(&self, id: &str) -> Result<(), String>;
fn loop_list(&self) -> Vec<RegisteredLoop>;
```

Wire type `RegisteredLoop` in `codelet-rpc-types`.

## Parser

```
/loop 30s <prompt>      ← every 30 seconds
/loop 5m  <prompt>      ← every 5 minutes
/loop 1h  <prompt>      ← every hour
/loop cancel <id>
/loop list
```

Implement in `codelet/fspec-tui/src/app/loop_parser.rs`:

```rust
pub enum LoopSubcommand {
    Add { interval: Duration, prompt: String },
    Cancel(String),
    List,
}

pub fn parse(args: &str) -> Result<LoopSubcommand, String>;
```

`parse_duration` accepts `30s`, `5m`, `1h`, `1d` etc.

## Dispatcher

```rust
SlashCommandAction::Loop => {
    // Args parsed by submit-line handler.
}

Action::LoopAdd { interval, prompt } => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        self.emit_notice("/loop: no active session");
        return;
    };
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        match backend.loop_add(session_id.clone(), interval.as_secs(), prompt).await {
            Ok(loop_entry) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[loop] registered {} every {:?}", loop_entry.id, interval),
                });
            }
            Err(e) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[error] /loop: {e}"),
                });
            }
        }
    });
}
```

## Lifecycle

When a session is destroyed (`destroy_session`), the `LoopRegistry` cancels all loops bound to it. Implement in `SessionManager::destroy_session`.

## Acceptance criteria

1. `codelet/core/src/loops/mod.rs` exists. No NAPI dependency.
2. NAPI `loop_register`, `loop_cancel`, `loop_list` (`session_manager.rs` lines 7319, 7382, 7390) become re-export shims (final removal in RPC-043 already covers them).
3. `/loop 30s "check the build"` registers a loop that injects the prompt every 30 seconds.
4. `/loop list` lists all loops.
5. `/loop cancel <id>` cancels.
6. Loops auto-cancel on session destroy.
7. Integration test in `codelet/fspec-tui/tests/loops.rs` covers add/list/cancel/auto-cancel.

## Risks

- Interval too short (e.g. 1s) floods the session. Document minimum interval (or accept user choice).
- `next_trigger` calculation must persist across fspec restarts.

## Out of scope

- Cross-session loops (TS doesn't support either).
