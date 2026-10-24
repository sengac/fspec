# BUG-237 ACDD evidence

Feature written first: `spec/features/cached-browser-recovery.feature`.
Example Mapping rule/example captured through fspec before the feature.

## RED (2026-10-22)

`cargo test --profile ci-test -p codelet-tools --lib web_search::recovery_tests -- --ignored --test-threads=1`
Full output: `/tmp/bug237-red.log`.

Seven installed-Chrome behavioral tests: three passed, four failed:
- launched idle-expired browser returned the same stale cached Arc;
- connected idle-expired browser returned the same stale cached Arc;
- screenshot disconnect attempted only once instead of twice;
- late concurrent stale failure replaced a newer healthy browser.

Both one-second-idle reproduction tests observed:
`Failed to create new tab: Unable to make method calls because underlying connection is closed`.
This exact error already matches the old retry classifier for tab operations;
the original incident therefore does not prove a subsequent WebSearch failure.
The demonstrated defects are stale cache admission, omitted screenshot recovery,
and unconditional cache invalidation racing with a newer generation.

The healthy-reuse, healthy-page-error, and visible-mode cases already passed.
Tests explicitly close the cache; ordinary object-drop ownership cleans up all
launched Chrome processes, including failed assertions. Connected Chrome remains
owned by its launcher.

## Implementation and GREEN

Cache mode and browser generation now share one lock. Cached transports are
probed before reuse; a failed generation is replaced conditionally so a late
caller reuses a newer healthy replacement instead of evicting it. Creation uses
the previous Chrome configuration, including explicit websocket connections.
Closed screenshot transports join the existing tab/navigation/script recovery
policy. Retry remains bounded to one attempt, and healthy page errors do not
relaunch Chrome. DEBUG marks replacement/retry; a failed retry remains WARN.

After the initial real-Chrome RED run, five policy cases were migrated to the
same production cache implementation with deterministic factories/probes so
they run without Chrome. The two expiry cases retain real installed-Chrome
coverage, verified one-second timeout and direct WebSearch retry/mode wiring.
RAII cleanup guards also run when an assertion fails.

- `/tmp/bug237-policy-green.log`: five deterministic policy tests passed.
- `/tmp/bug237-final-browser-green.log`: 35 WebSearch unit/regression tests passed,
  including both installed-Chrome expiry cases.
- `/tmp/bug237-browser-regressions.log`: three existing Chrome integration tests
  passed; seven pre-existing live-web/remote-browser tests remained ignored.
- `/tmp/bug237-check.log`: scoped cargo check passed.

The original seven scenarios were split without behavioral changes into two
features to satisfy fspec's one-feature/one-test-file gate:

| Feature | Scenarios | Canonical test file |
| --- | --- | --- |
| cached-browser-recovery.feature | healthy reuse, bounded recovery, page errors, concurrent replacement, visible mode | rust/tools/tests/browser_cache_policy.rs |
| idle-browser-transport-recovery.feature | launched expiry, connected expiry | rust/tools/src/browser_recovery_tests.rs |

All seven mappings include exact @step comments and implementation links.

## Final quality checks

- `/tmp/bug237-policy-final.log`: five deterministic policy tests passed after
  lint cleanup; no test-lint suppression was added.
- `/tmp/bug237-clippy-scoped.log`: cargo clippy passed for codelet-tools library
  and browser_cache_policy target with --no-deps (selected-crate lint scope).
- `/tmp/bug237-238-scoped-spec-check.log`: all three BUG-237/238 features pass
  fspec syntax, tag/category, and formatting checks with the real tag registry.
- Both feature coverage audits pass, with 5/5 and 2/2 scenarios covered and
  one canonical test mapping per scenario.
- Targeted rustfmt checks pass; all new Rust files are below 300 lines.

Global Gherkin validation passes all 2062 features. Global tag validation/check
reports 527 unrelated existing feature violations; no BUG-237/238 feature is
among them. Unscoped dependency linting initially encountered the parallel
BUG-239 worker's new codelet-common privacy regex expects; the selected-crate
lint run above passes independently. No unrelated production files were fixed.

BUG-237 is **done** through the full fspec lifecycle. Both features carry @done
and pass `/tmp/bug237-238-final-scoped-spec-check.log` with the repository's
tag registry. No commits or branches were created.
