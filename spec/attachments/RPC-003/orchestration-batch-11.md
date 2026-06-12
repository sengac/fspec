# Batch 11 Orchestration — 10 Event Storm `add-*` commands

Started: 2026-06-12 (UTC). Supervisor session: `c223c7ca-f14c-4ab2-9e6c-e4c8ded9b3bb`.
Cargo Serial Worker: `67c69245-cdb0-43ad-9a5f-e3e89c1875cf`.

## RESTORED 2026-06-12 — session interrupted and re-established

New supervisor session: `53f79e3e-8d2d-40ba-bbd5-3fc731cee930`.
New Cargo Serial Worker: `c4761c3c-7fd6-456a-9690-c3d91783719d`.

New worker session IDs (old → new, context restored from old history):

| Worker | OLD session_id | NEW session_id |
|--------|----------------|----------------|
| 1 | 7ba61446-87b2-40cb-b049-ca689a83a9a8 | 282ce254-2d0a-4dd8-8a57-0771613b5d7b |
| 2 | 414c24e3-dd7a-42b7-8695-3e60936c5897 | 40e55c41-0006-4f8f-b839-95ed1c978bd9 |
| 3 | 35c16ae8-d9f0-4783-b9fc-f5dec7e9cc40 | a45038b1-fed5-43d5-9e80-b5d3701c3aaa |
| 4 | bb6858f8-fbc8-4597-a9e1-64bf3dc221b9 | 97dc1cd1-5a15-479c-ba65-64e5260fbc70 |

Theme: Event Storm mutation commands. Workers 1–3 mutate `spec/work-units.json`
`eventStorm` sub-object (via WorkUnit `extra` map — no shared-type changes, like
`add-rule`). Worker 4 mutates `spec/foundation.json` `eventStorm`.

## Worker → command assignment (10 commands)

| Worker | session_id | RPC IDs | Commands | Target file |
|--------|-----------|---------|----------|-------------|
| 1 | 7ba61446-87b2-40cb-b049-ca689a83a9a8 | RPC-165, RPC-179 | add-aggregate, add-domain-event | work-units.json |
| 2 | 414c24e3-dd7a-42b7-8695-3e60936c5897 | RPC-172, RPC-185, RPC-174 | add-bounded-context, add-hotspot, add-command | work-units.json |
| 3 | 35c16ae8-d9f0-4783-b9fc-f5dec7e9cc40 | RPC-187, RPC-182 | add-policy, add-external-system | work-units.json |
| 4 | bb6858f8-fbc8-4597-a9e1-64bf3dc221b9 | RPC-180, RPC-183, RPC-175 | add-domain-event-to-foundation, add-foundation-bounded-context, add-command-to-foundation | foundation.json |

Already `specifying` at batch start: RPC-165, RPC-172. All others set to
`specifying` BY THE OWNING WORKER (per user constraint: status→specifying
moves happen inside the worker agents, not the supervisor).

## Phase tracking

| RPC | Cmd | Phase A | Phase B | Phase C | Validated | Done |
|-----|-----|---------|---------|---------|-----------|------|
| RPC-165 | add-aggregate | | | | | |
| RPC-179 | add-domain-event | | | | | |
| RPC-172 | add-bounded-context | | | | | |
| RPC-185 | add-hotspot | | | | | |
| RPC-174 | add-command | | | | | |
| RPC-187 | add-policy | | | | | |
| RPC-182 | add-external-system | | | | | |
| RPC-180 | add-domain-event-to-foundation | | | | | |
| RPC-183 | add-foundation-bounded-context | | | | | |
| RPC-175 | add-command-to-foundation | | | | | |

## Shared-file change requests (pending supervisor action)

| Requested by | File | Change | Status |
|--------------|------|--------|--------|
| | | | |

## Supervisor wiring checklist (Phase C, single edit pass)

- [ ] canonical.rs: add 10 lines to PORTED_COMMANDS
- [ ] dispatch.rs::run_ported: add 10 match arms; remove 10 from run_stub
- [ ] commands/mod.rs: register 10 modules (already declared as stubs — verify)
- [ ] help/configs/mod.rs: register 10 help configs
- [ ] main.rs: 10 Mode:: variants + 10 forward! arms + 10 intercept arms + 10 `mod <snake>;`
- [ ] cargo_shape.rs: bump main_cap + lock-list if needed
