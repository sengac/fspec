# PROV-159 — Complete Codex OAuth lifecycle repair

## Scope and provenance

Track ALL findings from the fspec versus original Codex OAuth review, especially
immediate logout/login and stale tokens. This card captures specification and
research only; no implementation or live OAuth reproduction was performed.

- Research date: 2026-10-16.
- fspec baseline: `b7628f2fee4a5961ce05ddc70c6166afb8e21f7c`.
- Upstream repository: https://github.com/openai/codex
- Upstream commit: `bb64b894fd67158be0f874872e6d7bfe717e7368`.
- Original checkout: `/tmp/codex`; not needed once these attachments exist.
- Companions: `prov159-upstream-storage-refresh.md` (samples S1-S5) and
  `prov159-upstream-login-lifecycle.md` (samples L1-L4).
- Evidence is read-only source inspection: no credential contents were read,
  no real keychain was modified, and no live authorization was attempted.
- Race consequences below require deterministic failing tests before production
  implementation; source inspection is not presented as runtime reproduction.

## Findings and required fixes

### F1 — P1: Keychain reads and file-only mutations disagree

Evidence: `rust/providers/src/codex/codex_auth.rs:97` reads macOS keychain;
`:131` prefers that over the file; `:143` writes only `auth.json`; `:71` deletes
only that file. `rust/rpc/src/oauth_disconnect.rs:69` reads preferred credentials,
strips tokens, then writes to the file. `rust/napi/src/codex_oauth.rs:264` follows
the same read/strip/file-write pattern.

With a keychain entry present, logout can return success while subsequent reads
still return old tokens. Login/refresh can write new file credentials while new
providers still select the keychain account; provider deletion leaves it intact.

Fix: one source-aware read/save/clear/delete policy for the current `CODEX_HOME`,
including fallback cleanup and honest storage errors. Preserve independent API
keys. Do not treat every keyring error as absence or silently switch accounts.
Sample S1 demonstrates upstream's coherent backend and fallback cleanup.

Correction to an early hypothesis: fspec already uses `Codex Auth` and a
canonical-home-derived `cli|<sha256-prefix>` key like upstream. The bug is mutation
policy, NOT a fixed legacy keychain account name.

### F2 — P1: Logout/re-login does not invalidate existing clients

Evidence: `rust/providers/src/codex/constructors.rs:125` creates middleware;
`rust/providers/src/codex/refreshing_client.rs:85` allocates independent token
state. `:137` checks local expiry, not the current credential authority.
`rust/providers/src/codex/recovery.rs:45` and
`rust/providers/src/codex/websocket.rs:72` use that cached state. RPC clear mutates
storage; TUI completion reloads credential navigation, not these client instances.

Risk: existing providers can keep using A after logout or successful login B;
restarting can conceal the cache problem.

Fix: shared auth authority or explicit invalidation/rebuild of provider, HTTP and
WebSocket authentication at successful transitions. Account-scoped connections,
routing and continuation state must not cross identities. Sessions deliberately
rebind or fail clearly, never silently retain A. Samples S3 and L4.

### F3 — P1: Refresh coordination is per client, not per authority

Evidence: `refreshing_client.rs:97` allocates a new lock per independent client;
`:151` double-checks only that lock. `constructors.rs:44` forces disk-loaded
credentials to `Some(0)` expiry. There is no guarded authority reload before
refresh; RPC/NAPI/legacy writers do not share the middleware lock.

Risk: C1/C2 load rotating token R0; C1 rotates to R1; C2 still sends stale R0,
fails, or replaces newer state. Rapid re-login/reconstruction amplifies this.

Fix: serialize by credential authority, reload and reuse newer same-account auth
before refresh, and handle competing processes sharing `CODEX_HOME`. Preserve
account guards and rejected-token checks. Existing bounded 401 recovery is
already present at `recovery.rs:91`; do NOT claim it is absent. Sample S2.

### F4 — P1: Late refresh resurrects logout or overwrites new login

Evidence: `refreshing_client.rs:159` awaits refresh, updates local state, releases
the lock, then persists at `:170`. `persist_tokens` at `:198` blindly writes a
new auth document, with `openai_api_key: None`. NAPI refresh at
`rust/napi/src/codex_oauth.rs:225` persists a supplied refresh result without
checking current account/generation. The legacy sync path in
`codex_auth.rs:158` similarly reads, performs network work, then writes later.

Risk: refresh(A) starts; logout clears A; login(B) commits; late refresh(A)
replaces B. Logout alone can also be undone; whole-document writes lose API keys.

Fix: account AND auth-generation/version fencing at commit time, coordinated
with all lifecycle writers. Check before requests and after awaited network
work; same-account re-login invalidates older generations too. Preserve sibling
credential fields. Sample S2 is a guarded-reload pattern, NOT proof that upstream
completely solves post-network stale writes; fspec needs explicit A7-A9 tests.

### F5 — P1: Esc cancels presentation, not authentication

Evidence: `rust/fspec-tui/src/views/provider_settings/oauth_login.rs:94` only
bumps the UI generation. `rust/fspec-tui/src/app/dispatch_provider_settings_oauth.rs:123`
spawns backend login; generation filtering only rejects result actions.
`rust/providers/src/codex/codex_oauth_server.rs:102` persists independently;
`rust/rpc/src/oauth_login.rs:127` device polling also persists independently.
The browser entry point at `codex_oauth_server.rs:77` binds fixed port 1455.

Risk: cancelled login still saves tokens; immediate retry finds the old listener;
older attempts overwrite newer login/logout. RPC timeout shortening the window
does not implement correct user cancellation.

Fix: backend attempt IDs, real cancellation of listener/poller/exchange, commit
fencing, supersession and logout cancellation. Await cleanup needed for immediate
callback-port reuse. Retain UI generation guards as additional protection.
Samples L1-L3 show upstream attempt ownership and cancellation.

### F6 — P1: RPC deadline is shorter than interactive OAuth

Evidence: `rust/fspec-tui/src/transport/embedded.rs:860` browser login and `:898`
device polling use unchanged `tarpc::context::current()`;
`rust/rpc/src/lib.rs:2221` and `:2251` await the entire flow. tarpc 0.34.0
`context.rs:91` defaults to 10 seconds, but
`rust/providers/src/codex/codex_oauth.rs:31` permits 300,000 ms.

Risk: ordinary human authorization exceeding 10 seconds fails at RPC level
instead of receiving the advertised five-minute allowance.

Fix: explicit operation-specific budgets covering the complete flow, OR short
start/status/cancel RPCs with asynchronous completion. Keep unrelated RPC
defaults short and ensure deadline/client-disconnect cleanup. Sample L2 shows
upstream's prompt login-ID/URL response and background completion pattern.

### F7 — P2: Disk-loaded freshness is discarded

Evidence: `constructors.rs:44` forces immediate expiry on every disk-loaded
token; `refreshing_client.rs:89` otherwise defaults to one hour when lifetime is
absent. The constructor does not use persisted `last_refresh` or token expiry.

Risk: fresh login immediately rotates again on provider construction, amplifying
F1/F3 and avoidable issuer dependence.

Fix: useful expiry/freshness metadata, a safety window and a tested conservative
fallback for absent/malformed expiry; route necessary refresh through F3. Do not
trust unverified JWT claims for authorization. Sample S4 is expiry-first with a
last-refresh fallback; choose fspec's own appropriate policy/constants.

### F8 — P2: File writes are unprotected and non-transactional

Evidence: `codex_auth.rs:143` calls `fs::write` without explicit restrictive
creation permissions, atomic replacement or coordination across auth writers.

Risk: permissive umask exposes new files; concurrent readers can see partial or
truncated JSON; competing whole-document writers lose sibling fields.

Fix: restrictive supported-platform permissions, safe complete-document
replacement and F4-coordinated mutations; test persistence failures. Sample S5
demonstrates upstream Unix 0600 creation mode ONLY: it also uses truncate/write,
so is NOT an atomic-write solution or proof existing permissions are tightened.

## Acceptance/regression matrix

Each row must map to a scenario and failing test before production changes.

| ID | Deterministic setup/action | Required result |
| --- | --- | --- |
| A1 | Keychain A plus file fallback; logout/read/restart | No OAuth resurrection; independent API key preserved |
| A2 | Keychain A; browser/device login B | UI, new provider and restart select B |
| A3 | Delete provider using keychain auth | No source silently restores deleted OAuth credentials |
| A4 | Reuse live HTTP/WebSocket provider after logout/login B | No new A request; deliberate rebind or clear reauth |
| A5 | Independent clients load R0 and refresh concurrently | Coordinated rotation/reload, no stale overwrite |
| A6 | External process rotates R0 to R1 before local expiry/401 | Reload R1, retain account guards |
| A7 | Pause refresh A; logout; release refresh | Stored/live auth remain logged out |
| A8 | Pause refresh A; logout/login B; release A | B remains authoritative everywhere |
| A9 | Pause refresh A1; same-account re-login A2; release A1 | Generation fencing preserves A2 |
| A10 | Cancel during browser wait/device poll/token exchange | No late persistence/false success; resources cleaned |
| A11 | Esc then immediate browser retry; old callback last | Port reusable; only newest attempt commits |
| A12 | Real tarpc login takes >10 seconds, within OAuth budget | Success; unrelated RPC defaults unchanged |
| A13 | OAuth deadline expires or client disconnects | Clear outcome, no leaked listener/late commit |
| A14 | Fresh/near-expiry/expired/missing/malformed disk expiry | Correct buffered decision, no refresh storm |
| A15 | Parallel reads/writes, injected write failure, Unix creation | Complete JSON, prior state safe, restrictive mode |
| A16 | Keyring read/save/delete failure | Honest safe error, no false success/hidden account fallback |
| A17 | Local logout/login with browser SSO cookies retained | Local isolation holds; SSO not misreported as stale auth |

## Implementation/testing handoff

- Centralize policy in `codelet-providers`; RPC/NAPI delegate. Audit legacy sync
  refresh and provider deletion too. Do not close after fixing only F1.
- Existing specs: `spec/features/codex-oauth-login.feature`,
  `spec/features/codex-refreshing-client.feature`,
  `spec/features/codex-oauth-napi-bindings.feature`,
  `spec/features/provider-settings-oauth-login.feature`,
  `spec/features/provider-settings-oauth-disconnect.feature`.
- Historical context: PROV-011/013/015/016/019/105/112/113/133. These are not new
  blocking dependencies on completed cards. Preserve PROV-112's API-key contract.
- Reuse `rust/test-helpers/`, temporary `CODEX_HOME`, real loopback endpoints,
  isolated stores and a controlled keyring seam. Never touch real credentials.
  Use `serial_test` for environment mutations and deterministic barriers/time
  control rather than arbitrary sleeps.
- Include real embedded tarpc tests: MockBackend UI tests alone miss F6 and
  persistence after cancellation. Cover HTTP and WebSocket lifecycle consumers.
- Existing `codelet-providers` targets: `codex_oauth_test`,
  `codex_oauth_server_test`, `codex_device_auth_test`,
  `codex_refreshing_client_test`. Existing `codelet-fspec-tui` targets:
  `provider_settings_oauth_disconnect_prov112`,
  `provider_settings_oauth_login_prov113`. Existing `codelet-napi` target:
  `codex_oauth_napi_test`. Add dedicated PROV-159 integration targets as needed.
- Run scoped commands such as `cargo test -p codelet-providers --test <target>`;
  never unscoped workspace tests. Run scoped check/clippy and spec/tag validation
  during implementation. This planning change does not run Rust tests.
- Local logout does not clear browser SSO cookies or necessarily revoke remote
  tokens. Immediate same-account browser authorization can be legitimate; an
  account chooser/revocation UX needs an explicit product decision and support.
- Ask before major refactoring; follow ACDD and keep files below 300 lines.
  Completion requires ALL A1-A17 obligations to be covered or explicitly resolved
  with evidence, not just a successful manual logout/login once.
