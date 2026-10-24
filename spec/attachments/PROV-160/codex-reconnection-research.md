# PROV-160: Codex network recovery and reconnect presentation

Research date: 2026-10-20\
Work unit: PROV-160\
Scope: fspec Codex provider, shared API-profile recovery, and native Codex.

## Request and conclusion

The user wants Codex sessions to recover from temporary disconnections, such as
losing mobile reception, and expose a reconnect status consistent with fspec's
ordinary API profiles. Native Codex is the behavioral reference.

This is a recovery-policy gap, not merely a missing label. Fspec's Codex adapter
currently terminates on transport failures and interrupted accepted responses.
Its limited setup retries do not emit reconnect status. Native Codex separates
bounded response-stream recovery from potentially unbounded foreground
connection recovery and presents retry events as temporary working status.

No production changes were made during this investigation. Recommendations below
are proposed scope, not a claim that conflicting acceptance criteria have already
been superseded or that recovery has been implemented.

## Source identity and reproduction

Fspec source references below were inspected on 2026-10-20. Line numbers may drift.
The worktree already contained unrelated stream-compaction changes; these were
left untouched. Fspec HEAD at card creation was `e8bde659`.

The clone at `/tmp/codex` is incomplete: its normal checkout and Git metadata are
partly missing. Its index and packed objects remain readable. The research matched
the index to upstream commit:

- `bb64b894fd67158be0f874872e6d7bfe717e7368`
- Commit date: 2026-10-15 03:48:33 UTC
- Subject: `Retire worktree and voice feature flags (#54175)`

Upstream references in this document are relative to that commit's repository
root, not a claim that the corresponding checkout files currently exist.
Individual files were read from Git objects without restoring the checkout:

```sh
GIT_OBJECT_DIRECTORY=/tmp/codex/.git/objects \
  git --git-dir=/tmp/codex-review-git.M1MU3z \
  show bb64b894fd67158be0f874872e6d7bfe717e7368:codex-rs/core/src/responses_retry.rs
```

The temporary Git metadata path is environment-specific; it is not a project
dependency. Future work should obtain the pinned commit if these temporary files
are gone rather than silently substitute a newer upstream revision.

## Current fspec behavior

### Ordinary API-profile recovery

- `rust/cli/src/interactive/recovery_network.rs:18`: three consecutive network
  retries, with delays of 1, 2, and 4 seconds.
- `rust/cli/src/interactive/error_classifiers.rs:80`: string-based classification
  of connection, DNS, timeout, SSE, TLS, and unexpected-EOF errors.
- `rust/cli/src/interactive/stream_loop.rs:2090`: emits `⟳ Reconnecting...` on
  the first retry, preserves partial assistant text, then restarts the agent
  stream using history plus a synthetic `Continue` user message.
- `rust/cli/src/interactive/stream_loop.rs:894`: successful text, and analogous
  successful data branches, reset the retry counter and emit `✓ Reconnected`.
  Exhaustion emits `✗ Reconnection failed` before terminal error handling.
- `rust/agent-loop/src/background_output.rs:436`: `StreamEvent::Status` becomes
  an informational user-notification chunk. This is the existing presentation
  pipeline to reuse rather than routing provider outages through daemon recovery.

### Codex HTTP and SSE

- `rust/providers/src/codex/recovery.rs:7`: default network idle timeout is
  120 seconds; setup rejection retries are limited to two.
- `rust/providers/src/codex/recovery.rs:108`: HTTP setup transport failures and
  setup deadlines immediately return errors; they do not retry.
- `rust/providers/src/codex/recovery.rs:131`: an OAuth 401 can refresh credentials
  once. This is distinct from network reconnection.
- `rust/providers/src/codex/recovery.rs:154`: only setup statuses 429 and 503
  retry, honoring bounded `Retry-After` advice; local delays start at 250 ms
  and double, capped at 30 seconds. These waits emit no reconnect lifecycle.
- `rust/providers/src/codex/recovery.rs:177`: accepted response-body read errors
  and idle deadlines terminate instead of replaying.
- `rust/providers/src/codex/streaming.rs:152`: closes `GenericEventSource` on
  the first transport error or premature EOF, preventing helper-level retries.

The ordinary network classifier does not match the current Codex messages such
as `Codex network setup failed; request was not replayed`,
`Codex network idle deadline exceeded`, or
`Codex interrupted stream before terminal completion: EOF`.
Thus these paths do not reach the shared reconnect display. Although the older
feature docstring describes an explicit generic-recovery bypass, the inspected
shared loop has no blanket Codex-specific gate; current behavior should not be
documented as relying on a gate that is not there. Typed recovery ownership is
preferable to making these errors accidentally match generic retry strings.

### Codex WebSocket and history safety

- `rust/providers/src/codex/websocket.rs:129`: setup errors fail; rejected
  upgrades have typed `WebSocketUpgradeRejected` diagnostics.
- `rust/providers/src/codex/websocket.rs:163`: send failures and deadlines fail
  without replay; receive closure/errors also terminate at line 206.
- `rust/cli/src/interactive/request_failure.rs:4`: typed upgrade/checkpoint
  failures are handled before generic recovery.
- `rust/providers/src/codex/streaming.rs:206`: commits successful response
  history before yielding buffered tool calls for execution. Existing safeguards
  must survive recovery; generated partial tool calls must not execute on retry.
- `rust/providers/src/codex/tool_dispatch.rs:15`: deduplicates item/call IDs
  within output state. This alone is not proof of cross-attempt exactly-once
  execution and should not be treated as such.

The separate client-to-daemon reconnect machinery in
`rust/fspec-tui/src/app/dispatch_reconnect.rs:18` is not provider recovery.
A provider outage should not falsely signal that the daemon disconnected.

## Native Codex reference behavior

All paths in this section refer to upstream commit `bb64b894fd67`.

### Separate retry layers

- `codex-rs/model-provider-info/src/lib.rs:66`: defaults are four HTTP request
  retries, five response-stream retries, a 300-second stream idle timeout, and
  a 15-second WebSocket connect timeout. These are reference defaults, not
  automatically approved fspec configuration changes.
- `codex-rs/codex-client/src/retry.rs:23`: request retries wrap opening the HTTP
  response, not consumption of its accepted stream. Default four retries mean
  five total sends. Local backoff starts at 200 ms with exponential growth and
  jitter; server advice can replace local timing.
- `codex-rs/core/src/responses_retry.rs:141`: eligible response-stream failures
  use the bounded stream budget and emit `Reconnecting... N/5`. Server advice
  controls timing without increasing the configured budget. The first WebSocket
  retry notification can be suppressed in release builds.

### Waiting for mobile reception to return

`codex-rs/core/src/responses_retry.rs:93` implements a distinct connection-failure
path: 5, 10, 20, 40, then 60 seconds repeatedly, without consuming bounded stream
retries. It emits `Reconnecting... waiting for network`.

The `unbounded_connection_retries` feature is stable and default-enabled
(`codex-rs/features/src/lib.rs:1432`). Eligibility is restricted to sampling,
non-internal sessions, and non-Bedrock providers; it is not an instruction to
retry every error or every background operation forever.

`codex-rs/codex-api/src/api_bridge.rs:265` distinguishes connection failures from
timeouts and other stream/network failures. An already-accepted stream dropping
is initially a bounded stream failure, not automatically an unbounded offline
failure. A subsequent eligible connection failure can enter the offline path.

### Request reconstruction and tool effects

`codex-rs/core/src/session/turn.rs:1740` rebuilds retry prompts from current
conversation history and attaches executed-tool information. It does not append
a synthetic `Continue` user message or claim to resume at a byte offset.

Completed items and tool calls are recorded as they arrive
(`codex-rs/core/src/stream_events_utils.rs:93`). Pending tool futures are drained
and successful results recorded before returning a stream failure to recovery
(`codex-rs/core/src/session/turn.rs:3272`). Recovery therefore preserves completed
work rather than blindly treating the failed exchange as never having happened.
This does not establish exactly-once guarantees for arbitrary remote side effects.

### Presentation and clearing

- `codex-rs/core/src/session/mod.rs:5318`: emits structured `StreamError` events
  with retry text and error details; the app-server marks retry notifications
  with `will_retry: true`.
- `codex-rs/tui/src/chatwidget/streaming.rs:398`: saves the previous working
  header and temporarily displays retry text/details in the status indicator.
- `codex-rs/tui/src/chatwidget/protocol.rs:36`: the next qualifying notification
  restores the previous header and clears retry details. Replayed history and
  repeated retry-error notifications do not incorrectly restore it.

Native Codex does not require an explicit recovered event or a persistent
`Reconnected` transcript entry. Fspec can retain its familiar notification wording
while adopting real recovery and explicit lifecycle semantics.

### Transport fallback is a separate policy

Upstream can switch WebSocket to HTTPS after bounded retry exhaustion, reset the
stream budget, and retain HTTP for the session
(`codex-rs/core/src/responses_retry.rs:120`). This is intentionally different from
fspec's current selected-transport contract. Do not copy that behavior implicitly.

## Existing specifications and required decisions

1. `spec/features/codex-stream-recovery.feature:9` prohibits replay of ambiguous
   setup failures and accepted responses. Its accepted-disconnect scenario
   explicitly expects no automatic replacement request.
2. `spec/features/codex-explicit-transport.feature:1` (CMPCT-081) prohibits silent
   transport downgrade and delta-to-full fallback. Its accepted-WebSocket-failure
   scenario also prohibits replacement WebSocket requests.
3. `spec/features/codex-websocket-transport.feature:1` contains older fallback and
   incremental-mode language that conflicts with the newer explicit-transport
   contract. Do not mistake it for current authorization to restore fallback.
4. `spec/features/codex-network-idle-deadline.feature:1` requires raw network
   progress, not UI ticks, to reset deadlines; local tools must not time out.
5. `spec/features/sse-disconnection-retry.feature:1` describes the generic
   three-retry/Continue policy, not native Codex's offline-wait semantics.

Revise accepted-disconnect/no-replacement criteria explicitly before implementing
safe same-transport recovery. Preserve full-current-DAG requests, selected
transport, committed generation, admission, credential authority, and checkpoint
validation. Recovery is not permission to use stale history or bypass rejection.

Confirm foreground versus internal-session eligibility, configurable budgets and
deadlines, and the desired fspec reconnect-status lifecycle during specification.
Recommended scope retains the selected transport and leaves existing ordinary
provider behavior unchanged.

## Suggested implementation and acceptance coverage

Implement one Codex-owned, typed retry policy with explicit reconnecting,
recovered, exhausted, and cancelled signals surfaced through fspec's existing
status pipeline. Avoid independent retries in both the adapter and generic loop.

Follow ACDD: update feature scenarios, observe meaningful failing behavioral
tests, then implement. Cover at least:

- Connection loss lasting longer than the generic seven-second backoff window,
  followed by successful recovery with the same user turn and selected transport.
- Accepted SSE and WebSocket disconnects, partial text/reasoning, premature EOF,
  and idle deadlines; terminal completion/checkpoint publication only on success.
- Bounded stream exhaustion versus eligible foreground offline waiting; internal
  requests remain bounded and permanent/auth/checkpoint failures stay terminal.
- Completed tool calls/results remain available; pending/partial tool calls do
  not cause repeated execution; no synthetic `Continue` user messages.
- Reconnect status is session-scoped, reaches the TUI/CLI pipeline, clears on
  real recovery, and does not masquerade as daemon disconnection.
- Cancellation during connection setup, body reads, and long backoff stops
  promptly with no subsequent request or orphaned recovery task.
- Server retry advice, credential refresh, account/generation changes, raw-byte
  idle tracking, and unchanged ordinary-provider behavior.

## Validation evidence and limitations

This was source/specification research, not a live mobile-network reproduction.
No successful reconnection behavior is claimed for the current Codex adapter.

Existing focused tests include `rust/providers/tests/prov154_codex_recovery.rs`,
`rust/providers/tests/prov154_codex_deadline.rs`, and
`rust/cli/tests/network_retry_test.rs`. In particular, the accepted-stream test
currently asserts no replay, rather than demonstrating interruption recovery.

A targeted run of the two PROV-154 provider integration targets was attempted
with debug info disabled and four build jobs. It triggered a cold dependency
rebuild and was interrupted before test execution. There is no fresh pass/fail
result. Its temporary log was `/tmp/fspec-codex-reconnect-investigation-tests.log`.
No workspace-wide test command was run and no production code was modified.
