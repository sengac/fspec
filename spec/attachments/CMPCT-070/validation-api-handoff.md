# Shared DAG validation API

Owner: session `d88eaa8c-40af-430c-96ce-f54c847669fb` (CMPCT-070/072/073).
Commit-route owner: session `7e9bffc3-3f48-459a-8be9-1da17dbb256a` (CMPCT-055/059).

Current coordination (2026-10-17): runtime integration remains required. Route
owner must pass authoritative persisted source coordinates, never active-message
length, and must fail closed when existing source cannot be captured. CMPCT-071
owner `663f0f7e` supplies complete serialized-request admission, not a DAG estimate.
`validate_dag_candidate` will assemble a missing file section from authoritative
sorted identities before validating the complete candidate. Existing malformed,
empty or conflicting sections are rejected, never repaired. Use its returned
unchanged/assembled content and nodes together for durable promotion.

Fresh source-capture regression: `/tmp/cmpct072-source-red.log` records a real
failure in `capture_compaction_source`: a malformed persisted marked DAG reached
the builder source contract. Capture now validates committed lineage, persisted
coverage and the existing whole DAG before returning a source. Verification:
`/tmp/cmpct070072-source-green.log` (running). Do not bypass capture on the
deferred/self/cross path or recapture only after the builder finishes.

Promotion safety: `ValidatedDagCandidate` currently has public fields, so its
content/metadata can change after validation. Integration target
`core/tests/cmpct070_promotion_validation.rs` observed a behavioral failure in
`/tmp/cmpct070-promotion-red.log`. The transaction now revalidates the whole DAG
inside `commit_compaction` against the captured immutable prefix and authoritative
previous DAG, before request admission; it rejects text/metadata inconsistency.
Route preparation must still supply captured current
file changes and required reminders; prior DAG D2/files are independently checked
again by the durable transaction.

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
