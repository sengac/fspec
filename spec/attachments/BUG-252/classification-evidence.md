# BUG-252 classification evidence

## Discovery and provenance

- Inspected the actual `probe_profile_models` AST and its callers in provider
  enumeration, inherited profiles, startup selection, settings, and the selector
  header. The existing nonblank custom-model exemption was already present.
- The prior empty/blank-model behavior conflates an unsupported listing route
  with endpoint unreachability. Clearing that false assertion does not enable
  arbitrary selection: startup still requires at least one real model entry.
- Baseline checkout: `e1612b799d4587a1c12993c028acf27c1326884e`, with concurrent
  work in the shared checkout. Rust: `1.95.0 (59807616e 2026-04-14)` on macOS
  aarch64. Board CLI: `fspec 0.10.9`.
- Reproductions use compiled Rust integration tests, isolated `FSPEC_USER_DIR`
  temporary directories, synthetic credentials and loopback endpoints only.
  No live configuration or external inference service is changed or contacted.
  Historical profile contents and running executable identity remain unknown.
- A read-only current user-config snapshot (mtime
  `2026-10-22T23:32:45.188426+00:00`) contains three OpenAI profiles, each with a
  configured base URL and zero configured/nonblank custom models. Only anonymous
  counts were emitted: no profile names, endpoints, IDs or keys. This is current
  configuration evidence, not proof of the configuration used by historical logs.

## Contract

| Observation | Discovery diagnostic | Inference knowledge | Startup selection |
| --- | --- | --- | --- |
| HTTP 404/405, configured models | unsupported | unknown | configured IDs remain eligible |
| HTTP 404/405, absent/blank models | unsupported | unknown | no selectable models; no unreachable assertion |
| HTTP 200, valid IDs | supported | unknown | returned/configured IDs only |
| HTTP 200, empty data | empty | unknown | configured IDs only, if present |
| HTTP 401/403 | authentication failure | unknown | blocked, configured IDs retained |
| Other unsuccessful HTTP status | HTTP failure | unknown | blocked, configured IDs retained |
| Invalid JSON/schema/model IDs | protocol failure | unknown | blocked, configured IDs retained |
| Refused connection or timeout | transport failure | unknown | blocked, configured IDs retained |
| Missing multithread runtime | skipped | unknown | blocked without an HTTP request |

The existing `ProviderInfo.is_unreachable` wire field remains the conservative
startup-selection gate for real probe failures. It is not a positive inference-health
signal when false. Empty model lists independently prohibit startup selection.
Settings retain persisted configuration; discovery never rewrites custom models
or persisted selections. There is no wire-schema expansion or capability guess.

Discovery performs one request at the existing composed models URL, with the
existing five-second timeout and optional bearer header. It does not retry, try
alternate routes or invoke inference. A later enumeration deliberately probes
again. Unsupported discovery is not retryable; failed probes do not acquire an
optimistic custom-model bypass. Diagnostic events contain category, numeric HTTP
status where applicable, unknown inference state, a probe-failed flag and a timeout flag,
not URL, profile name, keys, model payloads or provider response bodies.

## ACDD sequence and coverage

1. Read project guidance, testing guide, foundation and repair brief.
2. Capture rules, examples, user story and architecture notes through the
   serialized board wrapper; inspect existing Rust through AST and source.
3. Specify `profile-discovery-classification.feature` before creating tests;
   format/validate the feature and move BUG-252 to testing.
4. Create six scenario tests with exact step annotations and one test mapping
   per scenario. The old BUG-218 empty-profile assertion is superseded by the
   unknown-inference contract; its nonblank custom-model regression remains.
5. Run real RED before production edits, then make focused helper extractions
   rather than a broad provider-settings or profile refactor.
6. Run scoped GREEN and quality checks; leave BUG-252 validating for parent audit.

The six scenarios cover unsupported empty profiles, persisted configured-model
cold startup/switching with independently accepted inference, populated/empty
discovery, authentication/HTTP/protocol failures, refused/timeout transport and
skipped runtime. Assertions exercise actual profile enumeration, settings reads,
custom-model loading, persisted selections, startup resolution and captured
diagnostics. The independent inference request is made by the test, not by
discovery. Existing current-thread-runtime regression coverage is retained.

## Commands and results

Run Cargo commands from `rust/`; each command scopes package and test/lib target,
uses `--profile release-slim -j 2`, and tees the complete output to `/tmp`.

RED command:

```sh
set -o pipefail
cargo test -p codelet-sessions --test bug252_discovery_classification \
  --profile release-slim -j 2 2>&1 | tee /tmp/bug252-red.log
```

The initial `/tmp/bug252-red.log` attempt stopped on concurrent compilation
errors in reserved provider/settlement files. That is not behavioral RED.
The identical retry wrote `/tmp/bug252-red-retry.log` and executed all six
tests: **0 passed, 6 failed**, before any BUG-252 production edit. The empty
unsupported profile failed the unreachable assertion; the other five tests
failed on missing categorized, sanitized diagnostics. In particular, configured
model startup/settings assertions already passed before the diagnostic failure,
confirming that the pre-existing exemption was not absent.

Scoped validation commands:

```sh
cargo test -p codelet-sessions --test bug252_discovery_classification \
  --profile release-slim -j 2 2>&1 | tee /tmp/bug252-green.log
cargo test -p codelet-sessions --test bug218_optional_model_discovery \
  --test prov131_local_profile_probe_resilience --profile release-slim -j 2 \
  2>&1 | tee /tmp/bug252-profile-regressions.log
cargo test -p codelet-providers --test bug218_model_discovery_errors \
  --profile release-slim -j 2 2>&1 | tee /tmp/bug252-provider-regressions.log
cargo check -p codelet-providers -p codelet-sessions --lib \
  --profile release-slim -j 2 2>&1 | tee /tmp/bug252-check.log
cargo clippy -p codelet-providers -p codelet-sessions --lib \
  --profile release-slim -j 2 --no-deps -- -D warnings \
  2>&1 | tee /tmp/bug252-clippy.log
cargo clippy -p codelet-sessions --test bug252_discovery_classification \
  --profile release-slim -j 2 --no-deps -- -D warnings \
  2>&1 | tee /tmp/bug252-test-clippy.log
cargo clippy -p codelet-providers --lib --profile release-slim -j 2 \
  --no-deps -- -D warnings 2>&1 | tee /tmp/bug252-provider-clippy.log
```

Formatting and feature/tag/coverage validation pass, recorded in
`/tmp/bug252-format.log` and `/tmp/bug252-spec-quality.log`. Coverage reports
**6/6 scenarios fully mapped**, one test per scenario, with connected provider
and session implementation anchors. A read-only audit also checks every exact
step annotation within its mapped test-function bounds. The new coverage file
is 280 lines; all new Rust files are below 300 lines.

Observed results:

| Validation | Result | Evidence |
| --- | --- | --- |
| New classification scenarios | 6 passed, 0 failed | `/tmp/bug252-green.log` |
| Existing optional-discovery regressions | 7 passed, 0 failed | `/tmp/bug252-profile-regressions.log` |
| Existing runtime-resilience regressions | 3 passed, 0 failed | `/tmp/bug252-profile-regressions.log` |
| Provider discovery regressions retry | 3 passed, 0 failed | `/tmp/bug252-provider-regressions-retry.log` |
| Provider/session library check | pass | `/tmp/bug252-check.log` |
| Provider-only strict library Clippy | pass, zero warnings | `/tmp/bug252-provider-clippy.log` |
| Formatting, feature/tag validation, coverage audit | pass | `/tmp/bug252-format.log`, `/tmp/bug252-spec-quality.log` |
| Exact scenario/test/step/implementation bounds | pass, 6/6, one test each | `/tmp/bug252-step-audit.log` |
| Provider regression and strict test Clippy first attempts | infrastructure failure before execution: missing shared build files | `/tmp/bug252-provider-regressions.log`, `/tmp/bug252-test-clippy.log` |
| Strict library Clippy retry | blocked by concurrent core compilation error E0599 | `/tmp/bug252-clippy.log` |

At approximately `2026-10-23T00:47Z`, queued builds failed with ENOENT for
`proc-macro2`/`quote` build-script dependency and fingerprint files under the
shared release-slim target directory. Other owners reported the same interference;
this repair does not delete or clean build artifacts. This is not a failing
classification test and does not supersede the completed GREEN runs. Cargo
library check/Clippy were not reached by the stopped validation pipeline.
The sequential retry passed all three provider regressions and the scoped
provider/session library check. Strict library Clippy subsequently stopped on
concurrent core E0599: `core/src/persistence/conversation_commit.rs:20` calls
`MessageStore::store_with_id` before the storage owner's implementation is
available. This is recorded in `/tmp/bug252-clippy.log`; reserved core files are
not modified by BUG-252. The sequential pipeline did not reach test-target
Clippy. Provider-only strict library Clippy passes independently. Pending checks
are not claimed as passing. BUG-252 is now validating, not done; rerun combined
library Clippy and the test-target Clippy command above after the storage API is
available and before parent completion audit.

All board/coverage mutations use:

```sh
python3 /tmp/fspec-bug242-252-command.py <fspec-command-and-arguments>
```

No commits, new branches, live-state changes or global tool configuration edits
are part of this repair. Completion/done remains the parent's responsibility.
