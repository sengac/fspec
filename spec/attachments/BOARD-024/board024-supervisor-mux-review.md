# BOARD-024 supervisor review follow-up: Mux focus and coverage

This supplements all previous receipts without replacing them. Production remains
paused pending the refactor/oversized-file decision; canonical state is testing.

## Mux assertion correction

Reviewed the actual current `focus_agent_pane_for_session` and `bump_focus`
implementations read-only. Live pane focus is separate from the saved config's
focus preference today, so full MuxConfig equality may happen to pass with the
current implementation. It is nevertheless an unnecessary future constraint on
correct entry behavior and does not express the required layout invariant.

The Mux test now:

- Retains full config and live focus equality after chooser cancellation.
- After confirmation, compares only orientation, splits, pane kinds and enabled
  state through `assert_mux_layout_preserved`.
- Separately asserts live focus changed from Board and the focused session is
  the earliest associated agent, while active view remains Mux.

Only BOARD-024 test/support files changed. File lengths remain below 300:
chooser.rs 230 lines and support.rs 213 lines.

## Coverage verification

The supported `show-coverage` command verified the actual 17 scenarios and zero
placeholder entries before this correction. The earlier placeholder snapshot was
outdated; the prior follow-up had already refreshed and linked all scenarios.
After the one-line Mux test expansion, supported unlink-coverage/link-coverage
calls updated the two affected final ranges:

- Mux scenario: chooser.rs:163-209.
- Empty-board scenario: chooser.rs:211-230.

There remains exactly one test mapping per scenario, with no implementation
mappings. Mapping coverage is not passing behavior or completed implementation.

## Revalidated RED

Same scoped command as the prior supplement, from rust/:

```bash
CARGO_NET_OFFLINE=true CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_CI_TEST_DEBUG=0 cargo test --profile ci-test -p codelet-fspec-tui --test board024_enter_chooser -j 2 -- --test-threads=1
```

Raw receipt: `board024-red-mux-review.log`.

```text
Finished `ci-test` profile [unoptimized] target(s) in 6.64s
test result: FAILED. 1 passed; 16 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.17s
Cargo exit status: 101
```

Scoped rustfmt --check passed for the two changed files; shared metadata diff
whitespace check also passed. No production files or BUG-283 files were edited.

Evidence boundary is unchanged: 15 failures establish missing chooser entry and
one establishes the missing footer hint. The corrected post-confirmation Mux
layout assertion and deeper detail assertions are not reached yet. No claim is
made that this RED exercised those later assertions.
