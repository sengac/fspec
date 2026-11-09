# BUG-634 validation

## Behavior baseline

`1b19dc9f2^` supplies the behavior baseline. Anthropic retains its original HTML
authorization-link/code-paste form. Codex retains its automatic localhost callback
on port 1455. Both work through client-hosted browser listeners with exchange and
credential persistence on the connected backend. Browser login is not replaced
by headless or device login.

The original missing browser rows and unsupported headless RPCs were reproduced
before implementation. Additional regressions cover URL copy/open focus, late
start/ready actions after cancellation, failed browser launch, interrupted HTTP
connections, and Codex authorization invalidation by logout or a newer login.

## Completed validation

- `cargo test --profile ci-test -p codelet-fspec-tui --test bug634_daemon_browser_login --test bug634_codex_browser --test bug634_code_entry_focus --test bug634_daemon_oauth -j 4`: **17 passed**.
  Log: `/tmp/bug634-tui-combined-final.log`.
- `cargo clippy --profile ci-test -p codelet-fspec-tui --lib --test bug634_daemon_browser_login --test bug634_code_entry_focus --test bug634_daemon_oauth --test bug634_codex_browser -- -D warnings`: **passed**.
  Log: `/tmp/bug634-tui-clippy-final2.log`.
- Existing PROV-112, PROV-113, PROV-114 and REFAC-019 regression suites: **33 passed**.
  After the final asynchronous Codex start change, PROV-113 and REFAC-019 were
  rerun: **15 passed**. Logs and snapshot review are recorded in
  `websocket-oauth-test-evidence.md` in this directory.
- Anthropic browser helper and interrupted-connection regression: **7 new and
  9 existing browser-server tests passed**, with strict Clippy for both new
  targets. Logs: `/tmp/bug634-claude-connection-green.log` and
  `/tmp/bug634-claude-connection-clippy.log`.
- Codex client callback: **5 passed**, including binding the authorization
  issued by the daemon. Log: `/tmp/bug634-bind-green.log`.
- Codex daemon completion and pending-authorization lifecycle: **5 passed**,
  including successful token persistence, original deadline expiry, invalid
  state, logout, and newer-login invalidation. Log: `/tmp/bug634-pending-green3.log`.
- Provider strict Clippy passes after the lifecycle correction.
  Log: `/tmp/bug634-lifecycle-provider-clippy.log`.
- RPC strict Clippy passes for both completion and pending-login regression
  targets. Log: `/tmp/bug634-lifecycle-rpc-clippy-final.log`.
- Total new scenario regressions: **34 passed**, with none ignored.
- All seven BUG-634 features pass the combined Gherkin, tags, formatting,
  coverage, and specification audit: `fspec audit --work-unit BUG-634`.
- Changed parent-owned Rust modules and tests pass rustfmt and diff whitespace checks.

## Test boundaries

Tests use real WebSocket/RPC connections and real loopback browser form/callback
HTTP servers. Token-exchange success uses a local test issuer and isolated
credential directories. No live Anthropic/OpenAI account login was performed,
and no real user credential was written or cleared. An interactive browser
smoke test with the user's accounts remains a deployment check, not something
these offline regressions claim to have performed.

## Pre-commit verification

- `cargo fmt -p codelet-providers -p codelet-rpc -p codelet-fspec-tui -- --check`: passed.
- Fresh strict Clippy (`--profile ci-test`, `--lib`, changed OAuth integration
  targets, `-- -D warnings`) passed for all three packages. Logs:
  `/tmp/bug634-commit-providers-clippy.log`, `/tmp/bug634-commit-rpc-clippy.log`,
  `/tmp/bug634-commit-tui-clippy.log`.
- `fspec audit --work-unit BUG-634`: no problems in seven features.
- No RPC wire types or service declarations changed. Existing RPC signatures
  and result fields are reused; Codex start/completion support is implementation-only.
