@done
@BUG-362
@tools
@tool-execution
Feature: Exec stdin write acknowledgement
  """
  A write to a live unified_exec session's stdin is acknowledged with the real io result. Each write travels to the
  session's stdin forwarder as a StdinWrite (bytes plus a oneshot acknowledgement); the pipe forwarder
  (unified_exec/spawning/pipe.rs) replies with the write_all + flush result or "no stdin pipe", the PTY forwarder
  (spawning/pty.rs) replies with the blocking write result, a poisoned writer lock or a failed write task.
  unified_exec::write_stdin sends the write and awaits the acknowledgement; the unified_exec write action
  (tool/handlers.rs), the PTY control bytes (stdin_control.rs) and the session handle's write_exec_stdin
  (sessions handle_impl_4.rs) all use it, so a failed write is reported as an error naming the exec session instead of
  success. TUI-117 builds its red inline error on this.
  """

  Background: User Story
    As a user or agent answering a running command's input
    I want a failed stdin write to be reported as an error
    So that I know my answer never reached the command

  Scenario: Writing to a command that closed its stdin returns the write error
    Given a running exec session whose command closed its stdin
    When the write action sends "yes" to that session
    Then the write action fails with an error naming the exec session

  Scenario: Writing after the command's input was ended returns an error
    Given a running pipe exec session running "sleep 5"
    And its input was ended with an end-of-file control
    When the write action sends "yes" to that session
    Then the write action fails with an error naming the exec session
