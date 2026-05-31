# AST Research — RPC-036: Widen `codelet-rpc-types`

Captured via the AstGrep tool against
`/Users/rquast/projects/fspec/codelet/`.

## 1. Current `pub struct` inventory in `codelet/rpc-types/src/lib.rs`

```
pub struct WorkUnitInfo                    @ lib.rs:37
pub struct CheckpointCounts                @ lib.rs:74
pub struct SessionId                       @ lib.rs:92
pub struct SessionInfo                     @ lib.rs:192
pub struct LogRecord                       @ lib.rs:212
pub struct HealthInfo                      @ lib.rs:237
pub struct ModelInfo                       @ lib.rs:285
pub struct WorkspaceInfo                   @ lib.rs:316
pub struct ModelEntry                      @ lib.rs:341
pub struct ProviderInfo                    @ lib.rs:362
pub struct CompactionProgress              @ lib.rs:375
pub struct ToolCallInfo                    @ lib.rs:383
pub struct ToolResultInfo                  @ lib.rs:391
pub struct ToolProgressInfo                @ lib.rs:399
pub struct ContextFillInfo                 @ lib.rs:408
pub struct SupervisorPendingInjectionInfo  @ lib.rs:417
pub struct IncomingMessageImage            @ lib.rs:424
pub struct CompactionResult                @ lib.rs:432
pub struct HistoryMatch                    @ lib.rs:447
pub struct FspecRequest                    @ lib.rs:455
pub struct FspecResult                     @ lib.rs:467
pub struct TokenTracker                    @ lib.rs:479
```

22 structs, none of them named `SessionTokens`, `TokenRestoreState`,
`SessionModel`, `WorkUnitContext`, `ThinkingConfig`, `PauseState`,
`HitlOption`, `HitlRequest`, `HitlResponse`, or `IsolatedSessionInfo`.
Each of the names RPC-036 introduces is provably new — there is no name
collision risk.

## 2. Current `pub enum` inventory in `codelet/rpc-types/src/lib.rs`

```
pub enum SessionStatus         @ lib.rs:134
pub enum ThinkingLevel         @ lib.rs:297
pub enum SessionState          @ lib.rs:507
pub enum NotificationSeverity  @ lib.rs:518
pub enum StreamChunk           @ lib.rs:534
```

5 enums. None of them named `PauseKind`, `PauseResponse`, or
`ApprovalChoice`, so RPC-036 can introduce them without shadowing.

## 3. Callers of `StreamChunk::isolation_state_change`

```
codelet/napi/src/session_manager.rs:3522 — StreamChunk::isolation_state_change(false, None)
codelet/napi/src/session_manager.rs:3776 — StreamChunk::isolation_state_change(true, Some(worktree_path.to_string_lossy().to_string()))
```

Two production call sites. Adding an additional non-`Option` argument
would break both. We will therefore preserve the existing 2-arg
constructor signature and default `base_commit` to `None` inside the
constructor body — the new field is `base_commit: Option<String>` so
this is sound.

## 4. Pattern destructurings of the `IsolationStateChange` variant

`Grep` (text-mode) reveals one pattern destructuring in
`codelet/napi/src/types.rs:332-339`:

```rust
StreamChunk::IsolationStateChange {
    is_isolated,
    worktree_path,
} => json!({
    "type": "isolationStateChange",
    "isIsolated": is_isolated,
    "worktreePath": worktree_path,
}),
```

Adding the new `base_commit` field requires updating this destructuring
to either bind the new field or use `..`. We will use the explicit
binding for safety and emit `"baseCommit": base_commit` in the JSON
relay value so the bridge surface stays in lockstep with the wire
shape.

## 5. Existing reference shapes inside `codelet-tools`

```
codelet/tools/src/tool_pause.rs — PauseKind { Continue, Confirm, Triple } and
                                  PauseResponse { Resumed, Approved, Denied,
                                  Interrupted, AllowOnce, AllowSession }
codelet/tools/src/request_user_input.rs — HitlOption, HitlQuestion, HitlRequest
                                          (wrapping Vec<HitlQuestion>),
                                          HitlResponse { Answered, Cancelled }
```

These are session-loop internal shapes and are intentionally NOT the
shapes RPC-036 lifts. The wire types under `codelet-rpc-types` are the
AgentView-facing slice (Confirm/Triple for `PauseKind`,
Resume/ConfirmAccept/ConfirmDeny/TripleApprove/TripleApproveSession/
TripleDeny for `PauseResponse`, single-question shape for
`HitlRequest`). The mapping between the two type families belongs to
Phase 4 (`codelet-sessions`), not this card.

## 6. Existing test infrastructure

```
codelet/rpc-types/src/lib.rs — no #[test] blocks found
codelet/rpc-types/Cargo.toml — no [dev-dependencies] section yet
```

RPC-036 establishes the first test suite for `codelet-rpc-types`. We
add `serde_json = { workspace = true }` and `serde = { workspace = true }`
under a new `[dev-dependencies]` section and place tests in a
`#[cfg(test)] mod tests` block at the bottom of `lib.rs`.

## 7. Conclusion

The work is entirely additive on the type system surface plus a single
additive field on `StreamChunk::IsolationStateChange` and an update to
the matching destructuring at `napi/src/types.rs:332`. Two production
constructor call sites remain compatible because the constructor
signature is preserved. The widening can be merged without touching
`SessionManagerHandle`, `FspecService`, or the AgentView — those are
RPC-037 and RPC-045 respectively.
