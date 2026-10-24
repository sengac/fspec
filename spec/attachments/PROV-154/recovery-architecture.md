# PROV-154 integration design

## Boundary

`codex/recovery.rs` owns HTTP setup and wraps the accepted raw-byte stream.
The Codex model's existing `GenericEventSource` receives the wrapped stream;
the model closes its source on the first error and therefore does not invoke
generic SSE reconnection. No shared rig defaults change.

## Deadline and cancellation

The default deadline is 120 seconds, independently scoped to credential setup,
HTTP response setup, and each raw network read. A per-client duration builder
supports deterministic short integration tests. Network bytes, including SSE
comments, count as progress; UI polling cannot recreate the inner timer.
The async stream owns the timer while yielded `next()` futures are dropped.
After successful terminal completion the stream is closed before tools execute,
so tools cannot be mistaken for network stalls. The CLI Codex branch delegates
deadlines to transport instead of retaining its generic UI-chunk deadline.

All setup, refresh and backoff futures remain owned by the request; no detached
tasks perform retries. Existing CLI cancellation drops the request/stream and
retains accumulated text. Tests cancel pending setup and retry sleeps and assert
that the local endpoint receives no later requests.

## Retry safety

Only explicit rejected setup statuses 429 and 503 are retryable, with two retries.
Retry-After seconds or HTTP date are honored, capped at 30 seconds per delay;
otherwise use 250ms and 500ms. Ambiguous connection errors, accepted response
disconnects, and permanent failures never retry. One OAuth 401 refresh is allowed
per request, comparing the rejected bearer token to current shared state to avoid
invalidating credentials another request already refreshed. No accepted body or
function output is replayed automatically. Explicit protocol continuation remains
PROV-148's responsibility, distinct from setup recovery.

Successful response headers remain intact for PROV-153. Error bodies are not
embedded in recovery errors because a server may echo credentials. OAuth redirects
must be disabled, including refresh requests, to prevent replaying account headers
or refresh-token request bodies across hosts.

## Shared seams awaiting approval

- `refreshing_client.rs`: extract token helpers into a small sibling module and
  delegate `send_streaming` to recovery. Existing file is 319 lines.
- `interactive/stream_loop.rs`: Codex-only bypass of generic error retries and
  UI-chunk stall timers. Keep all non-Codex behavior unchanged.

## Red evidence

`cargo test -p codelet-providers --test prov154_codex_recovery -j 2`:
unauthorized setup fails before refresh; 429 has one request instead of three;
cancellation-backoff case returns immediately instead of waiting. Accepted body
no-replay control passes. Deadline tests initially require the missing builder.
