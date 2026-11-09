@done
@BUG-365
@tools
@tool-execution
@rust
Feature: Tools signal processes without unsafe code
  """
  rust/tools/src/bash_process.rs (ProcessGroupKiller), unified_exec/spawning.rs
  (build_kill_handle) and unified_exec/spawning/pty.rs (kill_anchor_pid)
  wrapped libc::kill in #[allow(unsafe_code)] against the workspace deny, and
  kill_anchor_pid dropped the result. process_signal.rs now signals through
  nix's safe kill/killpg, returns Signalled or AlreadyGone, and returns real
  failures as errors that the callers log.
  Tests: rust/tools/tests/bug365_safe_process_signalling.rs.
  """

  Background: User Story
    As a developer of the tools crate
    I want process signalling to go through a safe API that reports its result
    So that the workspace unsafe_code deny holds and kill failures are never hidden

  Scenario: Killing a running process group terminates it
    Given a running process started in its own process group
    When its process group is killed
    Then the kill reports the group was signalled and the process exits

  Scenario: Signalling a process that already exited reports it was gone
    Given a process that has already exited and been reaped
    When it is killed
    Then the kill reports the process was already gone

  Scenario: The tools sources contain no unsafe code suppressions
    Given the tools crate sources
    When they are scanned for unsafe code suppressions
    Then no source file allows unsafe_code
