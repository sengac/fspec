# PROV-159 implementation checkpoint

This is an implementation/verification checkpoint, not a request for more upstream research.
The shared-authority refactor was approved in work-unit note 5.

## Focused audit and actions

- Storage: preserve the previous file and preferred keychain credentials when either mutation fails; do not let malformed fallback JSON mask valid preferred credentials.
- Client binding: reject mismatched persisted credentials rather than silently constructing an unbound client. Storage-loaded providers require a valid generation binding.
- Refresh: initialize shared/legacy refresh callers from persisted freshness; treat a positively expired JWT as expired even with a recent last-refresh timestamp; publish rotated tokens and expiry in one document replacement.
- Login: fence delayed commits against durable account/generation transitions, including shared-home writers; make persistence-lock acquisition cancellable.
- Device flow: stale polling must not consume a replacement attempt; expire abandoned attempts and preserve one overall operation budget.
- Transport: extend the interactive deadline to device start as well as browser/poll; check authorization at the send boundary and discard revoked socket state.
- Regression fixture: keep WebSocket probes on loopback paths that are not rewritten to the production Codex endpoint.

## Confirmed red evidence from this continuation

`/tmp/prov159-policy-red.log`: three new behavioral regressions fail before their fixes:

1. A4: a stale client constructed after another account is stored sends the old credentials.
2. A14: the shared refresh entry point unnecessarily calls the issuer despite fresh persisted expiry.
3. A6: an externally rotated, expired JWT gains apparent freshness from last_refresh and skips refresh.

Additional storage, lifecycle, entry-point and transport tests run in isolated homes with loopback issuers.

## Workflow bookkeeping

The initial testing-to-implementing transition was rejected by fspec's existing
1-feature/1-test-file gate. This was resolved by separating PROV-159 scenarios
into dedicated features grouped by physical test target, retaining historical
coverage in the existing OAuth features. No gate bypass was used.

## Final validation — 2026-10-17

- All 45 scenarios across 15 dedicated PROV-159 features have test and implementation
  mappings. `fspec audit-coverage` passes for every dedicated feature.
- All 48 tests across the 11 provider and four TUI PROV-159 integration targets pass.
  These include real embedded tarpc cancellation, callback-port reuse, authorization
  beyond the default ten-second RPC deadline, account/generation fences, safe
  storage failures, and HTTP/SSE/WebSocket revocation after credential deletion.
- The final two regression tests failed before their fixes in
  `/tmp/prov159-resume-red.log` and pass in `/tmp/prov159-resume-green.log`.
- Existing provider OAuth/refresh tests and TUI PROV-112/113/114 compatibility tests
  pass. The three agent-loop `prov149_codex_reload` tests pass.
- Cross-crate `cargo check` and `cargo clippy` pass for providers, RPC, NAPI, TUI,
  CLI, sessions and agent-loop. Task-owned Rust files pass targeted rustfmt checks.
- `fspec validate` passes for all 1,847 feature files. Global `validate-tags` reports
  526 unrelated existing feature violations; none names a changed feature or any
  PROV-159 feature. Crate-wide formatting also reports unrelated existing files.
- Broad provider validation exposes three unrelated failing targets, not a green
  package-wide suite: `prov094_codex_bundle` references removed Codex templates;
  `prov156_codex_websocket` expects an empty continuation input; both reproduce
  unchanged in the archived baseline at `/tmp/prov159-baseline.WUjbMZ`
  (`/tmp/prov159-baseline-final.log`). The live
  `codex_gpt6_astra_live_test::live_gpt6_astra_non_streaming_is_rejected` expects
  the service to reject non-streaming but instead receives `pong`. It is an
  external-service expectation, not deterministic lifecycle evidence; it was not
  rerun with real credentials during this continuation.

Detailed local logs: `/tmp/prov159-final-provider-suite.log`,
`/tmp/prov159-final-tui-suite.log`, `/tmp/prov159-final-reload.log`,
`/tmp/prov159-final-check.log`, `/tmp/prov159-final-clippy.log`,
`/tmp/prov159-card-coverage.log`, `/tmp/prov159-final-validate.log`, and
`/tmp/prov159-final-tags.log`. These temporary logs are supporting evidence;
the results and known limitations are retained here for version control.
