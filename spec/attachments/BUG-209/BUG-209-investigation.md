# BUG-209: stale exec-stdin snapshots restore completed command prompts

Date: 2026-10-21

## Symptom and confirmed cause

A finished Codex command can appear to await stdin even after the process-exit
clear has reached the TUI. An older asynchronous pull snapshot can be applied
after dismissal because the fetched-request reducer has no freshness guard.

Confirmed reducer ordering:

1. A probe obtains a pending request while the exec process is alive.
2. The process exits and `ExecStdinRequestCleared` dismisses the overlay.
3. The older `ExecStdinPromptFetched` result arrives and reinstalls the request.
4. Turn `Done` does not remove the reinstated prompt.

The temporary reproduction dispatched this ordering through the real App
reducers. It established stale-result acceptance, not an observed live-network
timing incident.

## Codex command completion trace

- `rust/providers/src/codex/agent.rs`: `exec_command`, `write_stdin`, and `shell`
  use `ExecToolFacadeWrapper`; `shell_command` uses `BashToolFacadeWrapper`.
- `rust/tools/src/facade/wrapper.rs`: exec facades delegate to `UnifiedExecTool`.
- `rust/tools/src/bash_session.rs`: Bash execution also delegates to unified exec.
- `rust/tools/src/unified_exec/tool.rs`: completed polls remove the process and
  return an exit code without a running exec session ID.
- `rust/tools/src/unified_exec/exec_stdin.rs`: a detector checks every two seconds,
  suggests input after at least three quiet seconds, and emits `None` on exit or
  store removal if it previously fired. The detector then stops.
- The detector captures its callback Arc, so unregistering the originating
  model-turn callback does not prevent a later process-exit clear.
- `rust/tools/src/unified_exec/spawning.rs`: PTY child completion kills its
  liveness anchor, allowing the same exit detection path.
- `rust/sessions/src/background_session.rs`: clearing a stored request emits
  `StreamChunk::ExecStdinRequestCleared` on the session-tagged broadcast.
- `rust/fspec-tui/src/app/dispatch_stream_chunks.rs`: request and clear chunks
  enqueue the existing fetched/dismissed actions.

The relevant END signal is `ExecStdinRequestCleared`, not agent-turn `Done`.
Commands may legitimately outlive a model turn; clearing all prompts on `Done`
would hide input for genuinely running background commands.

## Vulnerable frontend paths

- `rust/fspec-tui/src/app/dispatch_exec_stdin.rs`: focus probes await an RPC and
  enqueue unversioned fetched/dismissed results. The fetched reducer checks HITL
  precedence but otherwise accepts the request unconditionally.
- `rust/fspec-tui/src/app/dispatch_pause_hitl.rs`: `tokio::join!` holds the exec
  snapshot until the pause and HITL reads finish, creating an explicit stale
  result window even when the exec read itself completes promptly.
- `rust/fspec-tui/src/store/agent_view/exec_stdin_state.rs`: insertion/removal has
  no generation or tombstone. Dismissal does not invalidate in-flight probes.

Push request and clear actions preserve their order in the ordinary FIFO path.
The problematic stale pull must be applied after the derived dismiss action,
not merely after reception of the clear chunk.

## Previous executable validation

Real local commands through actual Codex facades verified pipe and PTY natural
exit, exit after `write_stdin`, clear delivery after turn callback cleanup,
`shell_command` completion, and no further suggestion during post-exit observation.
No live Codex API request was made.

Existing scoped tests also passed: the Codex facade lifecycle test, three
session detector-clear tests, and five TUI push tests.

Investigation logs:

- `/tmp/codex-stdin-facade-lifecycle.log`
- `/tmp/codex-stdin-pipe-pty-exit.log`
- `/tmp/codex-stdin-shell-command-exit.log`
- `/tmp/codex-stdin-session-exit-clear.log`
- `/tmp/codex-stdin-tui-stale-probe.log`

Temporary investigation tests were removed; permanent regressions must accompany
the fix. Local logs are supporting evidence, not version-controlled artifacts.

## Fix scope and acceptance expectations

Invalidate asynchronous exec-stdin probe results when newer per-session state
has been applied or another probe supersedes them. Cover focus probes and the
Paused join path, including stale `None` results that would hide a newer request.
Preserve ordered push updates, HITL precedence, user dismissal semantics,
per-session isolation, and prompts for commands still alive after turn `Done`.
Use shared freshness logic rather than parallel ad hoc checks in both probes.

Broadcast lag/reconnect can independently lose state-only events. The current
subscriber logs lag and continues without authoritative exec-state resync.
That is a separate reliability gap, not the demonstrated stale-probe defect;
do not claim this fix guarantees delivery across dropped events.

## BUG-209 implementation and validation

Both focus-return and Paused probes now carry a per-agent-session generation
from launch through action delivery. The reducer discards superseded results,
including empty snapshots. New probes, pushed requests, clears, user dismissals,
and successful stdin submission advance that session's generation. Empty slots
retain the generation so an old request cannot resurrect a cleared overlay.
Push chunks apply their transitions synchronously rather than enqueueing another
action, closing the additional queue-ordering window. Turn Done remains unchanged.

The permanent regression target is
`rust/fspec-tui/tests/exec_stdin_probe_freshness_bug209.rs`, linked one-to-one to
nine scenarios in `spec/features/exec-stdin-probe-freshness.feature`. Tests exercise
both actual probe triggers, hold their completed actions, and deliberately deliver
them after newer state. They cover stale present and empty snapshots, newer
requests, dismissal/submission, overlapping probes in both delivery orders,
session isolation, fresh results, HITL precedence, and live commands after Done.
These are deterministic TUI integration tests, not live Codex API traffic.

Before implementation, five race regressions failed with incorrect overlay
state; the four compatibility scenarios passed. After implementation:

- All 87 tests passed across seven explicitly scoped TUI targets:
  `exec_stdin_probe_freshness_bug209`, `exec_stdin_prompt_p2`,
  `exec_stdin_push_chunk_bug171`, `inline_hitl_prompt_rpc411`,
  `inline_pause_prompt_rpc406`, `pause_hitl_rpc053`, and `tui111_sanitize_ingress`.
- `cargo check -p codelet-fspec-tui` passed.
- `cargo clippy -p codelet-fspec-tui --lib --test exec_stdin_probe_freshness_bug209`
  passed with existing warnings outside the changed code.
- Changed Rust files passed rustfmt checks; both affected features passed syntax
  and isolated registry-tag validation. Coverage audit found all mappings valid.
- Repository-wide tag validation still reports 526 violating feature files.
  Package-wide formatting still flags three untouched files:
  `dispatch_session_lifecycle.rs`,
  `model_selector_refresh_forces_catalog_refetch_bug207.rs`, and
  `rpc437_session_cleared_tui_consumption.rs`. These were not changed by BUG-209.

Cargo runs used `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0`.
Saved logs: `/tmp/bug209-red.log`, `/tmp/bug209-final-tests.log`,
`/tmp/bug209-adjacent.log`, `/tmp/bug209-check.log`, `/tmp/bug209-clippy.log`,
`/tmp/bug209-tags.log`, and `/tmp/bug209-fmt.log`.
