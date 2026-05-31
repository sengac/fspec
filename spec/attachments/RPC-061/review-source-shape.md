# Review: RPC-061 (Source Shape) — Supervisor / subordinate links surface

## Status: PASS

## 🔴 Critical Issues (Must Fix)
None

## 🟡 Warnings (Should Fix)
1. `codelet/fspec-tui/tests/source_shape_rpc061.rs:4` — Module doc says `Feature: spec/features/rpc061-supervisor-links.feature`, but the tests in this file actually pin `spec/features/rpc061-source-shape.feature`. The companion behaviour feature is `rpc061-supervisor-links.feature`. Should be `Feature: spec/features/rpc061-source-shape.feature` to comply with the ACDD test-file header rule (test file header MUST cite the feature it validates).
2. `codelet/fspec-tui/tests/source_shape_rpc061.rs:79` and `:113` — Doc comments say "RPC-061 methods" / "the five RPC-061 methods" but the corresponding feature scenarios are "all five **supervisor** methods" (line 32, 40 of the feature). Pure doc-comment drift; harmless to the `// @step` matcher but reduces traceability.
3. `codelet/fspec-tui/tests/source_shape_rpc061.rs:249` — Doc comment "FspecBackend trait gains the five RPC-061 forwarders" vs feature scenario "FspecBackend trait gains the five **supervisor** forwarders" (line 48). Same drift class as #2.
4. `codelet/fspec-tui/tests/source_shape_rpc061.rs:215` — Doc comment "handle_impl.rs in codelet-sessions wires the supervisor methods" vs feature scenario "codelet-sessions handle_impl wires the five supervisor methods" (line 56). Cosmetic drift only.
5. `codelet/fspec-tui/tests/source_shape_rpc061.rs:147` — Doc comment "dispatch_rpc061.rs **file** has the documented helper surface" vs feature scenario "dispatch_rpc061.rs has the documented helper surface" (line 77). Minor.
6. `spec/features/rpc061-source-shape.feature:18` — Background labelled `User Story` consists of free-form `As a / I want / So that` lines as a single block without an explicit step keyword. Backgrounds normally hold Given steps; the current shape compiles (Gherkin allows description text under Background) but is non-idiomatic. Other source-shape features in this repo (e.g. rpc060) use this same convention so it is consistent but technically a deviation from canonical Gherkin.

## 🟢 Observations (Nice to Have)
1. `codelet/fspec-tui/tests/source_shape_rpc061.rs:67-74` — The derive assertion has 3-way `||` fallback (two orderings plus a substring-AND fallback). This is robust against trait-derive reordering. Good defensive coding.
2. `codelet/fspec-tui/src/app/dispatch_rpc061.rs:50, 77` — `if tokio::runtime::Handle::try_current().is_err() { return; }` is the consistent guard used across the dispatch_rpc05X helpers (good DRY w/ surrounding modules). Consider extracting to a shared helper in a follow-up; not blocking.
3. `codelet/fspec-tui/src/app/dispatch.rs:295` — Style nit: missing newline between `self.navigator.apply_action(&action);` and `let _ = self.compositor.update(action);` — two statements on the same source line. Pre-existing, not RPC-061 surface.
4. `codelet/fspec-tui/src/views/agent/footer.rs:76` — `.unwrap_or_else(|| Line::from(...))` is `Option::unwrap_or_else`, which is safe (not the panic-`unwrap()` pattern). Flagged only because `unwrap` substring search hits it; no action needed.
5. `codelet/fspec-tui/src/app/dispatch_rpc061.rs:62, 84` — `let _ = action_tx.send(...)` explicitly discards the `Result`. Consistent with surrounding dispatch modules; acceptable, though tracing the error would help debug action-bus shutdown races.
6. `codelet/fspec-tui/tests/source_shape_rpc061.rs:26` — `panic!("read {}", path.display())` is wrapped under `#![allow(clippy::panic)]` at line 11. Test-only, acceptable.
7. Architecture note `[11]` says dispatch_rpc061 helps keep `app/dispatch.rs` under 300 LoC. `dispatch.rs` is at 298 lines — within the limit but only by 2 lines. Very tight; future RPC-06X additions risk breaching this if they add to `dispatch.rs` instead of a new helper module.

## Coverage Verification
- Feature file: `spec/features/rpc061-source-shape.feature` — OK (10 scenarios, all Given/Then/And, no placeholders, `@RPC-061` present, architecture docstring present at lines 5-16, `@done` tag present)
- Test file: `codelet/fspec-tui/tests/source_shape_rpc061.rs` — OK (every scenario covered; `// @step` text matches feature step text exactly; tests assert on real source content from disk via `fs::read_to_string`, not trivial)
- Impl files: 10 listed — OK (every source-shape claim was verified at the pinned line ranges; structure declarations and trait signatures all present at the claimed coordinates)
- Scenario coverage: 10/10 scenarios covered (verified by `fspec show-coverage`)

## File-size Audit (300 LoC limit)
```
     298 codelet/fspec-tui/src/app/dispatch.rs               PASS (tight: 2 LoC under)
     107 codelet/fspec-tui/src/app/dispatch_rpc061.rs        PASS
     754 codelet/fspec-tui/src/transport/embedded.rs         N/A (not bound by 300 limit — pre-existing aggregate)
    1279 codelet/fspec-tui/src/transport/websocket.rs        N/A (pre-existing)
    2240 codelet/core/src/session_manager_handle.rs          N/A (pre-existing trait+stub bundle)
    1337 codelet/sessions/src/handle_impl.rs                 N/A (pre-existing aggregate)
     178 codelet/fspec-tui/src/views/agent/header.rs         PASS
     264 codelet/fspec-tui/src/views/agent/footer.rs         PASS
     784 codelet/fspec-tui/src/components/mod.rs             N/A (Action enum aggregate)
    1560 codelet/rpc-types/src/lib.rs                        N/A (wire types aggregate)
    1727 codelet/rpc/src/lib.rs                              N/A (FspecService aggregate)
     674 codelet/fspec-tui/src/transport/mod.rs              N/A (FspecBackend trait aggregate)
```
The 300-LoC ceiling is enforced by the feature's own scenarios only for `dispatch_rpc061.rs` and `app/dispatch.rs` (both PASS). The pre-existing aggregate files (transport, rpc, rpc-types, core, sessions, components) are outside RPC-061's scope — flagged out-of-band only.

## Build/Test Output
```
cargo test -p codelet-fspec-tui --test source_shape_rpc061
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.38s
     Running tests/source_shape_rpc061.rs (target/debug/deps/source_shape_rpc061-7874269f1a3dc890)

running 10 tests
test session_footer_gains_supervisor_pending_count_field ... ok
test app_dispatch_catchall_routes_through_rpc061 ... ok
test dispatch_rpc061_file_has_expected_shape ... ok
test session_header_gains_subordinate_label_field ... ok
test action_enum_gains_rpc061_variants ... ok
test fspec_backend_trait_gains_supervisor_forwarders ... ok
test fspec_service_declares_supervisor_methods ... ok
test sessions_handle_impl_wires_supervisor_methods ... ok
test rpc_types_declares_incoming_message_input ... ok
test session_manager_handle_declares_supervisor_methods ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
EXIT=0

cargo build -p codelet-fspec-tui
[…compile lines…]
   Compiling codelet-fspec-tui v0.1.0 (/Users/rquast/projects/fspec/codelet/fspec-tui)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 38.25s
EXIT=0
```
Clean build, all 10 source-shape tests pass.

## Files Reviewed
- spec/features/rpc061-source-shape.feature
- codelet/fspec-tui/tests/source_shape_rpc061.rs
- codelet/rpc-types/src/lib.rs (lines 700-725)
- codelet/core/src/session_manager_handle.rs (lines 380-440)
- codelet/rpc/src/lib.rs (lines 280-330)
- codelet/fspec-tui/src/transport/mod.rs (lines 385-445)
- codelet/sessions/src/handle_impl.rs (lines 340-415)
- codelet/fspec-tui/src/components/mod.rs (lines 720-760)
- codelet/fspec-tui/src/views/agent/header.rs (lines 70-110)
- codelet/fspec-tui/src/views/agent/footer.rs (lines 45-95)
- codelet/fspec-tui/src/app/dispatch_rpc061.rs (full file, 107 lines)
- codelet/fspec-tui/src/app/dispatch.rs (lines 280-298)
- fspec show-work-unit RPC-061 (work unit JSON, rules + architecture notes)
- fspec show-coverage rpc061-source-shape (all 10 scenarios)
