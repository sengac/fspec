# Epic Review: RPC-386 — AgentManager handler binds to the wrong SessionManager

**Date:** 2026-06-29
**Reviewer:** Claude Code (fspec review skill, `@spec/skills/review-skill.md`)
**Work Units Reviewed:** 1 (RPC-386 — no children)

## Summary
- 🔴 Critical: 0
- 🟡 Warnings: 2 (both addressed)
- 🟢 Observations: 4 (benign / pre-existing — no action)

Overall review status: **PASS** (with minor warnings, all now fixed).

---

## Work Unit Results

### RPC-386: AgentManager handler binds to global SessionManager singleton — PASS

The dependency-injection fix is correct and well-wired end-to-end:
- `BackgroundSession` carries an owning-manager `Weak<SessionManager>` back-reference, stamped by
  `create_session_with_id` / `create_isolated_session_with_id` from a `self_weak` the manager
  populates via `init_self_weak` (idempotent `OnceLock::set`).
- `create_handler` / `create_async_handler` resolve the owning manager
  (`match owning_manager { Some(m) => m.as_ref(), None => instance() }`) — daemon path uses the
  injected manager, NAPI path falls back to the singleton.
- Registration sites (`bridges.rs::register_agent_manager_handler`, `agent_loop.rs`) pass
  `session.owning_manager()` through.
- `build_service` calls `init_self_weak()` on the daemon-owned manager.
- No `codelet-napi` dependency introduced (verified by `no_napi_dependency` tests + grep).
- The self-weak stamp happens BEFORE `spawn_agent_loop`, so the handler the loop registers sees the
  back-reference — race-free.

#### 🔴 Critical Issues
None.

#### 🟡 Warnings (both fixed)
1. **Rule [3] over-broad wording + missing scenario coverage for `set_role` / `message` / `profile`.**
   The original Rule [3] claimed *all* actions including `profile` "resolve subordinates on the
   owning manager". Review found:
   - `set_role` and `message` are correctly wired through the resolved owning manager
     (`agent_manager_handler.rs` dispatch arms, Message 73–83 → `handle_message` 551–606;
     SetRole 84–89 → `handle_set_role` 506–537) but had **no acceptance scenario**.
   - `profile` is **manager-agnostic** — `handle_profile` profiles the runtime and resolves no
     session, so including it in the "resolves on owning manager" rule was inaccurate.
   **→ Fixed:**
   - Narrowed Rule [3] to the session-resolving actions and explicitly carved out `profile` as
     manager-agnostic / out of scope.
   - Added a 7th scenario **"set_role and message resolve the subordinate on the owning manager"**
     + a new test `set_role_and_message_resolve_subordinate_on_owning_manager`
     (`rpc386_owning_session_manager.rs:566-717`, `#[cfg(feature = "test-support")]`). The test
     asserts the role is applied to the subordinate **in M** (`M.get_session(sub).get_role()`),
     that `instance()` does NOT contain it, and that a message routed to the subordinate **in M**
     is delivered and processed (canned chunk emitted). Proven non-vacuous via red-then-green
     (temporarily routing to `instance()` made it fail with `session_not_found`).
   - Confirmed **no production change** was required — both arms already resolved on the injected
     manager.

2. **Coverage line-range imprecision for two scenarios.**
   - "Spawn fires the owning manager's session_created broadcast" was linked to the owning-manager
     stamp (`session_manager.rs:614-619`) rather than the broadcast emission.
   - "A subordinate spawned into a manager with real hooks runs its agent loop" was linked to the
     `background_session.rs` accessors rather than the spawn-loop path.
   **→ Fixed:** re-linked to the precise behaviour sites in `create_session_with_id`:
   - broadcast → `session_manager.rs:709` (`self.session_created_tx.send(created_info)`)
   - runs-agent-loop → `session_manager.rs:693-694` (`self.hooks().spawn_agent_loop(...)`)

#### 🟢 Observations (no action)
1. File sizes exceed the 300-line guideline (`agent_manager_handler.rs` 1353, `background_session.rs`
   1345, `session_manager.rs` 1123, `common.rs` 829, test 717) — all pre-existing; RPC-386's
   additions are small/localized.
2. `SessionManager::new_arc()` is provided but `build_service` uses `Arc::new(new()) + init_self_weak()`;
   both correct and idempotent. Marginal DRY nit.
3. Self-weak wiring confirmed race-free (`OnceLock::set`, stamp before `spawn_agent_loop`).
4. NAPI fallback confirmed sound (singleton never sets self-weak → empty `Weak` → `None` → `instance()`).

---

## Fix Results

### RPC-386
- 🟡 W1 (Rule 3 over-broad + missing set_role/message coverage) → ✅ Fixed: narrowed Rule [3],
  added 7th scenario + passing test; confirmed profile is manager-agnostic; no production change needed.
- 🟡 W2 (coverage line-range imprecision) → ✅ Fixed: re-linked broadcast scenario to
  `session_manager.rs:709` and runs-agent-loop scenario to `session_manager.rs:693-694`.

## Final Verification
- RPC-386 tests: ✅ 5/5 (default) and 7/7 (`--features test-support`) pass.
- `no_napi_dependency` (agent-loop + sessions): ✅ pass — forbidden arrow not introduced.
- `codelet-sessions` lib (37) + `codelet-fspec` common tests: ✅ pass.
- `codelet-napi` builds: ✅ (singleton/NAPI parity preserved — separate handler copy untouched).
- `cargo fmt --all --check`: ✅ clean.
- `cargo clippy` (touched crates): ✅ clean for all RPC-386 files. (One pre-existing
  `MutexGuard held across await` warning lives in `sessions/tests/spawned_subordinate_session_registration_parity_rpc385.rs`
  — RPC-385's test file, out of RPC-386 scope.)
- Feature file valid; coverage audit 14/14 mappings valid; 7/7 scenarios fully covered.

## Pre-existing issues found (NOT caused by RPC-386)
- `agent-loop/tests/rpc084_streaming.rs`: 2 failing tests
  (`custom_provider_fallthrough_calls_run_agent_stream_with_images`,
  `openai_inlined_arm_calls_run_agent_stream_with_images`) — confirmed failing on clean HEAD via
  git stash. Unrelated to RPC-386 (provider image-streaming). Flagged for a separate card.
- Clippy `await_holding_lock` warning in the RPC-385 parity test file (above).
