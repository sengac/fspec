# BUG-251 / BUG-252 integrated finish audit

## Scope and ACDD

The existing feature-first executable RED evidence is retained, not recreated
or replaced with compilation failures. BUG-251 originally executed six tests:
two passed and four failed on lost image archives or unsupported restored media.
BUG-252 originally executed six tests: all six failed on the classification or
missing sanitized diagnostics. Production implementation followed those runs.

The connected implementation uses the common recoverable structured-input writer
and existing content-addressed blob store. Images and supported stored PDF/URL
sources use the common replay decoder for distinct display and model projections.
Discovery preserves configured model IDs and distinguishes unsupported listing
from real authentication, HTTP, protocol, transport or skipped-runtime failures.
Inference availability remains unknown; discovery never invokes inference.

No parallel media store, provider transport downgrade, arbitrary discovered model,
live configuration rewrite, commit or done transition is introduced.

## Finish changes

- Retain the existing production integration while auditing its real callers.
- Permit BUG-249/250's narrow `_transcript` display-only replay hook; BUG-251
  does not edit that shared decoder concurrently. Permission was delivered to
  owner session `bc2c91d4-3bec-46f2-b35e-1a32328d3022` after it became idle.
- Replace broad media coverage ranges with bounded implementation anchors through
  the serialized fspec wrapper. All six scenarios still have exactly one test.
- Remove media bytes/request bodies from two document-test failure messages.
  The assertions remain exact; only diagnostic payload exposure changes.
- Refresh mapped test bounds after that formatting-only assertion expansion.
  The main integration target is 294 lines, and its coverage file is 248 lines.

## Executed validation

`/tmp/bug251-252-finish-spec-audit.log` records both feature syntax checks, tag
checks and 6/6 mapped scenarios per card. The read-only audit verifies exact
scenario names, one mapped test, every exact `@step` inside its test bounds and
all implementation anchors inside their source files. This measures traceability,
not runtime coverage or behavior.

The refreshed `/tmp/bug251-252-finish-bounds.log` confirms all twelve exact
single-test mappings after the diagnostic-only test edit. Focused
`rustfmt --check --edition 2021` and `git diff --check` passed for the owned
changes. A read-only independent discovery source audit found no concrete
classification defect; it is not a substitute for executable tests.

Initial finish executions stopped before any scenario ran:

| Evidence | Actual result |
| --- | --- |
| `/tmp/bug251-finish-green.log` | Core E0599, missing `MessageStore::store_with_id` |
| `/tmp/bug252-finish-green.log` | Same shared-core compile blocker |
| `/tmp/bug251-252-finish-green-retry.log` | RPC transcript modules lacked dependencies during concurrent integration |

Those logs are neither behavioral RED nor GREEN. The storage owner subsequently
added the stable-ID primitive. BUG-246 fixed its shared admission gate's string
type mismatch after a successful coordination message. The RPC transcript owner
subsequently registered its dependencies. No reserved core, admission or RPC
source was independently patched by this audit.

## Current integrated run

The sequential pipeline uses the existing release-slim artifacts, two build jobs
and no target cleanup. Run from `rust/`:

```sh
cargo test -p codelet-sessions \
  --test bug251_multimodal_archives \
  --test bug252_discovery_classification \
  --test bug218_optional_model_discovery \
  --test prov131_local_profile_probe_resilience \
  --profile release-slim -j 2 -- --nocapture
cargo check -p codelet-sessions -p codelet-providers --lib \
  --profile release-slim -j 2
cargo clippy -p codelet-sessions -p codelet-providers --lib \
  --profile release-slim -j 2 --no-deps -- -D warnings
cargo clippy -p codelet-sessions \
  --test bug251_multimodal_archives \
  --test bug252_discovery_classification \
  --profile release-slim -j 2 --no-deps -- -D warnings
```

Logs are `/tmp/bug251-252-finish-green2.log`,
`/tmp/bug251-252-finish-check.log`, `/tmp/bug251-252-finish-clippy.log` and
`/tmp/bug251-252-finish-test-clippy.log`. Commands after a failing predecessor
do not run and are not claimed passing.

## Observed GREEN

The integrated test command completed after 41m 09s of queued build/link work.
All four targets executed successfully:

| Target | Result |
| --- | --- |
| `bug251_multimodal_archives` | 6 passed, 0 failed |
| Fresh child accepted-image cold replay, embedded input | 1 passed, 0 failed |
| Fresh child accepted-image cold replay, WebSocket input | 1 passed, 0 failed |
| `bug252_discovery_classification` | 6 passed, 0 failed |
| `bug218_optional_model_discovery` | 7 passed, 0 failed |
| `prov131_local_profile_probe_resilience` | 3 passed, 0 failed |

That is 22 main-process tests plus two fresh child invocations. Both cards have
durable copies of the fresh GREEN log. BUG-252 also has durable copies of its
original executable RED and initial GREEN, preserving their actual chronology.
The scoped check/strict-Clippy stages are still running or queued; no pending
stage is claimed passed.

The pipeline's shell PID is `94551`; its scoped library check PID is `2612`.
These are existing processes, not commands to rerun. The check has waited more
than ten minutes on the shared build lock after GREEN. Shared owners are still
integrating journal, archive and recovery changes. No extra build target, Cargo
profile, target directory or build cleanup is introduced to bypass that lock.
Available disk was about 11 GiB at the latest observation.

BUG-251 is now validating after actual integrated GREEN. BUG-252 remains
validating. The parent owns independent completion audit; neither card is done.
