# Final compaction completion coordination

Scope: the 27 CMPCT cards active when this completion pass began on 2026-10-18.
Existing staged and unstaged implementation is retained. No existing completion
attachment is treated as proof that a current test passes.

## Exclusive write ownership

| Owner | Files | Primary cards |
| --- | --- | --- |
| Core | `rust/core/**` | 055, 064, 070, 072 |
| Providers | `rust/providers/**` | 065, 069, 071, 074, 075, 078, 080, 081, 082 |
| CLI | `rust/cli/**` | 052, 057, 058, 059, 071, 079, 082 |
| Loop/tools | `rust/agent-loop/**`, `rust/tools/**` | 052, 054, 056, 068, 069 |
| Sessions/UI/RPC | `rust/sessions/**`, `rust/fspec-tui/**`, `rust/rpc-types/**`, excluding gate tests | 060, 061, 062, 067, 068 |
| Rig | `rust/patches/rig-core/**` | 065, 066, 071 builtin serializers |
| Gate | `rust/sessions/tests/cmpct083*` | 083 |
| Coordinator | Board updates, cross-owner coordination, final evidence | All active cards |

Each code owner also owns its target-specific features and coverage sidecars.
An independent read-only auditor checks the full active-card scenario inventory.
Cross-owner fixes must be requested from the owning agent, not written over it.

## Completion requirements

1. Read the retained shared no-fallback contract and current acceptance criteria.
2. For unfinished behavior, specify first and execute a genuinely failing test
   before production changes. Preserve existing RED evidence without inventing
   historical test-first provenance.
3. Run scoped Cargo tests using the `ci-test` profile with bounded parallelism;
   retain command logs. Never run unscoped workspace tests.
4. Verify complete scenario-to-test-to-implementation coverage, actual matching
   test functions and step comments, without coverage-validation bypasses.
5. Exercise production provider checkpoint persistence in the cold-process gate.
   A real selected adapter without persistence binding is insufficient evidence.
6. Advance each board card through its remaining states only after its owner and
   independent checks substantiate completion. Preserve dependency order.

## Initial validation

- `fspec validate`: all 1941 feature files parse successfully.
- `fspec validate-tags`: 526 repository-wide violations and 1415 passing files.
  This is a failing baseline, not a successful quality check. Changed compaction
  features require a separately verified scoped tag result; unrelated features
  will not be rewritten to disguise the baseline.
- The CLI does not support bootstrap's suggested `--sync-version` option.
  `fspec bootstrap` itself completed and its workflow instructions were loaded.

This coordination record is not a completion claim. Owner test outcomes, remaining
behavioral blockers and final board evidence must be recorded separately.

## Coordinator regression mailbox

The configured neighboring tests were run, rather than assuming historical
completed features still compile after the checkpoint API changes:

- `/tmp/cmpct-finish-prov149-provider.log`: `prov149_codex_history` fails to
  compile because `history.snapshot()` now returns a `Result<Option<_>, _>`;
  its existing property test still calls `ok_or_else` on the outer result.
  Provider owner must migrate the real assertion without discarding errors.
- `/tmp/cmpct-finish-prov149-loop.log`: the companion `prov149_codex_reload`
  execution is recorded separately. Any analogous API migration belongs to the
  loop owner, preserving its original assertions.

## Supervised current blockers

- The actual gate run in `/tmp/cmpct083-gate-970b4a72.log` is 18/19 GREEN,
  not complete: the Codex fresh child rejects nonpositive committed-context
  limits. Provider and sessions owners must retain registry-resolved limits;
  no guessed defaults or fixture-only overrides. Full handoff is in
  `gate-owner-supervised-2026-10-18.md`.
- Sessions owner reports four genuine restore-atomicity failures and one
  cumulative-spend failure. CLI owns preflight and aggregation repair;
  sessions owns integration and the unweakened acceptance assertions.
- `/tmp/cmpct-finish-prov149-loop.log` confirms two compile failures at
  `agent-loop/tests/prov149_codex_reload.rs:113` and `:135`: `is_none` is
  applied to `Result<Option<_>, _>`. Loop owner must propagate the outer
  error and retain the original absence assertions, then execute the target.
- Independent read-only inventory is `/tmp/cmpct-independent-audit.json` and
  its summary sibling. It covers all 27 active cards; missing steps, stale
  ranges and absent sidecars remain actionable, not permission to fabricate
  coverage. Owners must refresh after their final source edits.

No board card is marked complete on the basis of these partial results.

## Final review mailbox — 10:55 UTC

Latest cold-process gate is GREEN in
`/tmp/cmpct083-current-three-targets-final.log`: 11 cold-process scenarios,
8 failure scenarios and 2 stub serializer scenarios, unfiltered parent runs.
This supersedes the older 18/19 result, but does not waive the remaining
independent semantic and traceability review.

Read `/tmp/cmpct-independent-current-owner-defects.md` and
`/tmp/cmpct-independent-reaudit-owner-actions.md` before final handoff.
Treat old observations as hypotheses to verify against current source, not
unchanging defects after a repair. Remaining owner responsibilities include:

- Core is strengthening the failed-promotion fixture, whole-candidate positive
  control and exact cold-suffix equality. The coordinator explicitly requested
  this review; existing passing tests alone did not close these oracle gaps.
- CLI must exercise matching actual Anthropic/Gemini serializers in the alias
  identity scenarios, rather than labeling an installed OpenAI serializer as
  either provider. Finish structured-overflow sidecar, exact steps, new recovery
  scenario mapping and correct function spans/statistics after final edits.
- Loop must run the outer terminal-owner integration and preserve the original
  neighboring `prov149_codex_reload` safety assertions against legitimate
  generation/replay state, not delete them to obtain GREEN.
- Providers must retain original typed-output continuation assertions while
  supplying legitimate history lineage in `prov149_codex_history`; independently
  isolated private lock/writer tests must not be replaced by unbound files.
- Sessions must finish actual-runtime replay/unknown-context tests with genuine
  measured terminal usage, then refresh every complete scenario test mapping,
  implementation mapping and sidecar statistic.
- Rig must finish exact Unicode test ranges and genuine terminal usage tests;
  keep missing usage fail-closed instead of accommodating incomplete fixtures.

All strengthened existing passing acceptance tests are compatibility evidence,
not fabricated new behavioral RED. Any exposed production defect still requires
the executed failing test before its repair. No board or completion claim is
authorized until these owners and the final independent audit substantiate it.

## Next CLI integration priorities

The remaining cross-owner failures must take priority over repeating already
passing publication tests or compaction summaries. CLI owner `d3fea5df` owns
all three repairs; the acceptance tests are already genuine behavioral RED:

1. Expose `Session::preflight_compaction_restore(manifest, envelopes)` using
   provider `validate_committed_restore` (or `validate_restore` without a
   boundary). It must validate every retained checkpoint before live boundary,
   counters, writer bindings or readiness mutate. Sessions owner integrates
   it ahead of boundary installation and reruns `cmpct061_restore_atomicity`.
2. Fix final-response cumulative output/reasoning aggregation, not occupancy.
   `cmpct067_runtime_fill` observes correct current occupancy 8000 but spend
   1700/300 instead of 3400/600. Preserve its two physical segment assertions.
3. Route the original structured terminal overflow to the outer recovery owner
   before in-loop recovery mutates history/exhausts its budget when the outer
   owner is present. Preserve standalone CLI recovery. Loop-owned
   `cmpct054_terminal_dependencies` checks ordinary handlers are already gone
   while compaction dependencies survive; an in-loop builder is not equivalent.

Sessions alone is repairing registry-limit propagation against the existing
Codex cold-process RED; do not duplicate that production edit in providers or
weaken positive-limit admission. Gate owner reruns both unfiltered targets only
after the production repair. The current 18/19 count is still incomplete.

## Post-compaction coordinator mailbox — 11:03 UTC

All eight original owners remain assigned; no overlapping replacement owner is
authorized. The 10:54 independent semantic review remains actionable until each
item is verified against current source and execution. Old RED results are not
asserted to remain current after a repair. Gate and Rig received fresh tasks.

- Loop: `/tmp/cmpct-loop-supervised-all-retry.log` currently stops at the
  missing `has_terminal_overflow_owner` test import. The latest CLI history
  retry reaches the same compile blocker; neither log is GREEN merely because
  its filename says so. Preserve the neighboring reload assertions. Also
  review the ignored `clear_history()` result at `agent_loop.rs:938`: the
  checkpoint API is now fallible, so a failed clear must not report success.
- Providers: `/tmp/cmpct-provider-final-owned-suite2.log` records two failed
  neighboring targets, `prov149_codex_interruption` and
  `prov149_codex_reminders`. Preserve typed checkpoint and continuation
  assertions; distinguish a legitimate generation-bound cleared checkpoint
  from an absent checkpoint instead of silently dropping identity guarantees.
- Core, CLI and Sessions: finish the owner-specific semantic and exact
  traceability work in `/tmp/cmpct-independent-reaudit-owner-actions.md`, then
  provide a current final readiness handoff with actual command outcomes.
- The current filesystem has 19 GiB available. Avoid new broad builds and
  unit-harness compilation until safe headroom is established; continue bounded
  cached targets and read-only traceability review. Do not delete repository or
  shared build artifacts without coordinating with active owners.

This mailbox is a coordination record, not a completed-card or full ACDD
provenance claim. No temporal-validation bypass or assertion weakening is
authorized.

## Resumed coordinator mailbox — 11:13 UTC

The original exclusive owners remain assigned after context recovery. Use
`/tmp/cmpct-independent-unclosed-owner-actions.md` (11:08) instead of older
intermediate defect counts. In particular, the core semantic-oracle repairs,
loop cold-replay/durable-snapshot repairs, and sessions actual-runtime tests
are credited; do not repeatedly repair superseded findings.

Current remaining work is CLI semantic/sidecar completion, consolidated sessions
scenario mappings, loop neighboring checkpoint regressions and final spans,
Rig terminal-usage evidence and Unicode bounds, and the strengthened gate
oracles. The first strengthened gate run fails compilation, so its earlier
11/11 and 8/8 results are not execution evidence for the new test bytes.
Provider compilation and library Clippy have completed successfully in
`/tmp/cmpct-provider-completion-quality.log`; neighboring targets and affected
test-target quality checks have separate logs and must complete independently.

At recovery the filesystem has 22 GiB free. Continue cached, bounded targets
and avoid uncoordinated deletion or broad workspace compilation. Board changes
remain coordinator-only and require the current owner handoff and independent
acceptance/traceability checks, not an old attachment's completion wording.
