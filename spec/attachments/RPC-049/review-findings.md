# Review: RPC-049 — /resume durable restore via restore_session_messages + restore_session_token_state

**Date:** 2026-05-22
**Reviewer:** Claude Code (fspec review skill)
**Work Unit:** RPC-049 (story, parent RPC-030, depends on RPC-048)

## Summary

| Severity      | Count | Status     |
|---------------|------:|------------|
| 🔴 Critical   |     1 | ✅ Fixed   |
| 🟡 Warnings   |     2 | ✅ Fixed   |
| 🟢 Observations |   1 | ✅ Fixed   |

## Status: ✅ PASS (after fixes)

---

## 🔴 Critical Issues (Must Fix) — FIXED

### 1. `@step` text mismatch in cross-transport parity test

**File:** `codelet/fspec-tui/tests/rpc049_cross_transport_parity.rs:92`

The Gherkin step in `slash-command-resume-cross-transport-parity.feature:28` reads:

```
And the StubSessionManagerHandle's resume_session call counter increments by 2 (once per transport)
```

But the test comment was:

```
// @step And the StubSessionManagerHandle's resume_session call counter is 2
```

This violates the ACDD rule that `@step` text MUST exactly match the feature file step.

**Fix:** Rewrote the `@step` comment to match the feature file step verbatim.

---

## 🟡 Warnings (Should Fix) — FIXED

### 2. Source-shape scenario over-claims the 300-LoC enforcement scope

**File:** `spec/features/slash-command-resume-source-shape.feature` (scenario 1)
**Related test:** `codelet/fspec-tui/tests/source_shape_rpc049.rs:114-127`

The Gherkin step asserted:

```
And every file under codelet/fspec-tui/src/ is strictly less than 300 lines of code
```

But the test only checks the three RPC-024/025/026 hot-path directories:
`src/app/`, `src/views/agent/`, `src/store/agent_view/`. The broader claim is
actually false — `transport/websocket.rs` (980 LoC), `transport/embedded.rs`
(530 LoC), `transport/mod.rs` (479 LoC), `components/mod.rs` (446 LoC) and
`compositor_tests.rs` (402 LoC) all exceed the ceiling. Rule [7] of the
example map narrows the contract to "no NEW file…exceeds 300 LoC" plus the
`dispatch.rs` invariant, so the feature file was the offender — not the test.

**Fix:** Narrowed the Gherkin step (and its matching `@step` comment) to name
the three hot-path directories explicitly, and expanded the doc-string to
document why historical infrastructure files are out of scope.

### 3. `@wip` tags lingered on three feature files

**Files:**
- `spec/features/slash-command-resume-source-shape.feature`
- `spec/features/slash-command-resume-cross-transport-parity.feature`
- `spec/features/slash-command-resume-persistence-lift.feature`

Each feature carried both `@done` and `@wip` tags simultaneously, indicating
the `@wip → @done` tag swap was not completed at the original "done"
transition.

**Fix:** Removed `@wip` from the three feature files; `@done` remains.

---

## 🟢 Observations (Nice to Have) — FIXED

### 4. Misleading wildcard-arm comment in `dispatch.rs`

**File:** `codelet/fspec-tui/src/app/dispatch.rs:292`

The comment read:

```
// RPC-022 + RPC-049 dispatch arms route through try_dispatch_rpc022 so this file stays < 300 LoC.
```

But the RPC-049 `Action::SessionResumeComplete(id)` arm is wired directly on
line 291 — only the RPC-022 arms are forwarded via `try_dispatch_rpc022`.
The comment confused future readers into thinking the variant was routed
elsewhere.

**Fix:** Tightened the comment to read "Remaining RPC-022 dispatch arms route
through try_dispatch_rpc022 so this file stays < 300 LoC."

---

## Coverage Verification

| Feature file | Coverage | Scenarios |
|--------------|----------|-----------|
| `spec/features/slash-command-resume.feature` | ✅ 100% | 4/4 |
| `spec/features/slash-command-resume-cross-transport-parity.feature` | ✅ 100% | 1/1 |
| `spec/features/slash-command-resume-persistence-lift.feature` | ✅ 100% | 3/3 |
| `spec/features/slash-command-resume-source-shape.feature` | ✅ 100% | 3/3 |

All scenarios link to a test file and an implementation file with line ranges
that point at the live code paths.

---

## Implementation Quality Checks

**Trait surface (codelet-core)**
- `SessionManagerHandle::resume_session` default impl orchestrates
  `Uuid::parse_str → load_session → get_session_message_envelopes →
  build TokenRestoreState → restore_session_messages →
  restore_session_token_state`. Each step propagates `Result<(), String>`.
  Matches architecture note [0] exactly.
- `StubSessionManagerHandle::resume_session` overrides the default to bump an
  `AtomicU64` call counter (used by parity tests) and short-circuits to
  `Ok(())`.

**Persistence lift (codelet-core)**
- `codelet_core::persistence::manifest::get_session_message_envelopes` lifted
  from the NAPI binding with `String` errors (not `napi::Error`). Synthetic
  compaction-summary envelopes and blob-rehydration semantics preserved.
- `codelet/napi/src/persistence/napi_bindings.rs::persistence_get_session_message_envelopes`
  reduced to a 2-line delegate that parses the UUID and forwards to
  `codelet_core::persistence::get_session_message_envelopes`.

**RPC surface (codelet-rpc)**
- `FspecService::resume_session` trait method added (line 250).
- `FspecServiceImpl::resume_session` (line 1052) delegates through
  `session_manager()?.resume_session(&session_id)`, returning `Ok(())` when
  no handle is wired.
- `EmbeddedFspecBackend::resume_session` and
  `WebSocketFspecBackend::resume_session` are one-line client-side delegates
  matching the `clear_history` / `compact_session` shape.

**Dispatch wiring (codelet-fspec-tui)**
- `Action::SessionResumeComplete(SessionId)` variant added to
  `components/mod.rs` with descriptive rustdoc tying it to the round-trip.
- `handle_attach_to_session` (dispatch_rpc026.rs) preserves RPC-026
  focus-move / append semantics, then spawns a tokio task that awaits
  `backend.resume_session(session_id)`. Ok routes to
  `Action::SessionResumeComplete`; Err routes to
  `Action::EmitSessionNotice(id, "[error] /resume failed: {e}")`. Includes a
  defensive `Handle::try_current().is_err()` guard so synchronous unit-test
  callers can still observe the open_sessions move/append without a
  panic-from-no-runtime.
- `handle_session_resume_complete` spawns a second task that calls
  `backend.get_buffered_output(id, 1000)` and replays each chunk into the
  action bus as `Action::ChunkReceived(id, chunk)`. Uses
  `unwrap_or_default()` (no `unwrap()` violation).
- `dispatch.rs` adds one match arm:
  `Action::SessionResumeComplete(id) => self.handle_session_resume_complete(id.clone())`.

**Source-shape guarantees**
- `codelet/fspec-tui/src/app/dispatch_rpc026.rs` — 218 LoC (< 300).
- `codelet/fspec-tui/src/app/dispatch.rs` — 299 LoC (< 300).
- No `codelet_napi` references in `codelet/fspec-tui/src/` (RPC-002
  invariant).

**MockBackend (test fixture)**
- `resume_session_calls()` + `last_resume_session()` accessors.
- `set_resume_session_error(message)` scripts the Err branch.
- `set_buffered_output(chunks)` scripts the replay set.
- `async fn resume_session` impl bumps the call counter, captures the
  argument, and honours the scripted error.

---

## Build & Test Verification

```
$ cargo test -p codelet-fspec-tui --test slash_resume_rpc049 \
                                  --test rpc049_cross_transport_parity \
                                  --test source_shape_rpc049
... 5 + 1 + 3 = 9 tests ... ok

$ cargo test -p codelet-core --lib persistence::tests::rpc049
... 3 tests ... ok

$ fspec validate (RPC-049 feature files)
✓ slash-command-resume.feature is valid
✓ slash-command-resume-cross-transport-parity.feature is valid
✓ slash-command-resume-persistence-lift.feature is valid
✓ slash-command-resume-source-shape.feature is valid
```

Total RPC-049 test count: **12 passing, 0 failing**.

---

## Files Reviewed

- `spec/features/slash-command-resume.feature`
- `spec/features/slash-command-resume-cross-transport-parity.feature`
- `spec/features/slash-command-resume-persistence-lift.feature`
- `spec/features/slash-command-resume-source-shape.feature`
- `codelet/fspec-tui/tests/slash_resume_rpc049.rs`
- `codelet/fspec-tui/tests/rpc049_cross_transport_parity.rs`
- `codelet/fspec-tui/tests/source_shape_rpc049.rs`
- `codelet/fspec-tui/tests/common/mod.rs` (MockBackend section)
- `codelet/core/src/persistence/tests.rs` (RPC-049 tests, lines 2280–2395)
- `codelet/core/src/persistence/manifest.rs` (lines 970–1018)
- `codelet/core/src/session_manager_handle.rs` (default impl + stub override)
- `codelet/napi/src/persistence/napi_bindings.rs` (thin delegate at 726–736)
- `codelet/rpc/src/lib.rs` (resume_session trait + impl arms)
- `codelet/fspec-tui/src/app/dispatch.rs`
- `codelet/fspec-tui/src/app/dispatch_rpc026.rs`
- `codelet/fspec-tui/src/components/mod.rs` (Action enum section)
- `codelet/fspec-tui/src/transport/mod.rs` (resume_session trait method)
- `codelet/fspec-tui/src/transport/embedded.rs` (resume_session delegate)
- `codelet/fspec-tui/src/transport/websocket.rs` (resume_session delegate)

---

## Fix Results

### RPC-049

- 🔴 Issue 1: `@step` text mismatch in `rpc049_cross_transport_parity.rs` → ✅ Fixed: replaced "is 2" with the verbatim "increments by 2 (once per transport)" wording.
- 🟡 Issue 2: source-shape scenario over-claimed the 300-LoC scope → ✅ Fixed: narrowed feature step + matching `@step` comment to enumerate the three hot-path directories actually enforced.
- 🟡 Issue 3: `@wip` lingered on 3 feature files → ✅ Fixed: removed `@wip` from the source-shape, cross-transport-parity, and persistence-lift feature files.
- 🟢 Issue 4: misleading wildcard-arm comment in `dispatch.rs` → ✅ Fixed: clarified that only the remaining RPC-022 arms route through `try_dispatch_rpc022`.

## Final Verification

- All RPC-049 tests pass: ✅
- Build succeeds: ✅
- Coverage complete (all four feature files at 100%): ✅
- Feature files valid: ✅
- `@step` comments match feature file steps verbatim: ✅
- Source-shape ceiling holds: ✅
