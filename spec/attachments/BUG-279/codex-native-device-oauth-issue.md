# BUG-279: native Codex device-code login protocol compatibility

## Report and scope

Roland reports an error after choosing **Sign in with device code**. This card
owns the targeted built-in native Codex start/poll/exchange fix through ACDD.
PROV-030 concerns broad browser/headless UX unification; PROV-094 concerns the
separate scripted `codex.rhai` provider. Neither is an exact duplicate.

Native route: TUI Codex Headless action → RPC `oauth_login::device_start` →
`codex::device_session::start_device_login` → `request_device_code`.
Completion goes through the retained lifecycle attempt, polling, token exchange,
account-ID extraction and existing credential commit. Browser OAuth is not the
repair target. No authentication, account or workspace check may be bypassed.

## Read-only official comparison

The official public checkout `/tmp/fspec-codex-device-upstream` is clean and
read-only, origin `https://github.com/openai/codex.git`, pinned HEAD
`6ba3a84680236c621b93e0b8dc140687a37f41fc`, committed
`2026-10-28T04:05:31Z`. The incomplete `/tmp/codex` remnant is not repaired.

- [Device implementation](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/src/device_code_auth.rs)
- [Prompt unit test](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/src/device_code_auth_tests.rs)
- [Login integration controls](https://github.com/openai/codex/blob/6ba3a84680236c621b93e0b8dc140687a37f41fc/codex-rs/login/tests/suite/device_code_login.rs)

Observed source differences requiring executed causal fixtures:

| Boundary | Pinned official implementation | Current native Fspec |
| --- | --- | --- |
| Usercode request | JSON `client_id` | Form `client_id` |
| Usercode response | `user_code`/`usercode`; trimmed string interval, missing interval defaults to zero | Only `user_code`; required numeric interval |
| Poll request | JSON `device_auth_id` and `user_code` | Form containing only device ID |
| Poll status | 403/404 pending, bounded wait; other non-success statuses fail | Deserializes JSON without HTTP-status check |
| Code exchange | Issuer `/deviceauth/callback` redirect URI | No redirect URI |

Official polling waits up to 15 minutes; Fspec has an existing shared 300-second
operation budget including start, polling and exchange. Preserve Fspec's bound
and cancellation ownership rather than extending persistent access. Preserve
useful numeric interval compatibility and explicit legacy pending/slow-down
controls if they do not authorize non-success HTTP responses. The official
workspace enforcement is not permission to remove or weaken Fspec checks.

## Sanitized log findings, not live causality

Read-only category/timestamp scanning of all files under `~/.fspec/logs`,
including `private/client.log.2026-10-28` and
`private/fspec-combined.log.2026-10-28`, found **zero matching device-auth failure
categories**. Whole-word device mentions occurred at
`2026-10-27T03:43:23.959168Z` in combined 2026-10-27 and
`2026-10-28T04:25:22.437070+00:00` in current private client output. These are
not evidence of the reported error. No raw log lines, payloads, account details,
credentials, authorization headers, PKCE material, tokens or codes were copied
or displayed. Logs were not modified. Active logs may receive ordinary daemon
output independently of this read-only scan.

The exact error Roland observed is therefore **unconfirmed**. Source protocol
differences are compatibility hypotheses until loopback fixtures execute RED.

## Acceptance and safety plan

Specify first, then add real native production-path loopback fixtures, preferably
through the embedded RPC/TUI boundary. Execute a behavioral RED against official
JSON requests/string interval before production changes. Verify successful
start, pending 403/404, authorized polling and exact callback token exchange;
persist only valid synthetic credentials. Require explicit malformed, terminal,
timeout, cancellation and stale-attempt failures without credential mutation.
Errors visible at the TUI must not disclose server bodies or authentication
secrets. No real login or network auth request is authorized.

All test/Cargo commands use child-only isolation under
`/tmp/fspec-device-oauth-isolated`: FSPEC_USER_DIR, XDG_CONFIG_HOME,
XDG_RUNTIME_DIR and TMPDIR, plus debug-0 `ci-test`, offline `-j2`, named packages
and integration targets. Test CODEX_HOME is temporary and guard-restored;
keyring fixtures install a mock credential builder. Never repurpose agent HOME
or CODEX_HOME, build upstream or another source root into the target cache,
install/replace binaries, restart, commit, push, deploy or clean unrelated files.

Verify private credential/config baseline unchanged without printing hashes or
contents. Actual config SHA256 is
`8eed6007c58b8f902efcf640fb8625a2662367cf4551f532ab354773650fce4c`.
Stop on mismatch. Preserve task-start HEAD
`9b490e8917e8928cc87ab3ca6bad920bf7afbc39`, existing board changes and BUG-278
audit attachment; preserve every prior work-unit record and registered evidence.
BUG-271/272/274/278 remain done. The failed historical session remains read-only;
the taken-over task is excluded. No subordinate or additional sessions.

Final evidence must include fresh exact commands/exits/logs/source hashes,
relevant auth lifecycle regressions, affected production and default/fixture
binary checks/builds, Clippy `-D warnings`, coverage and scoped feature/tag gates.
The 522 existing unrelated global tag-invalid files and prior unrelated OAuth
timing limitation remain documented, not silently fixed or claimed clean.
