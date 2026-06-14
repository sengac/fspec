# Batch 15 Orchestration — Feature-File Mutation Commands (TS → Rust)

Supervisor session: `ba983ebf-beda-49c7-8042-bfcda65dfc68`

## Selected 10 commands (all feature-file `.feature` mutation, line-based editing)

| RPC ID  | Command          | snake_case        | Worker |
|---------|------------------|-------------------|--------|
| RPC-212 | create-feature   | create_feature    | W1 |
| RPC-190 | add-scenario     | add_scenario      | W1 |
| RPC-192 | add-step         | add_step          | W1 |
| RPC-171 | add-background   | add_background    | W2 |
| RPC-167 | add-architecture | add_architecture  | W2 |
| RPC-219 | delete-scenario  | delete_scenario   | W3 |
| RPC-221 | delete-step      | delete_step       | W3 |
| RPC-218 | delete-features  | delete_features   | W3 |
| RPC-314 | update-scenario  | update_scenario   | W4 |
| RPC-315 | update-step      | update_step       | W4 |

## Roles (5 agents total — 1 reserved for cargo per user instruction)

| Slot | Role          | session_id | Commands |
|------|---------------|------------|----------|
| CARGO | Cargo Serial Worker | 1e1ccd12-8e79-4851-9280-bab2c2400789 | all cargo/build/binary runs |
| W1   | Worker        | 4beec135-ec57-4eb5-a792-9cd7feb8093c  | RPC-212, RPC-190, RPC-192 |
| W2   | Worker        | d4cb65bd-192d-47be-bb6f-08fec7f86dde  | RPC-171, RPC-167 |
| W3   | Worker        | 71b22d11-daf2-49c1-8251-5e547ff6b34a  | RPC-219, RPC-221, RPC-218 |
| W4   | Worker        | 699efb4b-44c9-4dec-90d9-06fa3387cd48  | RPC-314, RPC-315 |

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
|              |      |        |        |

## Phase status

- [x] Phase A — Specifying (W1: 34 scen across 3 cmds; W2: 28 scen/2; W3: 27 scen/3; W4: 25 scen/2. All validate, @wip tagged. No new io/ helpers needed.)
- [x] Phase B — Testing (tests fail with NotYetPorted). W1: 19 core+15 cli; W2: 16 core+12 cli; W3: 14 core+13 cli; W4: 14 core+11 cli. All red-confirmed via cargo runner. Help fixtures captured.
- [x] Phase C — Implementing (all 4 workers wrote impl+help+bridge for 10 cmds; 2-arg signatures confirmed)
- [x] Wiring (canonical/dispatch/main/help-configs/mod + cargo_shape allowed/count/main_cap) — supervisor done
- [x] Validating — 2 builds clean; 10 core binaries green (60 tests); 10 CLI binaries green (50 tests); cargo_shape 11p/11ign; cross_frontend_parity 8/8. 4 initial assertion-shape failures fixed by owning workers (TS-parity).
- [x] Done — all 10 work units DONE. Final regression: 133 passed / 0 failed across 22 targeted binaries (63 core + 70 CLI+guards). cargo_shape 11p/11ign, cross_frontend_parity 8/8.
- [ ] Wiring (canonical/dispatch/main/help-configs/mod)
- [ ] Validating
- [ ] Done

## Estimates
RPC-212=5 RPC-190=3 RPC-192=5 | RPC-171=3 RPC-167=3 | RPC-219=3 RPC-221=2 RPC-218=3 | RPC-314=3 RPC-315=3

## Design decisions
- create-feature (RPC-212): mirror the TS rich result shape (CreateFeatureResult) as observed from `node dist/index.js create-feature` non-TTY output — TS output is canon.
- delete-features (RPC-218): DirectoryNotFound → empty list; preserve TS glob ordering.
- delete-step (RPC-221): mirror runtime text-match behaviour; keep help fixture byte-exact even if it mentions an index.

## PHASE C wiring notes (supervisor TODO)
- delete-features (RPC-218): TS `--help` is PLAIN Commander.js default format (no custom help module), like list-foundation-sections. Hard-code its help string in main.rs and special-case the intercept arm.
- add-architecture (RPC-167): minimal formatter (627B). update-step (RPC-315): help has quirky `undefined` lines — reproduce exactly.
- Stub→impl signature change: remove 1-arg run_stub arms + add 2-arg run_ported arms for all 10 commands in ONE pass (core lib won't compile until then).
