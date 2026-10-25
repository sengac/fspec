# BUG-261 validation and direct review

The forced-refresh repair preserves the original typed issuer failure and maps
credential setup/refresh deadlines to `NetworkFailure::SetupDeadline`. Credential
rejection and malformed issuer replies remain terminal. Token synchronization,
binding, stale refresh protection, cancellation and existing retry limits remain
unchanged. The shipping OAuth endpoint rewrite is unchanged.

## Authorization and isolation

Existing native approval c5e9fdf3-4008-4453-9a43-4c4c761e3659 authorized the
nondefault test-support-only numeric-loopback HTTP seam and BUG-262 module split.
Existing native approval 8b77846d-1240-434b-848b-634f91d9029c authorized fallible
test-fixture repair and affected callers. No approval is fabricated here.
The fixtures use disposable CODEX_HOME, a process mock credential builder and
synthetic tokens. The seam rejects hostnames/nonloopback/HTTPS/userinfo/fragments;
default builds omit its injectable field and methods. Issuer redirects remain
disabled. No production credential configuration was needed.

## Evidence chronology

- `red-ci-test.log` is INVALID FIXTURE EVIDENCE. The original bare /responses
  fixture was rewritten toward the real Codex endpoint, and local requests were
  zero. Synthetic tokens were used; actual external transmission is UNKNOWN.
  No successful inference, billing, real credential use or absence of transmission
  is claimed. The unsafe prototype was removed and never rerun.
- `seam-isolation-verified.log` passed before the full-model causal red.
- `red-http-cause.log`: genuine local HTTP failure proves erased typed cause.
- `red-model-local.log`: 3 failed / 2 passed against actual loopback 401/issuer.
- `green-model-local.log`: 4 passed / 1 timing failure; not the final result.
  Bounded fixture deadlines were corrected without removing assertions.
- `green-fixture-matrix-verified.log`: FINAL shared-fixture source, 6 BUG-261
  tests + 4 credential storage + 3 review regressions = 13 passed.
- `clippy-fixture-comparison.log` established the old fixture's three preexisting
  expect violations. `clippy-verified.log` passes after the authorized fallible
  fixture repair. `check-shipping.log` passes the default-feature provider build.
- Replacement-session `final-regressions.log` stopped after 2 passed / 1 failed:
  the preexisting 150 ms proactive offline assertion observed no status yet.
  No source was changed to conceal it. A single isolated confirmation on the
  identical source (`final-regressions-boundaries-confirmation.log`) passed all
  3 boundary tests. This timing sensitivity is a retained warning, not a claimed
  clean first run.
- `final-regressions-other-targets.log` passes all remaining required final-source
  targets: prov154_codex_recovery 4, prov159_refresh_completion 2,
  prov159_refresh_races 5. Together with the confirmation: 14 passed.

## Direct review and scoped quality

Reviewed recovery.rs, refreshing_client.rs, client_http.rs, client_test_endpoint.rs,
the six-case test and transport helper, shared credential fixture and its three
affected callers. Error sources survive the full model and HTTP boundary;
actual request counts, five-retry bound, cancellation and terminal credential
errors are asserted. All touched Rust files are below 300 lines. Scoped rustfmt
check and git diff --check pass. Native feature syntax, registered tags and
coverage audit pass; all six scenarios have current test/implementation links.
No lint suppression, unsafe block, credential policy change or production URL
relaxation was introduced. Existing unrelated edits are preserved.

The previously held blocked transition (P(hold)=0.43408555, argmax hold) was not
retried or bypassed. The card continued through its subsequently approved valid
workflow; final status must be read from the native work-unit tool.
