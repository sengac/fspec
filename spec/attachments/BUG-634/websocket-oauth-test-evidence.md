# WebSocket OAuth forwarding evidence

## ACDD

Specified `daemon-oauth-code-and-token-routing.feature` before writing
`rust/fspec-tui/tests/bug634_daemon_oauth.rs`. The first compiling run failed
all four tests on inherited unsupported/unimplemented methods, before adding
the forwarding module. Red output: `/tmp/bug634-oauth-red.log`.

## Verification

- `cargo test --profile ci-test -p codelet-fspec-tui --test bug634_daemon_oauth --test provider_settings_oauth_disconnect_prov112`: 4 + 12 passed.
- `cargo check --profile ci-test -p codelet-fspec-tui`: passed.
- `cargo clippy --profile ci-test -p codelet-fspec-tui --test bug634_daemon_oauth --test provider_settings_oauth_disconnect_prov112 -- -D warnings`: passed after replacing panic-based error extraction in tests.
- New feature validation and tag validation: passed.
- Existing disconnect feature validation: passed.
- Scoped audit: new routing feature clean; old disconnect feature retains one unrelated pre-existing missing implementation-boundary-text mapping for `Cancelling disconnect with n makes no backend call`.

Logs: `/tmp/bug634-oauth-green.log`, `/tmp/bug634-oauth-check.log`,
`/tmp/bug634-oauth-clippy.log`.

## Exercised boundaries

The shared real WebSocket daemon fixture returns fresh Anthropic PKCE, whose
challenge is checked using SHA-256 and URL-safe base64. Invalid completion
state returns daemon CSRF validation, without token exchange. Temporary
`FSPEC_HOME` credentials are read, cleared idempotently, and malformed JSON
preserves the daemon error. Unsupported device providers and malformed
Copilot handles fail in daemon validation; the invalid Copilot enterprise
host `[` fails URL parsing without provider network access. All seven
forwarders report typed disconnected errors when no connection exists.

The macro chain now runs scheduling_loops -> oauth -> finish. Device start
and poll use the same interactive context as embedded; other deadlines match
embedded. No browser-login RPC forwarder was added. Client-browser capability
was initially Anthropic-only; the follow-up client browser work also enables
Codex, and the parent owns its UI/trait tests.

## Existing-suite and frozen-contract follow-up

Command:

`cargo test --profile ci-test -p codelet-fspec-tui --test refac019_contracts --test provider_settings_oauth_disconnect_prov112 --test provider_settings_oauth_login_prov113 --test provider_settings_oauth_copilot_device_prov114 --no-fail-fast`

- Before contract updates: PROV-112 12 passed, PROV-113 9 passed, PROV-114 6
  passed; REFAC-019 5 passed and 1 failed on the intentional backend capability
  addition and corrected token/browser documentation hashes.
- After updates: all 33 tests passed (12 + 9 + 6 + 6).
- Logs: `/tmp/bug634-oauth-contracts-before.log` and
  `/tmp/bug634-oauth-contracts-after.log`.
- REFAC-019 retains ordered-prefix protection; its trailing-addition allowlist
  now recognizes the seven OAuth forwarders, client-browser capability, and
  `OAuthBrowserReady`. Only the observed intentional backend hashes and
  capability entry were updated in the frozen snapshot.
- PROV-113's obsolete embedded-only scenario is now a no-browser-capability
  scenario, with matching test/coverage. Removed unsupported-WebSocket claims
  from its embedded-positive test. The revised feature validates.
- `docs/PROVIDERS.md` describes both client-side browser flows, Anthropic code
  paste, Codex port 1455 callback, alternate flows, and wrapped/copyable URLs.

Pending at this checkpoint: Codex callback and browser-ready action workers
are still implementing. Re-run the command after their changes; an inserted
`OAuthBrowserReady` (rather than appended) must be pinned in the Action snapshot,
and any new backend/RPC methods require review rather than a blanket update.

## Final validation after both browser integrations

The same four-target Cargo command completed against the integrated helpers
and client browser flows: **33 passed, 0 failed** (PROV-112: 12, PROV-113: 9,
PROV-114: 6, REFAC-019: 6). Final log:
`/tmp/bug634-final-old-oauth-before.log` (the filename reflects when the queued
command was started; it compiled and ran after the reviewed snapshot updates).
Cargo completed in 8m52s, including waiting behind other workspace builds.
There are no pending commands from this validation.

The final snapshot additions pin `OAuthBrowserReady` and the embedded
Anthropic/Codex client-browser capability. An existing headless-action doc
comment had accidentally attached to the new browser action; it was restored
to `OAuthHeadlessReady`, preserving that action's original hash. The RPC
contract snapshots already match: the new Codex branch lives in the delegated
completion helper, not the snapshotted RPC method body. No unrelated contract
changes were accepted.

Scoped audits now report **no problems in four features**: OAuth login,
disconnect, Copilot device login, and daemon code/token routing. Updated moved
implementation ranges and linked the extracted `app/oauth_start.rs` path.
Targeted rustfmt checks and diff whitespace checks passed.

The final provider documentation explicitly preserves Anthropic's client-hosted
HTML paste form (not TUI code entry for browser login), its copyable local form
URL, Codex's automatic client callback on port 1455, daemon exchange/storage,
and the separate headless/device alternatives. This supersedes the initial
follow-up documentation description above.

## Final async lifecycle correction

Reviewed the later change in `rpc/src/service_implementation/oauth_worktrees.rs`:
`oauth_headless_start` now awaits the asynchronous lifecycle helper. Refreshed
only that method's hash in the RPC implementation snapshot; no other snapshot
entries or App production code changed in this pass.

`cargo test --profile ci-test -p codelet-fspec-tui --test refac019_contracts --test provider_settings_oauth_login_prov113 --no-fail-fast`

Completed after the correction: **15 passed, 0 failed** (REFAC-019: 6,
PROV-113: 9), with no skipped tests or pending commands. Final Cargo log:
`/tmp/bug634-final-async-contract-before.log` (queued before the hash refresh,
compiled and ran afterward). The focused pre-refresh source-contract failure
is recorded in `/tmp/bug634-final-async-source-review.log`; the corresponding
green source review is `/tmp/bug634-final-async-source-green.log`.
The earlier 33-test run remains historical evidence; these 15 tests revalidate
the affected suites after the subsequent async correction.
