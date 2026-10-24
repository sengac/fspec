# BUG-237: Browser recovery after idle transport expiry

Research date: 2026-10-22. HITL-approved recovery investigation. Upstream idle-timeout
behavior is verified from source; failure of a subsequent tool call is not established.
Source references describe the triage snapshot, including dependency version 1.0.21.

## Context and impact

The browser transport and event loops exited after idle receive deadlines expired.
The application caches a browser without a liveness probe and recognizes only two
closed-connection strings for retry. A stale cached instance may therefore affect a
later WebSearch operation. This must be reproduced rather than inferred from a
background shutdown message alone.

## Incident evidence

Log: `~/.fspec/logs/fspec-combined.log.2026-10-22:484` through `:493`.
UTC time: 01:22:27; Chrome process marker: `40029`.

1. Transport: `Transport loop got a timeout while listening for messages`.
2. Browser: `Got a timeout while listening for browser events`.
3. Both loops finish; listeners and tab loops are cleared.
4. Teardown: `Couldn't send message to transport loop telling it to shut down`.

Treat these as one lifecycle incident. The final warning is secondary shutdown noise,
not evidence of a separate root cause or proof that Chrome crashed.

## Verified upstream behavior

Dependency source is available in the local Cargo registry under
`headless_chrome-1.0.21/src/`:

- `browser/transport/mod.rs:247`: `messages_rx.recv_timeout(idle_browser_timeout)`
  returns `RecvTimeoutError::Timeout`, logs the exact transport error, and exits.
- `browser/mod.rs:371`: `events_rx.recv_timeout(idle_browser_timeout)` follows the
  equivalent timeout/exit path for browser events.
- `browser/transport/web_socket_connection.rs:144` emits the failed shutdown-send warning.

This establishes expiry of idle receive deadlines. It does not establish the precise
configured timeout at incident time or the success/failure of a later browser operation.

## Application ownership

- `rust/tools/src/chrome_browser.rs:77` defaults `CODELET_CHROME_TIMEOUT` to 300 seconds.
- `rust/tools/src/chrome_browser.rs:106` applies the timeout when connecting;
  `rust/tools/src/chrome_browser.rs:112` sets `idle_browser_timeout` for launched Chrome.
- `rust/tools/src/web_search.rs:65` owns the global cached browser;
  `rust/tools/src/web_search.rs:85` returns an existing instance without a liveness probe.
- `rust/tools/src/web_search.rs:187` recognizes only `connection is closed` and
  `connection closed`; `rust/tools/src/web_search.rs:199` performs one recreation/retry.
- The timeout field's comment describes 30-second page-load timing, unlike the actual
  300-second default and browser-idle wiring; clarify only as relevant to this fix.

Related work: BROWSE-002 is done; BROWSE-001 concerns remote browser control and is not
a duplicate fix for stale CDP/browser-cache recovery.

## Expected outcomes

1. Reproduce a cached-browser operation after idle loop expiry and capture the actual
   returned error/liveness behavior.
2. Detect stale browser state and recreate/reconnect safely using bounded recovery,
   including both launched and explicitly connected modes where supported.
3. Retry only failures that justify browser recovery; page/script/navigation errors
   must not become an unbounded relaunch loop.
4. Avoid races in which one operation clears a newer healthy cached browser or multiple
   callers repeatedly create replacement processes.
5. Make diagnostics distinguish expected idle shutdown, successful recovery, and a real
   user-visible failure; avoid treating teardown noise as another incident.
6. Preserve browser-mode/configuration choices and existing cleanup ownership.

## Regression evidence and completion

Review `rust/tools/tests/chrome_web_search_test.rs:1`. Add deterministic coverage for
stale-cache recovery, bounded replacement failure, healthy reuse, nontransport errors,
and concurrent callers if cache replacement is changed. Use a short controlled timeout
or test seam rather than waiting five minutes; distinguish tests requiring installed
Chrome from ordinary deterministic tests.

Do not merely disable all timeouts, raise the idle limit, retry every error, or kill an
externally owned browser. Completion requires demonstrated next-operation behavior,
scenario-linked recovery tests, and scoped quality checks without process leaks.

## Implementation evidence

See [regression-evidence.md](regression-evidence.md) for the observed idle-expiry
error, RED/GREEN runs, scenario mappings, and scoped validation outcomes.
