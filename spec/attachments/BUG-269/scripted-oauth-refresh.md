# BUG-269: Scripted OAuth refresh helper is not in the request path

## Finding and limits

Source-confirmed integration gap, not a live credential failure reproduced by
the audit. Scripts that perform their own refresh in headers are not evidence
of this failure and must not be forced into duplicate refresh.

## Evidence

- `rust/providers/src/oauth/scripted_refreshing_client.rs:58` exposes
  ensure_fresh_if_needed, but the audit found only test callers.
- `rust/providers/src/custom/rig_streaming.rs:36` obtains script headers directly.
- `rust/providers/src/custom/provider.rs:138` and `custom/http.rs:94` use the
  plain HTTP client path, not the documented refresh wrapper.
- Existing device-flow refresh tests invoke the helper directly, not an actual
  expired-token completion request. The integration originates in `66682746`.

## Acceptance/test direction

Use a real custom provider script with expired synthetic credentials and a fake
issuer; issue actual streaming and nonstreaming completion requests. Confirm
refresh happens before authorization is sent and the continued request succeeds.
Test rejected refresh, transient network failure, concurrent callers, cancellation,
deadlines and safe diagnostics. Preserve global/project provider resolution and
scripts that intentionally own their refresh policy.

This is separate from dedicated Codex forced-refresh error erasure (BUG-261).
Related contracts: PROV-088 and scriptable-subscription-providers.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
