# RPC-068 — Final TS-frontend regression + boundary audit

**Parent:** RPC-030 · **Phase:** 8.4-8.5 · **Estimate:** 3 pts · **Depends on:** RPC-067

## Goal

The final card. Two checklists:

1. **TS-frontend regression**: Run the full TS test suite. Every TS-facing function must produce identical behaviour. Since `codelet-napi` is now a thin adapter over `codelet-sessions`, any TS regression means the Phase 4 extraction broke something.

2. **Boundary audit**: Confirm the architectural invariants hold by direct inspection / grep.

## TS regression

Run:
```
npm test              # full vitest run
npm run test:integration
npm run test:e2e      # if exists
```

Compare against the pre-RPC-030 baseline (commit before RPC-031 landed). Every test must pass.

Specific TS test areas to spot-check:
- `src/tui/__tests__/AgentView.test.tsx` — full AgentView integration
- `src/tui/__tests__/session-store.test.ts` — session manager interactions
- `src/llm/__tests__/` — provider, model, thinking-config
- `src/persistence/__tests__/` — envelope round-trip, blob storage, message-index
- `src/__tests__/integration/` — anything end-to-end

If a test fails:
- Diff `codelet/napi/index.d.ts` against baseline — should be empty.
- Check the TS test against the specific change in `codelet-sessions`.
- Likely culprit: a string-error format changed (was `napi::Error`, now `String`). Fix the format string in the `#[napi]` adapter.

## Boundary audit checklist

### Grep for forbidden imports

Run from project root:

```bash
# Should produce zero output
rg "use codelet_napi" codelet/core codelet/rpc codelet/rpc-types \
  codelet/rpc-embedded codelet/rpc-server codelet/fspec \
  codelet/fspec-tui codelet/sessions

# Should produce zero output
rg "codelet-napi" codelet/{core,rpc,rpc-types,rpc-embedded,rpc-server,fspec,fspec-tui,sessions}/Cargo.toml
```

### Confirm deleted artefacts

```bash
# Should not exist
test ! -f codelet/napi/src/session_manager.rs

# Should exist
test -f codelet/napi/src/session_bindings.rs
test -f codelet/sessions/src/background_session.rs
test -f codelet/sessions/src/session_manager.rs

# Should contain ONLY these files
ls codelet/napi/src/persistence/
# Expected: mod.rs, napi_bindings.rs (+ optional tests.rs, lazy_init_tests.rs)
```

### Confirm GLOBAL_CHUNK_CALLBACK is gone

```bash
# Should produce zero output
rg "GLOBAL_CHUNK_CALLBACK" codelet/
rg "GlobalChunkCallback" codelet/
rg "unsafe impl Send for GlobalChunkCallback" codelet/
```

### Confirm tokio::broadcast is wired

```bash
rg "broadcast::Sender<\(SessionId, StreamChunk\)>" codelet/sessions/src/
# Expected: present in session_manager.rs and background_session.rs
```

### Run all dependency-rule tests

```bash
cargo test --workspace --test no_napi_dependency
# All pass
```

## Final verification matrix

| Item | Expected state |
|---|---|
| `codelet/napi/src/session_manager.rs` | deleted |
| `codelet/napi/src/session_bindings.rs` | exists, ≤ 1000 LOC |
| `codelet/napi/src/persistence/` contents | `mod.rs`, `napi_bindings.rs` (+ optional tests) |
| `codelet/sessions/src/lib.rs` | exists, declares `background_session` + `session_manager` modules |
| `codelet/sessions/src/background_session.rs` | exists, contains `BackgroundSession` |
| `codelet/sessions/src/session_manager.rs` | exists, contains `SessionManager` |
| `codelet/core/src/persistence/` | contains `message_envelope`, `messages`, `manifest`, `blob`, `blob_processing`, `history` |
| `GLOBAL_CHUNK_CALLBACK` | grep returns zero |
| `unsafe impl Send/Sync for GlobalChunkCallback` | grep returns zero |
| `rpc → napi` | dependency rule test passes |
| `fspec → napi` | dependency rule test passes |
| `fspec-tui → napi` | dependency rule test passes |
| `sessions → napi` | dependency rule test passes |
| `core → napi` | dependency rule test passes |
| `rpc-types → napi` | dependency rule test passes |
| TS test suite | all pass |
| Cross-frontend integration test (RPC-066) | passes |
| Behaviour-parity test suite (RPC-065) | passes |
| `codelet/napi/index.d.ts` | byte-identical to pre-RPC-030 baseline |

## Acceptance criteria

1. All items in the verification matrix above are confirmed.
2. `npm test` passes with no regressions vs pre-RPC-030.
3. `cargo test --workspace` passes.
4. A markdown report summarising the audit results is committed at `spec/attachments/RPC-068/boundary-audit-report.md` so future agents can verify the invariants.

## What "done" looks like

When this card completes:

- The Rust ratatui AgentView has 100% TS-Ink-AgentView parity.
- `codelet-napi` is a thin adapter (~500–1000 LOC) over `codelet-sessions`.
- The `fspec` binary runs real agent sessions with zero NAPI dependency.
- Every architectural invariant is test-asserted.

**This card completes RPC-030.**

## Out of scope

- Marketing announcements / PR posts.
- Performance benchmarking (separate concern).
