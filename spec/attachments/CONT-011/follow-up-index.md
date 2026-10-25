# CONT-011: Auto-continue audit follow-up index

Audit baseline: `f0eb0759`, 2026-10-25. This card coordinates fixes, not a
claim that a single defect explains every reported stoppage.

The confidence labels below describe the original discovery, not final repair
status. Final implementation, real provider/disk tests, retained limits and
native closure evidence are in [validation and review](validation-and-review.md).
CONT-010 stays read-only backlog; Done PROV-161's lifecycle/reload contract
supersedes the older creation-only assumptions without changing this batch scope.

## Cards

| Card | Scope | Evidence |
| --- | --- | --- |
| BUG-260 | Codex refusal-history follow-up | Reproduced against compiled production history API |
| BUG-261 | Codex forced-refresh network error identity | Source-confirmed; dedicated reproducer still needed |
| BUG-262 | Chat tool ordering before Done | Source-confirmed; longstanding |
| BUG-263 | Chat terminal evidence and accepted completion | Source-confirmed composition gaps |
| BUG-264 | Generic Responses failure/incomplete semantics | Source-confirmed latent adapter issue; not dedicated Codex |
| BUG-265 | Recovery resets nudges/goal rejections | Source-confirmed; pre-existing |
| BUG-266 | Rhai stop, trailing usage and HTTP settlement | Source-confirmed; existing error contract needs reconciliation |
| BUG-267 | Built-in-name script shadowing dispatch | Source-confirmed selection/dispatch mismatch |
| BUG-268 | Copilot actual endpoint routing | Source-confirmed; longstanding |
| BUG-269 | Scripted OAuth automatic refresh integration | Source-confirmed missing runtime composition |
| PROV-162 | Codex typed protocol replay policy | Discovery: current text-only policy is explicitly specified |
| CONT-010 | Profile lifecycle defaults/spec reconciliation | Existing backlog card; PROV-161 supersedes creation-only claims |

The ten BUG cards are children of CONT-011. PROV-162 and CONT-010 are related
discovery work, not automatically blocking dependencies.

## Integration work owned here

- Fix stale source-layout assertions in
  `rust/cli/tests/auto_continue_engine_test.rs:381` and
  `rust/cli/tests/cont006_goal_immediate_termination.rs:580`.
- Do not mistake those assertion failures for missing production registration
  or teardown: the implementation moved to Codex `agent.rs` and session `reminders.rs`.
- Add real serialized provider-stream tests for armed stop -> nudge -> accepted
  Done, goal rejection, tool ordering, terminal failure and recovery accounting.
- Exercise native Claude/OAuth, Chat, dedicated Codex, Gemini, ZAI, Copilot,
  custom/Rhai and the generic Responses adapter at their actual boundaries.
- Assert completed side effects, native tool acceptance, persisted history,
  terminal event count, request usage and cancellation; string searches alone
  do not establish behavioral coverage.

## Constraints and sequence

Start with the reproduced BUG-260, then high-impact ordering, terminal and
recovery fixes. Each implementation follows feature -> failing test -> code.
Preserve immediate Done termination, safety/refusal behavior, trusted attribution,
generation/lineage validation, cancellation and bounded recovery. Do not drain
trailing executable tools or convert provider failures into clean completion.

See [full audit](fspec-auto-continue-investigation.md) and
[validation evidence](validation-evidence.md). Each child has focused Markdown
research; findings are not all claimed to be new October regressions.
