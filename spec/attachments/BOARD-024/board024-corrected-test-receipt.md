# BOARD-024 corrected test receipt

Scope: BOARD-024 integration tests and their own coverage sidecar only. No
production, REFAC metadata, shared helpers, protected BUG-283 files, statuses,
builds, test executions, restarts, or commits changed/run by this worker.

## Corrections

- canonical.rs now uses actual architecture note objects with stable IDs,
  deleted flags and createdAt; stateHistory.state/timestamp/reason; legacy
  statusHistory.from/to/timestamp/reason; eventStorm.items/level/nextItemId with
  actual typed/color/deleted event records. Rules/examples/questions also carry
  their real discovery item metadata, and virtual hooks include blocking.
- Canonical completeness now includes assumptions, blockedBy and relatesTo.
  Retired event discovery and all omitted markers are asserted across both
  transports. Noncanonical shapes are deliberately isolated under the unknown
  legacyCompatibility field, not mislabeled as representative current fields.
- Held requests require exactly one typed details completion, exact card ID,
  expected success/error result kind, and successful raw JSON record ID. Same-ID
  reopen and different-card requests must have different generations before
  any delayed action is delivered. Arbitrary nonempty Action queues cannot pass.
- Attachment activation exercises three separate outcomes using the existing
  set_viewer_port_for_test/attachment_url path: supported port plus successful
  opener exit, unavailable viewer with no opener invocation, and actual failing
  opener exit. The production open::that path executes against a private PATH
  containing only a logging xdg-open shell fixture; host browser commands are
  never available to it. This fixture explicitly requires the Linux command
  protocol and returns an error, rather than launching a browser, elsewhere.
- Exact encoded URL and exact original OpenAttachment path are checked for both
  duplicate basenames. Reactivating after each outcome must preserve that exact
  path, proving selection retention beyond a potentially ambiguous basename.
- Visible errors must occupy an actual Error: region containing the target and
  the correct failure category; unrelated body words cannot satisfy the check.
  Invalid/missing/directory targets use an available viewer and prove no opener
  invocation, so viewer unavailability cannot mask validation failures.
- Rendered selection is discovered from actual inverse/accent cell styling and
  can span wrapped rows; no particular arrow glyph or full-path-on-one-row is
  required. Link hit coordinates come from the current rendered buffer. The
  stale after-scroll click is explicitly in bounds and must remain inert.
- Modal isolation now unwinds details -> chooser -> original Board/Mux, checking
  layout, pane focus, selected card, current session and original association.
  A subsequent ProviderSettings open/close must return to that same origin,
  exercising the preserved BUG-283 origin behavior without editing its files.

## Explicit typed API obligation

The corrected tests intentionally stage the following missing Action contract:

```rust
BoardCardDetailsLoaded {
    request_generation: u64,
    work_unit_id: String,
    result: Result<String, String>,
}
```

The successful string is the full canonical record JSON, not a truncated board
summary. Every request/reopen/navigation generation must be distinct. Details
must reject stale success and error completions without replacing loading,
resurrecting dismissed dialogs or changing another card. No such variant exists
in current production source by static inspection. If production chooses a
different explicit typed API, adapt these exact typed assertions to it; do not
restore generic queue/string-count checks.

Visible failures must name their card/attachment target and category in an
Error: region. Supported browser failure must propagate into that region;
tracing-only warnings are not sufficient. Successful launching and unavailable
viewer behavior are distinct requirements.

## Validation and ACDD correspondence

- Scoped rustfmt --check and git diff --check pass.
- All 8 BOARD Rust sources remain below 300 physical lines; maximum 287.
- No unwrap(), panic!(), or new allow-lint directives occur in these sources.
- All 22 protected hashes match /tmp/board024-controller-baseline/protected.json.
- All 17 original Gherkin scenario names and Rust scenario function names remain.
- 10 changed test ranges were refreshed using 10 native unlink-coverage and
  10 native link-coverage operations, with mandatory step validation and no skip
  flags. The other 7 mappings remain unchanged. Native show-coverage readback
  confirms 17 scenarios, one test mapping each, 4 scenario test files, 880 mapped
  test lines and zero implementation mappings. This is traceability, NOT GREEN.

## Parent-owned causal run required before behavior production

From rust/:

```bash
CARGO_NET_OFFLINE=true CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-tui --test board024_enter_chooser -j 2 -- --test-threads=1
```

This worker did not execute the command because the parent exclusively owns
builds/tests. First capture the missing typed-API compiler RED if it remains
absent; after introducing only the explicit completion contract, rerun while
chooser/details behavior is still absent to establish corrected behavioral RED.
These are required future observations, not reported test results.

The earlier 1-pass/16-fail receipt proves only missing chooser/footer entry.
It does not execute the corrected canonical, generation, attachment, scrolling,
Mux unwind, or WebSocket assertions. Run and inspect every deeper assertion once
reachable; do not infer GREEN or feature completion from mappings or the old RED.
