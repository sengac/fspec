# BUG-274: existing facade and actual build integration verified

The real public `persistence::session_manifest_issues` and its default RPC caller
already exist. The current card title/description and feature architecture now
describe verification of this implementation, not an absent API. Original intake
title/description and rationale remain recoverable in architecture notes.

## Acceptance

**2/2 public-facade tests pass:** stable non-draining repeated diagnostics include
path/session identity, category, cause and recovery guidance; healthy and newly
created sessions remain available; damaged target loading stays strict; all source
bytes remain unchanged. A genuinely invalid sessions root returns a store-wide
error, never empty success. The shared BUG-271 isolation, strict cleanup and both
real RPC/TUI transport controls also pass.

Final isolated command from `rust/`, using the approved isolation variables and
`CARGO_PROFILE_CI_TEST_DEBUG=0`:

```
cargo test --offline --profile ci-test -j2 -p codelet-core --test bug271_manifest_isolation --test bug274_manifest_diagnostics
```

`/tmp/bug271-274-final-acceptance.log`: 7+2 passed, zero failed. No empty stub,
feature-only declaration, weakened target errors or cleanup bypass was added.

## Actual binaries, features and quality

All three actual builds succeed, rather than checking only consumer libraries:

```
cargo build --offline --profile ci-test -j2 -p codelet-rpc -p codelet-cli -p codelet-fspec
cargo build --offline --profile ci-test -j2 -p codelet-rpc --features test-fixture
cargo build --offline --profile ci-test -j2 -p codelet-fspec --features test-stub-provider
```

Seven-package default `cargo check` succeeds, including final Fspec. Default
seven-package Clippy, RPC/Fspec fixture-feature Clippy, owned acceptance-test
Clippy and the corrected durable-fixture Clippy all succeed with `-D warnings`.
Exact full commands, individual exit receipts and logs are in
`spec/attachments/BUG-272/bug271-272-274-validation-results.json`.

All final scoped tests: **120 passed across 35 targets**, including all 21
three-card acceptance scenarios. Five owned feature/tag/coverage checks,
one-test/current-source range assertions, owned rustfmt and diff checks pass.
All 2161 repository features validate. Global tags retain exactly 522 unrelated
pre-existing failing files; all five scoped features pass. The shared JSON records
the initial setup/timing/coverage failures and superseding results rather than
claiming every historical run passed.

## Historical scope and limits

`spec/attachments/BUG-273/bug273-final-validation.md` accurately records its six
historical library checks; it remains unchanged. This card adds current actual
RPC/final-binary and supported-feature evidence separately, without rewriting
past results. Controller live RPC receipts twice return identical successful
empty issue arrays: API availability only, not damaged-fixture proof or live
history repair. Original receipts are `/tmp/bug274-live-rpc-diagnostics-1.json`
and `-2.json`.

See `spec/attachments/BUG-271/bug271-272-274-safety-and-history.md` for the exact
authorized cleanup and fixture-model isolation correction. The actual user config
hash remains the verified value; no private config or backup is attached. No
current required check is blocked. No schema-cause, live repair, Windows/workspace
suite, deployment, commit, push, installation or daemon restart is claimed.
