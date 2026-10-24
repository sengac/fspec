# Provider diagnosis of frozen Z.AI reservation failures

Owner: provider session `bee8c5ed-a798-4b83-be98-3b43164bb436`.
Coordinator request and actual frozen gate RED were inspected before any edit.
No provider production, test, feature or coverage changes were needed or made.
This document is the durable cross-owner handoff; no board/stage/commit actions.

## Root cause and disposition

The failing gate fixture supplied registry output4096 and explicitly overrode
every selected provider to4096. The actual Z.AI native builder intentionally
serializes8192 (`rust/providers/src/zai.rs:264`). CLI passes the authoritative
manager reservation unchanged; selected admission correctly rejects8192>4096.
Accepting that mismatch, silently increasing the reservation, ignoring a real
user override or reducing the wire allowance would weaken the existing contract.
None of those production changes was made.

The gate owner had already repaired the fixture when this request was inspected:
`rust/sessions/tests/cmpct083/fixture.rs:120` now supplies the real8192 Z.AI limit
for both registry data and fixture overrides, including cold-process setup.
`rust/sessions/tests/cmpct083/zai.rs:27` additionally requires selected manager
and actual provider limits to agree. The physical wire assertion still requires
the provider's unchanged8192 allowance. This is an input-fixture correction,
not authorization to bypass an actual4096 override in production.

## Actual RED and repaired gate

- `/tmp/cmpct083-frozen-final-gates.log`: cold9/11, both Z.AI rows fail with
 8192 versus4096, CARGO_EXIT101. SHA256
 `5f2959e4c703eb221ba3279f7f52e1ef78223479244c1864d14440a6079911b3`.
- `/tmp/cmpct083-frozen-final-repaired-gates.log`: parent cold11/11,
 failure8/8 and stub2/2, CARGO_EXIT0. Nested child-process results are not
 double-counted. SHA256
 `a8184de4d052bbf0d904d93561a38c6e336d72046fe987e325372d37b859ff20`.
- `/tmp/cmpct083-frozen-final-repaired-status.log` records audit/fmt/diff/check/
 clippy/final-metadata exits0. Gate owner owns final gate certification.

No new provider behavior was introduced, so no new failing provider test or
test-first production repair is claimed. Existing gate RED is genuine; the
already executed gate-owner fixture repair is not attributed to this owner.

## Fresh focused provider verification

From `rust/`, executed cached ci-test with explicit targets and `-j2`:

```sh
cargo test --profile ci-test -p codelet-providers --features test-support -j2 \
  --test cmpct071_selected_preparation --test cmpct071_builtin_output_limits \
  --test cmpct078_attempt_authority --no-fail-fast
```

All9 tests pass, zero failed/ignored/filtered. They retain exact8192/4096/MAX
wire serialization and strict under-reservation rejection before physical send.
The same three targets plus lib pass cargo check and clippy with the same
profile/features/jobs. Commands and individual exits are retained in:

- `/tmp/cmpct-provider-zai-focused.log` (TEST_EXIT0).
- `/tmp/cmpct-provider-zai-check.log` (CHECK_EXIT0).
- `/tmp/cmpct-provider-zai-clippy.log` (CLIPPY_EXIT0).
- `/tmp/cmpct-provider-zai-fmt.log` (existing36-file scoped check, FMT_EXIT0).
- `/tmp/cmpct-provider-zai-traceability.log` (3features11scenarios, AUDIT_EXIT0).

The strict audit checks complete current test ranges, local ordered steps,
reachable assertions, implementation bounds, statistics, ordinary scratch
links and tags. It includes two additional setup/OAuth scenario mappings whose
executions are retained in the prior final suite/private-unit logs; those are
not counted as newly executed in this9-test focused run. No coverage bypass.
The broad109-test suite was not rerun.

## Exact source identity and cross-owner limit

Relevant provider source SHA256 values, all unchanged:

- `rust/providers/src/zai.rs`:
 `4ae7690e51a87db876dbbd78be1ab66c35ad9dd95c816e6cc06dbb2d48ca2551`.
- `rust/providers/src/manager.rs`:
 `3417db56b180d9b32731806f300bfdc99758313a9840b6646605c3e7eb5b730e`.
- `rust/providers/src/selected_request.rs`:
 `c6b0990dbcea28973ba41c14ffebc44191376e23cb7e5082c89d9ba07bfc630f`.
- `rust/providers/src/request_admission.rs`:
 `0a1fd62ca9585c637667a4913ffd95230532c8c55f955e95824038eaf54f6d32`.

Gate-owner repair source hashes:

- `rust/sessions/tests/cmpct083/fixture.rs`:
 `745382068294a64be9591cda24cb380ce7c00579e25500fca5c46ff4ce8e46b1`.
- `rust/sessions/tests/cmpct083/zai.rs`:
 `8f0bdb2b5afb6c350bee0d860f10e11a887e1bc54e247675342d5116bcfd64f1`.

The7-entry focused provider before/after manifests match; digest
`ceed6f6bec268aaa5a5c8daba1346fb70edce2a634d5053a34119212dd8baf35`.
Files: `/tmp/cmpct-provider-zai-diagnosis-before.sha256` and `-after.sha256`;
empty drift diff and SOURCE_DRIFT_EXIT0 are retained alongside them. The prior
704-entry provider/dependency source manifest also still verifies unchanged.

Independent current recheck of the repaired gate before-manifest has3301 matches
and ONE later foreign test-source drift:
`rust/patches/rig-core/tests/cmpct065_responses_usage_aggregation.rs`.
Current hash is
`1f9d85ce618ad6dd9cc2fc9d1560337608521f90fae37fc841c5fad01d468330`.
No whole-gate current-source freeze claim is made from that manifest. Evidence:
`/tmp/cmpct-provider-zai-gate-source-verification.log`. Gate owner414 received
this distinction successfully; coordinator notification returned busy/not queued.
Rig/CLI/loop/CMPCT083 source and sidecars were not edited by this owner.
