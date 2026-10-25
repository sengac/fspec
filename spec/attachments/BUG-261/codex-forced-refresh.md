# BUG-261: Forced Codex refresh loses network recovery

## Finding

Source-confirmed at `f0eb0759`, introduced in the `2699693b` recovery path.
The post-401 forced refresh erases typed network failures and timeout identity,
converting issuer outages into terminal sign-in-again messages.

## Evidence

- `rust/providers/src/codex/recovery_auth.rs:25` preserves typed setup failures.
- `rust/providers/src/codex/recovery.rs:92` propagates proactive refresh failures.
- `rust/providers/src/codex/recovery.rs:150` and line 151 replace forced-refresh
  timeout and failure with untyped strings.
- `rust/providers/src/codex/reconnect.rs:177` retries only downcastable NetworkFailure.
- `rust/providers/tests/prov154_codex_recovery.rs:26` covers successful/rejected
  refresh; `rust/providers/tests/codex_recovery_boundaries.rs:63` covers proactive
  issuer-offline refresh, not 401 followed by issuer outage.

## Reproduction and expected behavior

Use a fake provider returning 401 and an issuer that fails DNS/connect/setup
or exceeds its deadline. The current forced-refresh path should demonstrate
terminal authentication failure. Add a red test before changing it.

Preserve retryable causes into Codex recovery while keeping true credential
rejection terminal and actionable. Verify recovery status, bounded attempts,
cancellation, refresh races and absence of credential leakage. Compare proactive
and post-401 behavior using real HTTP boundaries, not helper-only calls.

Related contracts: PROV-154, PROV-159, PROV-160.
No live outage was independently reproduced in this audit.
See [full audit](../CONT-011/fspec-auto-continue-investigation.md).
