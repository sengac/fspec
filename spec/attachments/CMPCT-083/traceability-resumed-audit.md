# Resumed strict traceability audit — 2026-10-18

Owner: `78a79fad-6073-4e98-bfa9-ee9e6859aee0`, replacing `7bb4349a`.
No board edits, staging, commits, production edits or gate bypasses.

## Initial checkpoint

The inherited 17 owned feature files currently contain 110 scenarios. Twenty-three
scenarios have no test mapping. Existing mappings have implementation mappings,
but their presence does not establish semantic accuracy or current line ranges.
Seven features already map more than one test file and therefore fail the strict
transition gate. Newly written, currently unmapped tests require further splits.
The source gate explicitly requires one mapped test file per feature and step
documentation for every scenario. A scenario outline counts as one scenario here,
not one per examples row. This inventory is not measured coverage or green tests.

| Card | Existing feature | Scenarios | Mapped targets | Unmapped scenarios |
| --- | --- | ---: | ---: | ---: |
| 052 | compaction-inference-handoff | 8 | 1 | 3 |
| 053 | terminal-recovery-single-owner-locking | 3 | 1 | 0 |
| 054 | terminal-compaction-dependency-lifetime | 4 | 1 | 1 |
| 056 | cross-session-compaction-ownership | 11 | 1 | 5 |
| 057 | committed-compaction-watchdog-lifecycle | 4 | 1 | 1 |
| 058 | one-selected-overflow-dag-protocol | 2 | 1 | 0 |
| 059 | durable-versioned-dag-boundary | 14 | 2 | 8 |
| 060 | typed-durable-summary-identity | 5 | 1 | 0 |
| 061 | structured-generation-aware-session-replay | 6 | 1 | 2 |
| 062 | committed-token-state-restore | 3 | 1 | 0 |
| 067 | coherent-request-context-fill | 4 | 2 | 1 |
| 068 | committed-compaction-context-projection | 12 | 7 | 2 |
| 069 | complete-token-event-forwarding | 8 | 3 | 0 |
| 071 | whole-request-admission-before-dag-commit | 10 | 3 | 0 |
| 073 | saved-threshold-override-propagation | 4 | 1 | 0 |
| 079 | runtime-compaction-no-substitute-success | 5 | 2 | 0 |
| 082 | explicit-request-admission-without-fallback | 7 | 3 | 0 |

## Proposed handoff and execution

1. Obtain parent approval and stable specification/test-documentation handoff
   from CLI `56af903a`, loop `2bc4af08`, sessions/RPC/TUI `e9b3a644`.
   Shared provider targets need coordination with `042b9b3c`; core059 with
   `620d65fd`. Those owners retain production and their own sidecars.
2. Preserve scenario wording, examples, tags and architecture notes. Partition
   scenarios by actual owning integration target, retaining the original feature
   for its primary target and adding descriptive sibling features with the same
   work-unit tag. Do not move or delete regression tests to satisfy the gate.
3. A scenario genuinely requiring two targets cannot simply keep two mappings:
   preserve its promises as separately named target-specific acceptance scenarios
   with owner review, or obtain an owner-written single-target integration test
   exercising every promise. Never pretend one target covers another's assertions.
4. Document actual Given/When/Then actions at their execution/assertion sites.
   Refresh ranges only after documentation and owner implementation stabilize.
   Link real functions to actual production functions with strict `fspec
   link-coverage`; no validation skip flags.
5. Run scoped feature/tag/coverage audits. Parent alone executes board transitions
   through normal gates. Report remaining semantic gaps separately from mappings.

## Initial split groups

- 052: CLI inference handoff; loop durable submission (three unmapped scenarios).
- 054: terminal recovery lifetime; real-turn cancellation cleanup.
- 056: cross-session locking; selected target identity; deferred candidate
  identity/cancellation; caller interruption. Successful cross-target cold reload
  promises need semantic review rather than automatic linkage.
- 059: core durable boundary; CLI runtime commit; selected publication;
  reminder obligations; promoted acknowledgement. Automatic publication currently
  shares a scenario across two targets; owner review required.
- 061: structured replay; provider-reset failure atomicity.
- 067: coherent backend/TUI fill; runtime header fill; header provenance.
- 068: TUI committed projection; runtime publication; RPC snapshot reduction;
  durable session publication; restored projection; restored counters; pending
  restore; pending projection; delayed background ordering.
- 069: JSON forwarding; physical settlement ledger; runtime finish settlement.
- 071: CLI whole-request candidate fit; serialized request admission; selected
  physical request preparation. Builtin/custom/stub parity sidecars stay with
  their existing owners.
- 079: no-substitute runtime outcomes; recovery preparation isolation.
- 082: CLI limits/error classification; provider physical admission; admission
  lock/hook cancellation. Core082 fail-closed hook sidecar remains core-owned.

Parent was busy when the initial report message was attempted; this attachment
is the durable coordination channel. No cross-owner files changed pending handoff.

## Read-only verification and semantic warnings

- Installed `fspec audit-coverage compaction-inference-handoff` reports all 14
  referenced files found; the corresponding 068 audit reports all 27 found.
  These commands verify paths, not complete scenario mapping or assertions.
- Installed `fspec show-coverage compaction-inference-handoff` reports **100%
  (5/5)** while listing three further scenarios as uncovered in the same output.
  Treat this summary as misleading, not completion evidence. Independent feature
  scenario enumeration is mandatory. No reporting implementation changes here.
- All inherited mapped test ranges include a top-level function and stay within
  current file bounds; all inherited implementation ranges also stay in bounds.
  This shallow check does not establish that their selected production lines
  still execute the scenario behavior.
- `cmpct059_runtime_commit.rs` currently puts a publication-failure `When`
  comment inside the successful-installation test, before appending suffix tool
  work. Its actual failure test lacks that action's documentation. Whole-file
  matching can accept this misplaced comment; handoff must fix placement, not
  merely obtain a green gate. A duplicated capture Given also exists.
- `cmpct059_selected_publication.rs` has meaningful publication-pending and exact
  identity assertions but no `@step` documentation. Link only after documenting
  real setup, failing publication and reconciliation actions.
- Successful cross-target compaction in `cmpct056_cross_session_locks.rs`
  verifies durable boundary identity, two covered originals and published token
  agreement, but its newly added cold-reload step has no matching documentation.
  This explains its unmapped state; do not discard its strengthened promise.
- The worktree gained `selected-compaction-provider-identity.feature` during the
  audit, without a sidecar yet. It is new owner work, excluded from the inherited
  17-feature/110-scenario snapshot; final inventory must include it after handoff.

## Sessions handoff accepted

`e9b3a644` explicitly handed off stable specifications and test documentation for
060/061/062/067/068/073, reserving only its runtime067 distinct-response-ID fixture
repair. Parent coordination message was rejected busy; this records the approved
owner handoff before edits. Target partitions will retain original scenarios and
report assertion gaps honestly; approval does not authorize weakening promises.
