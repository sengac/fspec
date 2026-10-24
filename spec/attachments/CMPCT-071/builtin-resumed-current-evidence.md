# Resumed builtin provider verification — 2026-10-18

Owner: c74aea6f. Preserve inherited changes; coordinator alone owns board states.
No staging, commits, other provider-production changes or gate edits.

The inherited Gemini native function-name repair is retained. The seven-target
current-source run in `/tmp/cmpct-builtin-resumed-current-tests.log` passes all
six Anthropic066 tests, the Gemini pair, all output-limit tests and all eleven
serializer tests. Because new rejection tests were added during compilation,
that run additionally demonstrates two authentic behavioral REDs: Gemini and
OpenAI Responses lose typed HTTP status/body. The prior OpenAI Chat rejection
test passes. This mixed run is NOT a final green verification.

The structured-stream-rejection feature was extended and formatted/validated
before adding real local-HTTP regressions for Gemini, Responses and Anthropic.
Each checks structured overflow, misleading authentication message and plain
text, exact typed status/body and termination after exactly one error.
Strict `link-coverage` accepts each test's actual scenario-step mapping without
bypasses in `/tmp/cmpct-builtin-resumed-test-mappings.log`.

The complete four-test RED is queued in
`/tmp/cmpct-builtin-resumed-http-red.log`; Anthropic additionally flattens errors
and emits a second incomplete-usage error. No production repair precedes the
complete behavioral RED. Shared ci-test cache, scoped `-j 2`; disk remains
above 20 GiB (44 GiB observed). Busy parent notification was rejected, not queued.

The original 17:50 Gemini source-ready evidence and RED/GREEN logs remain in
`CMPCT-066/rig-provider-current-audit.md`. Full cold-process083 chronology,
negative runtime gates and terminal054 builder acceptance remain external;
this builtin adapter work does not certify those gates.

## Stable complete handback — 2026-10-18 18:57 AEST

The earlier pending statements above are superseded by actual completed results.
Production and test source are frozen after this handback unless the coordinator
requests further work. No duplicate baseline was rerun after the supervisor's
handoff request; the already-running quality chain was allowed to complete.

### Actual RED, then repair, then GREEN

`/tmp/cmpct-builtin-resumed-http-red.log` records **one passed, three failed**,
zero ignored/filtered, with Cargo reporting test failure. Each new real HTTP
regression fails specifically because its original typed status/body is lost:
Anthropic, Gemini and OpenAI Responses. The inherited Chat regression passes.
The feature was updated and formatted/validated before the new tests, and all
three failures were inspected before production changes.

The three streaming adapters now close their event source, yield the existing
`CompletionError::HttpError` with the original error and return immediately.
They no longer flatten original HTTP status/body into `ProviderError` display
text or fall through to synthetic successful finals/secondary usage errors.
No HTTP status, body, endpoint, request mode or selected-model identity changes.
The original Chat typed-error repair and Gemini native tool-pairing repair remain
intact. No CLI/loop classification or error-chain substitution is introduced.

`/tmp/cmpct-builtin-resumed-current-green.log` includes this final production and
test source: **26 passed, zero failed/ignored/filtered**, `EXIT_STATUS=0`.
The exact bounded command, from `rust/`, is:

```sh
cargo test --profile ci-test -j 2 -p codelet-providers \
  --test cmpct066_anthropic_terminal_usage \
  --test cmpct071_builtin_gemini_tool_pairing \
  --test cmpct071_builtin_output_limits \
  --test cmpct071_builtin_serializers \
  --test cmpct071_builtin_serializers_options \
  --test cmpct071_builtin_serializers_unicode \
  --test cmpct071_builtin_stream_rejections
```

Counts in that order are **6 + 2 + 3 + 6 + 4 + 1 + 4**. The four rejection tests
perform twelve real POSTs covering structured HTTP400 overflow, HTTP401 with
misleading overflow message text and plain HTTP400 text. Each checks exact typed
status/body and exactly one error with no successful final. Existing serializer
byte identity, generation, no-preparation-send, source preservation, Unicode,
mandatory context, output reservation and native tool-pair semantics all remain.

### Completed quality

These existing jobs completed with explicit `EXIT_STATUS=0`:

| Exact command from `rust/` | Evidence |
| --- | --- |
| `cargo check --profile ci-test -j 2 -p rig-core -p codelet-providers --lib` | `/tmp/cmpct-builtin-resumed-check.log` |
| `cargo clippy --profile ci-test -j 2 -p rig-core --lib` | `/tmp/cmpct-builtin-resumed-rig-clippy.log` |
| `cargo clippy --profile ci-test -j 2 -p codelet-providers` with the seven explicit `--test` targets above | `/tmp/cmpct-builtin-resumed-tests-clippy.log` |

Rig library Clippy has no warnings. Providers retain only the two inherited,
outside-owned-subtree warnings: unused `codex` import in manager and unused
`CodexItem::is_user`. No lint suppression or unrelated warning repair.

`/tmp/cmpct-builtin-resumed-final-traceability-format.log`, exit0, records:

- Exact-path feature format, syntax validation, coverage audit/report for all
  seven owned features, not global feature mutation.
- `rustfmt --edition 2021 --check` for the066 target, every builtin071 target
  and the shared builtin test support.
- `rustfmt --edition 2024 --check` for Anthropic request-usage/request-body,
  Gemini request-body, Chat request-body and Responses request-body helpers.
- Scoped `git diff --check` for provider-subtree changes, owned tests/specs and
  updated066 coverage sidecar.

Legacy oversized streaming files receive only their minimal error branches;
this does not claim whole-file rustfmt cleanliness for inherited legacy code.
An earlier diagnostic used edition2021 against Rig's2024 let-chain helper and
included inherited usage-helper format differences. The final correct-edition,
explicit helper/test checks above pass without broad legacy reformatting.

### Exact scenario traceability

All **26 scenarios map1:1 to26 real tests**, each feature maps one test target,
and every scenario has production implementation mappings. The rejection feature
now has four real mappings and correct4/4/100% statistics, not stale1/1 counts.
The066 implementation lines shifted by the Anthropic repair are refreshed.

| Feature basename | Scenarios / tests | Mapped lines |
| --- | ---: | ---: |
| `anthropic-terminal-cache-usage` | 6 | 423 |
| `builtin-provider-physical-serializer-parity` | 6 | 671 |
| `builtin-provider-physical-serializer-parity-options` | 4 | 161 |
| `builtin-provider-physical-serializer-parity-unicode` | 1 | 211 |
| `builtin-provider-structured-stream-rejection` | 4 | 135 |
| `builtin-provider-output-limit-serialization` | 3 | 68 |
| `builtin-provider-gemini-tool-pairing` | 2 | 139 |

Strict `fspec link-coverage` accepts each new test and corresponding implementation
mapping without skip flags: `/tmp/cmpct-builtin-resumed-test-mappings.log` and
`/tmp/cmpct-builtin-resumed-impl-mappings.log`. The latter explicitly exits0.
The read-only scoped integrity/registry check verifies exact scenario names,
one real test per mapping, literal Given/When/Then step documentation, all source
line bounds, mapped-line statistics, registered ordinary tags and feature-level
work-unit tags for all seven features. It exits0 in the final traceability log.
Mapped-line counts are traceability metadata, not measured execution coverage.
No new global `fspec validate-tags` success or real board transition is claimed.

### Readiness and external boundaries

- **CMPCT-066 owned slice ready:** six real HTTP/SSE usage regressions, complete
  mappings, scoped check/Clippy/format green. Its CMPCT-065 dependency remains
  the coordinator's separate quality/normal board-transition responsibility.
- **CMPCT-071 builtin slice ready:** twenty regressions across six explicit
  targets, six matching features, all owned mappings and scoped quality green.
  Other071 provider/core/CLI layers remain independently owned.
- **CMPCT-054 provider seam ready only:** all four builtin stream adapters retain
  original typed HTTP evidence; authentic outer-terminal builder invocation,
  strict classification and final error-chain preservation remain CLI/loop gates.
- **CMPCT-083 not certified here:** the observed fresh matrix no longer listed
  Gemini among its failures, but only the gate owner can certify its complete
  current-source cold-process chronology and negative runtime readiness gates.

No owned behavioral, mapping or scoped quality blocker remains. Latest disk
headroom is26GiB, above20GiB. Shared cache and `ci-test -j 2` only; no board,
index, staging, commit, branch, bypass or cross-owner production edit.
