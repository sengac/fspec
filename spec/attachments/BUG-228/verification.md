# BUG-228 verification

The feature now requires one armed shared dispatch implementation and NAPI
delegation, not duplicated frontend loops. No production code changes were needed.
The obsolete source-shape regression failed with a missing NAPI
`BackgroundOutput::with_provider` dispatch site before the test repair.

The replacement checks real frontend exports, shared lock/sync/output ordering,
the single helper definition, and absence of inline arming. Mutation controls
reject missing delegation, missing sync, a local frontend loop, and duplicate
arming. Six existing real-session behavioral tests remain intact. Moving the
wiring test into its own target keeps the behavioral file under 300 lines.

From `rust/`, with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`:

- Red: `cargo test -p codelet-sessions --test cont009_completion_contract_sync both_twins_call_shared_sync_helper_at_dispatch_site -j 2`
- Green: `cargo test -p codelet-sessions --test cont009_completion_contract_sync --test bug228_shared_dispatch_wiring -j 2` (7 passed)
- Check: `cargo check -p codelet-sessions -j 2`
- Clippy: `cargo clippy -p codelet-sessions --test cont009_completion_contract_sync --test bug228_shared_dispatch_wiring -j 2`

Logs: `/tmp/fspec-bug228-{red,green,check,clippy}.log`.
Feature syntax, mapped coverage audit, changed-file rustfmt, and fspec review pass.
Coverage is 7/7 scenarios. Example Mapping was explicitly recorded during a
review-driven return to specifying; it documents the prior research-derived
contract rather than pretending those board entries preceded the first red run.
The missing shared-helper call and frontend delegation are tested independently;
source wiring alone is not claimed as a physical frontend end-to-end execution.
