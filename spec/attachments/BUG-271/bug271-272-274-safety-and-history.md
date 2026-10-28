# BUG-271 / BUG-272 / BUG-274: safety and recoverable history

This is the fresh bounded three-card task, using the controller-verified
`codex/gpt-6.1-sol`. No old task, live historical session or checkpoint was
resumed or repaired. The controller created one fresh Fspec session for this
three-card task; no additional subordinate agents were spawned. No commit, push,
installation or daemon restart was performed.

## Authorized cleanup

Roland explicitly authorized exactly one `cargo clean` in
`/Users/rquast/projects/fspec/rust`. The controller interrupted only this task,
verified no Cargo/compiler/linker processes remained, then executed it:
exit 0, `Removed 69347 files, 25.0GiB total`. Available space rose from about
2.2 GiB to 27,595,824 KiB. Sources, specifications, evidence and installed
services were preserved. Original receipts are
`/tmp/fspec-cargo-clean-evidence.md` and
`/tmp/fspec-cargo-clean-preflight.json` (`[]`). No further cleanup was requested
or performed by this task.

## Honest fixture isolation correction

The pre-clean TUI fixture changed actual user `tui.lastUsedModel` to
`anthropic/claude-opus-4-5`: redirecting only `set_data_directory` was insufficient.
The controller privately backed up the config and restored only that test-written
key to the previously verified `codex/gpt-6.1-sol`, using unchanged default-model
evidence, not a guessed baseline. Other bytes and parsed values were verified
unchanged. The receipt is `/tmp/fspec-fixture-model-repair-receipt.json`.
Private configuration/backup contents are not attached.

Every subsequent validation command explicitly uses:

```
FSPEC_USER_DIR=/tmp/fspec-bug271-272-274-isolated-z8nz6hi7/user
XDG_CONFIG_HOME=/tmp/fspec-bug271-272-274-isolated-z8nz6hi7/config
XDG_RUNTIME_DIR=/tmp/fspec-bug271-272-274-isolated-z8nz6hi7/runtime
CARGO_PROFILE_CI_TEST_DEBUG=0
```

Cargo commands use `--offline --profile ci-test -j2`, named packages/targets,
one compiler queue, individual logs/exits and a 6,000,000-KiB free-space guard.
No harness `HOME` or `CODEX_HOME` is repurposed. The controller separately
approved triage of the existing OAuth test's subprocess-local temporary
`CODEX_HOME` and mock keyring. Session/UI fixtures write the approved temporary
user config; explicit temporary data directories remain in place.

The actual user-config SHA-256 checked before and after completed validation
phases is:
`8eed6007c58b8f902efcf640fb8625a2662367cf4551f532ab354773650fce4c`.
Before the broader session/UI checks, the isolated standalone PROV-101 positive
fixture passed and wrote `google/gemini-2.5-pro` only to temporary configuration,
while the actual user-config hash stayed unchanged. Its receipt is
`/tmp/fspec-fixture-isolation-prov101-proof.json`. A later real writer proof after
checked creation/TUI checks is `/tmp/bug271-isolation-proof-after-tui.log`. A mismatch stops validation; it
never triggers automatic restoration.

## Preserved evidence and limits

Initial BUG-272 attribution-failure runs are not causal RED. Valid RED is
`/tmp/bug272-post-clean-consolidated-red.log`: six admitted scenarios, one pass
and five behavioral failures, before the production guard. The network scenario
proves both real writes and genuine outcomes before its failing assertion.
Initial isolated GREEN is `/tmp/bug272-post-clean-green.log`: all six pass.
Interrupted/incomplete builds are not counted as successful checks.

BUG-271's original six-scenario transport/creation feature and coverage, and
its duplicate/stale core mappings, are preserved as registered attachments.
The checked-creation scenario is unchanged in its separate feature, satisfying
the workflow's one-feature/one-test-file gate without dropping acceptance.

BUG-274's obsolete missing-API intake is retained in architecture notes. Its
current title/description describe verification of the existing public facade.
The historical BUG-273 six-library validation document remains unchanged and
accurate; new RPC/final-binary results are recorded separately. Controller live
RPC receipts `/tmp/bug274-live-rpc-diagnostics-1.json` and `-2.json` both contain
`{"isError": false, "result": []}`: API availability only, not damaged-fixture
proof or live-session repair.

No remote schema cause, successful affected-session repair, Windows execution,
live-store benchmark, full-workspace suite or deployed-binary replacement is
claimed. No outcome fabrication, ancestry/affinity reset, broad diagnostic
suppression or compaction fallback was added.

## Final closure record

Final results are 120 passing reported Rust tests across 35 scoped targets,
including all 21 acceptance scenarios. All default/fixture builds, check and four
warnings-denied Clippy groups pass. After applying `@done` through supported CLI
commands, all five owned feature syntax/tag checks and coverage audits pass again
(`/tmp/bug271-272-274-done-tags.log`); diff checking and the actual user-config hash
remain unchanged. Global tags retain the same 522 unrelated pre-existing failures.

Workflow transitions emitted generic configured `prov149_codex_history` /
`prov149_codex_reload` and older quality-command suggestions. Those old queued
commands were not resumed, per the user's explicit bounded-task instruction.
They are not claimed as runs or passes. Current required replay/checkpoint,
durable preservation, accepted/interrupted batches and prior 273/275–277 controls
are instead enumerated in the attached fresh 120-test results. Existing unrelated
hook/configuration records were not altered to suppress that guidance.
