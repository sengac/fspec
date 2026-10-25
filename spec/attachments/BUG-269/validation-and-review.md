# BUG-269 validation and direct review

## Scope and native approval

Read the actual card, attachment and original PROV-088 automatic-refresh
specification before changing production. The supervisory caller explicitly
approved `request-refresh-design.md` through native HITL, including exact selected
engine/AST/config ownership, explicit Custom headers ownership, credential-path
single-flight locks, admitted synchronous refresh settlement after caller
cancellation, root fencing and safe diagnostics. The actual answer is recorded
in native Example Mapping and `discovery-and-approval.log`. No human statement or
approval was fabricated. No inference retry, refresh retry/deadline, issuer,
credential fallback or native-provider policy was introduced.

## Genuine RED and fixture corrections

Specifications and native rules/examples precede the seven tests and production.
`actual-request-causal-red.log` records **1 passed / 6 failed** against unchanged
production: the first standalone nonstreaming expired request skips the issuer, two
independent providers exchange zero times, the selected project backend skips
refresh, rejected refresh incorrectly succeeds, and cancellation/root-wait tests
cannot observe any issuer admission. The headers-owned control already passes.
This establishes the actual helper-to-request composition gap, not merely an
unused-function/source-shape assertion.

The front-door parameterized RED stops at its first case; the other seven
front-door/alias combinations are asserted in final GREEN, not separately
claimed as observed causal RED. The later refused-connection and bearer/API-key
controls similarly extend final verification without inventing earlier failures.

Earlier attempts remain preserved. The original project agent fixture omitted
the required attributed tool catalog; echoing tools exposed another unsupported
direct-agent attribution setup. Neither is a causal refresh RED. The corrected
test constructs the normal project-selected CustomProvider agent and sends a
real request through its selected backend's legitimate completion front door;
it does not disable catalog/admission guards or alter the production tool path.
The later `final-runtime-and-controls.log` is a compiler failure from a fixture
helper accidentally placed inside a Rhai raw string, NOT a passing runtime run.
The first GREEN attempt is **6 passed / 1 failed**: a transport-fixture sentinel
zero was incorrectly used as a wiremock HTTP status. The sentinel now means a
numeric-loopback refused connection and no invalid mock response is installed.
No assertion or production policy was weakened to repair these fixture errors.

## Minimal connected implementation

All four existing standalone/rig streaming/nonstreaming routes already call
`invoke_build_headers`; this is the single new integration point. It binds the
selected compiled engine, AST and OAuth configuration rather than reloading a
global script by slug. Explicit device/PKCE OAuth plus supported preferred or
legacy hook pairs uses the existing ScriptedRefreshingClient sequence. Custom,
bearer/API-key and missing-hook paths keep their existing headers ownership.

The small weak lock table stores only credential-path locks, never credentials
or session metadata. Each independent request rechecks stored tokens while
owning that path's lock. A queued cancelled caller cannot newly admit refresh;
an already admitted synchronous callback retains its lock through settlement
and may persist rotated credentials, without resuming cancelled inference.
Canonical credential destinations are captured before admission and checked
after waits and before persistence. Store reads/writes use that captured path,
so a changed environment cannot redirect a background refresh to replacement
credentials. Script exception bodies are redacted at the new request boundary;
the existing helper's public script errors remain available to its old callers.
Storage conversion and 0600 behavior remain equivalent to the shared store.

## Final-source evidence

`settled-source-runtime-green.log`: **23 distinct tests pass**, no filtering:
seven new request scenarios, six existing custom-stream settlement controls,
five existing scripted device/refresh controls and five existing OAuth bridge
controls. The seven scenarios include eight preferred/legacy × four-front-door
cases, independently constructed concurrency, six authentication ownership
controls, project/global precedence, rejected 403/503 and actual refused local
transport, explicit later requests, cancellation, a genuinely timed caller
deadline armed after issuer admission, and changed-root active/queued requests.

Captured local HTTP verifies old refresh tokens, exact freshly authorized
inference, one exchange per credential path, no automatic inference replay,
ordinary completion/usage and persisted synthetic tokens. The credential reader
is a synthetic numeric-loopback `/stored` fixture reading only the captured
temporary token file, not an installed script, user credential or interception
of the refresh helper. Every admitted operation settles before restoring the
isolated environment. Temporary HOME/config/auth, mock keyring and local issuer
and inference are used throughout; no external or billable provider calls.

`final-shipping-check.log` and `settled-shipping-clippy.log` cover default library
consumers: providers, CLI, agent-loop, sessions and NAPI. `settled-test-clippy.log`
checks the explicit new integration target. The earlier test Clippy run rejected
three fixture `expect` calls; these now use proper error handling rather than
lint suppression, and runtime tests are repeated on the corrected final fixture.
Commands use offline Cargo,
`ci-test`, jobs 2 and command-scoped `CARGO_PROFILE_CI_TEST_DEBUG=0`. No persistent
runtime/build settings or debug assertions change. Existing shared-fixture
dead-code and unrelated NAPI warnings remain disclosed, not suppressed or fixed
outside scope. The strict 6 GiB disk reserve remains intact.

Native coverage is generated only in a disposable one-feature project, installed
as generated artifacts, and linked/rebased through native commands. Seven
scenarios map one-to-one to seven tests and the real header/auth/coordinator
implementation. `final-native-coverage.log` and
`final-native-review-and-format.log` contain audits, scoped Gherkin validation,
format/whitespace checks and native review with seven covered scenarios and no
findings. Scoped native tag validation passes after adding the registered
authentication feature-group tag; the earlier missing-tag failure is not a pass.
No coverage/work-unit JSON was hand-edited or gate bypassed.

## Direct review and exclusions

Reviewed all four actual callers of the shared header boundary, exact loaded
script ownership, preferred/legacy aliases, absent/fresh-token paths, Custom and
API-key/bearer preservation, canonical path locks, queued cancellation and
admitted callback lifetime, root checks around reads/waits/persistence, safe
returned diagnostics, unchanged HTTP/SSE settlement and ordinary usage. New and
touched focused source/test files remain under 300 lines. Native provider and
Codex policies are untouched. No new sessions, model changes, compaction actions,
SDKs, installed-script audit, real credentials, daemon/configuration changes,
manual commits or publication occur. Native Done is verified separately.
