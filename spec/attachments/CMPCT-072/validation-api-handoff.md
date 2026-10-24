# Shared DAG validation API

## Current core completion — 2026-10-18

The existing API shapes are unchanged. `capture_compaction_source` and durable
promotion now reject missing immutable original references before selected
admission. Reconciliation validates DAG/boundary/original availability before
clearing publication pending. Cold replay also rejects missing model, committed
revision and output-reservation identity/admission metadata. Public
`publish_compaction` rejects obsolete/already acknowledged identity before its
callback and runs callback/acknowledgement under the manifest writer transaction;
callbacks must not reenter manifest mutation.

Behavioral RED: `/tmp/cmpct-core-completion-red2.log` (4 failures).
GREEN: `/tmp/cmpct-core-completion-green.log` (31 tests, all seven selected targets).
Four owned sidecars pass exact scenario/range/scoped-tag verification in
`/tmp/cmpct-core-completion-mappings.log`. Full commands and limitations are in
`../CMPCT-055/core-completion-evidence.md`. Actual CLI/loop route evidence remains
the runtime owners' responsibility; the loop over route names below is only a
core contract test, not proof of deferred/direct/self/cross execution.

## Current runtime obligation wiring — 2026-10-18

Core owner bbf1af42 confirms no new candidate fields or major API migration is
needed. Use `DagValidationContext.required_files` and `.required_reminders`
from the target's authoritative source captured under the operation lock, before
the builder runs. Do not infer the target's obligations from the builder's
working directory, route-name string, or an empty default context.

1. Capture target source and obligations for deferred/direct/self/cross paths.
2. Validate/assemble the full candidate before selected physical preparation.
3. In `commit_compaction_admitted`'s callback (which runs under the manifest
   writer lock), revalidate `candidate.dag.content` with those obligations,
   `candidate.source.prefix.len()` bounds, and `current.compaction`'s prior DAG.
4. Require the validated object to equal `candidate.dag`; only then prepare and
   admit the exact selected physical request. Reject before promotion on mismatch.
5. Preserve independently retained system reminders in runtime replay as well;
   DAG inclusion validation does not replace that runtime responsibility.

Existing core promotion enforces prior D2/file preservation by itself. It cannot
discover current runtime reminders or changed files. Empty defaults at the CLI
promotion call site remain an acceptance gap, not evidence of no obligations.
Parent/runtime must execute actual production route tests to close this gap.

The current cold-boundary fix also rejects malformed committed syntax,
out-of-source coordinates, uncovered prefixes, missing operation/protocol and
non-successor source generations. This protects cold replay, not route capture.

Owner: session `d88eaa8c-40af-430c-96ce-f54c847669fb` (CMPCT-070/072/073).
Commit-route owner: session `7e9bffc3-3f48-459a-8be9-1da17dbb256a` (CMPCT-055/059).

Current integration contract is maintained in
`spec/attachments/CMPCT-070/validation-api-handoff.md`. Route tests must execute
the real deferred/direct/self/cross commit paths, not loop over route-name strings
calling the standalone validator. These cards cannot close without route evidence.

Exports from `codelet_core::compaction`:

- `validate_dag_structure(content)` validates complete syntax and returns metadata.
- `validate_dag_candidate(content, &DagValidationContext)` validates syntax,
  persisted bounds, coverage and durable preservation. Returns unchanged candidate
  text plus complete metadata, or typed `InvalidSource` / `InvalidCandidate`.

Context fields:

- `persisted_turn_count: usize`: authoritative persisted source count, not the
  active message count after prior compaction.
- `required_coverage_end: Option<usize>`: inclusive required persisted prefix end.
- `existing_dag: Option<String>`: authoritative previous marked DAG, if any.
- `required_files: BTreeMap<String, String>`: authoritative file identities and
  operations (`Created`, `Modified`, `Deleted`).
- `required_reminders: Vec<String>`: authoritative required reminder identities.

The validator automatically requires all previous D2 metadata/body bytes and file
identities. Cross-depth overlap is valid; same-depth overlap is rejected. Missing
or malformed required file metadata is rejected, not repaired. Capture source
under its owning lock; inaccessible source must fail before constructing context.

Route integration is still required in CMPCT-055/059: every deferred/direct/self/
cross route must call the same validator before promotion. The standalone core
tests do not establish that production routes consume the API.

Initial validation evidence:

- `/tmp/cmpct070-behavioral-red.log`: four behavioral failures before implementation.
- `/tmp/cmpct070_whole_dag-green.log`: five core contract tests pass directly.
- `/tmp/cmpct072_durable_preservation-green.log`: four preservation contract tests
  pass directly.
- Required scoped Cargo runs are pending in
  `/tmp/cmpct070072-resumed-cargo.log`; previous attempts failed in unrelated shared
  CLI `reset_session_to_reminders` Result migration.

Do not mark CMPCT-070/072 done solely from standalone validator tests. Source
ownership, route integration and durable commit effects need route-owner evidence.
