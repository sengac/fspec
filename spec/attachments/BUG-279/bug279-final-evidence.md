# BUG-279 final evidence

## Result and scope

The targeted native Codex device flow is compatible with the pinned official
protocol in real production-path loopback integration fixtures. Only two shipping
files change: `codex_device_auth.rs` and `device_poll.rs`. Client ID, issuer,
browser OAuth, account extraction, lifecycle ownership and the existing shared
300-second operation budget remain unchanged. No real login is attempted.

PROV-030 (broad UX unification) and PROV-094 (scripted provider parity) were
checked before creating the distinct BUG-279. The issue document was registered
early through actual `add-attachment` and verified in the work-unit list.

## Official source comparison

The read-only checkout is clean at official public commit
`6ba3a84680236c621b93e0b8dc140687a37f41fc`, committed 2026-10-28T04:05:31Z.

- [Device implementation](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/src/device_code_auth.rs)
- [Prompt test](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/src/device_code_auth_tests.rs)
- [Device integration tests](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/tests/suite/device_code_login.rs)
- [Matching public client ID](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/src/auth/manager.rs#L1794)
- [Matching issuer](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/src/server.rs#L78)

| Boundary | Previous native behavior | Corrected behavior |
| --- | --- | --- |
| Start | Form client_id | JSON client_id |
| Decoder | Required numeric interval, only user_code | Trimmed string/missing interval and usercode alias; numeric compatibility retained |
| Poll | Form containing only device ID | JSON with both original device_auth_id and user_code |
| Status | Parsed JSON even on failed HTTP | 403/404 bounded pending without body parsing; other non-success statuses cannot authorize exchange |
| Exchange | No redirect URI | Exact issuer /deviceauth/callback redirect URI with existing PKCE/code/client fields |
| Errors | Reflected server bodies/unknown errors; logged user code | Useful stage/status errors without server secrets; user code removed from debug logging |

Numeric interval and legacy successful-HTTP pending/slow-down handling remain
intentional backward compatibility. The upstream 15-minute budget does not
replace Fspec's existing 300-second shared budget. Upstream workspace policy
differences are not used to remove or weaken Fspec checks. No upstream build,
installation, or repair of the incomplete `/tmp/codex` was performed.

## Causal executed RED and fixture corrections

All RED receipts remain separately named; production OAuth source was unchanged
until the successful supported implementing transition.

- `red-provider` is a compilation failure and is **not behavioral RED**.
- `red-provider-executed` contains four useful behavioral defects; its malformed
  case stopped on missing seed/ENOENT and is excluded from causal counts.
- `red-native` stopped at missing projected row; both setup failures are excluded.
- `red-provider-corrected` compiled and executed **0/5 passing**: the permissive
  primary user_code/string-interval response independently reaches the parser and
  fails there (not HTTP415); HTTP500 with success-shaped data reaches one token
  exchange; pending/timeout handling and exchange-secret redaction also fail.
- `red-native-executed` compiled and executed **0/2 passing**: the actual projected
  device row and Enter action call real embedded RPC/native start and hit the
  strict JSON endpoint's HTTP415, while a separate HTTP404 body reaches settings
  unredacted. Authentication results are never scripted successful transports.

The empty default credential listing was corrected using a synthetic provider
record through the real projection, not by changing native/TUI production code.
The seven already-specified scenarios were split into two feature files solely
to satisfy Fspec one-feature/one-test-file traceability after a held transition.
The first transition did not execute; the single corrected retry succeeded.

## GREEN and final validation

**44 distinct top-level tests pass**, zero fail: five new provider cases, two
new native menu/RPC cases, all 11 legacy device controls, 21 provider lifecycle/
storage/entry-point controls and five native RPC/logout/error controls.
Nested output and the superseded legacy GREEN are not counted twice.

The successful native path includes actual menu selection, device-waiting,
real start/poll/exchange requests, persistence and success. The error path also
checks the rendered error screen and a fresh successful retry. HTTP500 is now
an explicit error with zero exchanges and byte-identical existing credentials.
Malformed input, unknown terminal errors, timeout, cancellation, failed exchange
and missing account identity cannot report successful authorization or replace
existing credential bytes. Existing browser/lifecycle controls pass unchanged.

The oversized legacy test target was split into small modules while retaining
all 11 functions and meaningful display, identity, persistence, delay, count,
expiry, denial, timeout and network checks. Its sequence, terminal and happy
pending responders each require POST, JSON and both exact original identifiers.
Only obsolete form/no-redirect assumptions were replaced with official wire
requirements. No blanket lint suppression, unwrap or panic was added.

Production library check; default and test-stub-provider fspec binary builds;
and production, named provider/native targets, default/fixture binary Clippy
with `-D warnings` all pass. Owned Rust 2021 formatting and diff checks pass.
A legacy entry-module ordering correction and two native test-only redundant
clone/closure fixes have fresh dedicated test/Clippy receipts. The failed
native Clippy receipt is retained and superseded explicitly, without suppressing
lints or changing assertions. Production source remains unchanged from the
preceding GREEN. Every owned Rust file remains below 300 lines.

Coverage is 100% for both new features (5/5 and 2/2) and the maintained legacy
feature (9/9); current scenario/test/implementation ranges and audits are valid.
Both new features pass formatting, syntax and registered tags. Global tags
still report 522 pre-existing invalid files (1,642 pass); this is not an
all-project-clean claim. Existing legacy tag debt is not silently repaired.

The companion JSON records exact commands, timestamps, exits, log hashes,
source snapshots, final hashes and bounded coverage. Logs/individual receipts
remain under `/tmp/fspec-device-oauth-work/logs`. Every compiler phase used the
same working-tree cache, offline ci-test debug-0, named targets, -j2 and the
6,000,000-KiB guard, with no competing compiler queue or other source root.

## Log review and safety

Sanitized read-only scans found no matching historical device-auth failure
category. Whole-word mentions and timestamps are recorded in the issue/JSON,
not raw lines. The exact error Roland saw remains **unconfirmed**; synthetic
protocol evidence must not be represented as live log causality. Logs were not
modified by review; ordinary active-daemon appends can occur independently.

All HTTP test endpoints are loopback. Credentials, IDs, codes, tokens and JWTs
are synthetic; test CODEX_HOME is scoped and restored; credential tests install
the mock builder and do not access the real Keychain. Child-only FSPEC_USER_DIR,
XDG_CONFIG_HOME, XDG_RUNTIME_DIR and TMPDIR are isolated under
`/tmp/fspec-device-oauth-isolated`; daemon HOME/CODEX_HOME are not repurposed.

Both real private credential/config baseline files verify unchanged without
disclosing credential hashes or contents. Actual config SHA256 remains
`8eed6007c58b8f902efcf640fb8625a2662367cf4551f532ab354773650fce4c`.
All 1,790 pre-existing work-unit records remain identical, including done
BUG-271/272/274/278 and BUG-278's independent audit attachment registration.
HEAD remains `9b490e8917e8928cc87ab3ca6bad920bf7afbc39`.

The historical failed session manifest still matches
`51e67fcbc1c9b1c60e2c0a2247c49e5edeabe0aa537e7aa224cb8bc8f5a19a08`;
neither it nor the taken-over task was changed, resumed or contacted. No agents
or sessions were created. No real device request, browser login, code submission,
account change, fallback, commit, push, installation, binary replacement,
restart, deployment, cargo clean or unrelated cleanup was performed.

No Windows or live account-login execution is claimed. The previously documented
unrelated intermittent OAuth timing limitation is not claimed fixed; the scoped
unchanged lifecycle controls passed. This receipt records completed validation;
the ordinary supported done transition and its unsuppressed checkpoint behavior
follow registration of these final attachments.
