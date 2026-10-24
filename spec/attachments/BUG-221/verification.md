# BUG-221 verification

The shared stream loop now recognizes captured thinking with no visible text or
tool progress at a length stop even when numerical reasoning usage is absent.
It retains the existing numerical detection path and bounded retry counter,
does not invent usage, and refuses recovery after interruption.

The feature `thinking-only-exhaustion-recovery.feature` precedes the integration
target `rust/cli/tests/bug221_thinking_only_exhaustion.rs`; its support fixture
uses local physical streams and real session history. Scenario mappings and
implementation mappings are recorded in the feature coverage sidecar.

Commands from `rust/` use `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`:

- Red/green: `cargo test -p codelet-cli --test bug221_thinking_only_exhaustion -j 2`
- Regression: `cargo test -p codelet-cli --test thinking_exhaustion_recovery_test -j 2`
- Check: `cargo check -p codelet-cli -j 2`
- Clippy: `cargo clippy -p codelet-cli --test bug221_thinking_only_exhaustion -j 2`

Evidence logs are `/tmp/bug221-red.log`, `/tmp/bug221-green.log`,
`/tmp/bug221-thinking-regression.log`, `/tmp/bug221-check.log`,
`/tmp/bug221-clippy.log`, and `/tmp/bug221-coverage.log`.
The earlier incident's wire stop reason remains unproven; these regressions
establish the conditional recovery defect, not retrospective incident certainty.
Actual retry settings and truthful policy messages are separate BUG-222 work.
