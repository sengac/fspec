@done
@tool-execution
@windows
@tools
@BUG-156
Feature: Windows builds: Bash tool cannot execute Windows commands (PATHEXT / shell wrapping not handled)
  """
  Adopts the VTCode (github.com/vinhnx/VTCode, commit 9118afcfd) patterns: vtcode-bash-runner executor.rs (powershell -NoProfile -NonInteractive -Command vs sh -c), process_group.rs (taskkill /PID <pid> /T graceful, /T /F forceful), and terminal_app.rs (target_os=windows cmd /C vs /bin/sh -lc branching). Windows command construction is extracted into a pure, unit-testable function (build_windows_shell_invocation) plus a cfg(windows) WindowsProcessTreeKiller guard mirroring ProcessGroupKiller; wired into bash.rs and bash_streams::wait_for_tasks_with_abort. Unix path untouched.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. On Windows, every user command MUST be wrapped in a real Windows shell before spawning: prefer `powershell -NoProfile -NonInteractive -Command <cmd>`; if PowerShell cannot be located, fall back to `cmd /C <cmd>`. The raw command string must NEVER be spawned directly via CreateProcess.
  #   2. On Windows there are no process groups; the process tree MUST be killed with `taskkill /PID <pid> /T` (graceful) and `taskkill /PID <pid> /T /F` (forceful), mirroring the Unix ProcessGroupKiller guard pattern (guard struct with kill() and Drop impl, cfg(windows)).
  #   3. The Unix spawn path (`sh -c` with process_group(0) + ProcessGroupKiller) MUST remain byte-for-byte unchanged — no Unix regression.
  #   4. Because the command runs inside a real Windows shell, the shell performs PATHEXT resolution, so bare command names like `whoami` resolve to `whoami.exe` and `cmd` resolves to `cmd.exe`.
  #
  # EXAMPLES:
  #   1. On Windows, the agent runs `whoami` via the Bash tool: the command is spawned as `powershell -NoProfile -NonInteractive -Command whoami` and succeeds, printing the user identity (whoami.exe resolved by PATHEXT).
  #   2. On Windows, the agent runs `cmd /c dir` via the Bash tool and gets a directory listing instead of a "program not found" spawn error.
  #   3. On Windows, the user aborts a long-running command: the entire process tree (shell plus any children it spawned) is terminated via taskkill /PID <pid> /T, and the tool returns "Command interrupted by user".
  #   4. On Linux, the agent runs `echo hello` via the Bash tool exactly as before: it is spawned via `sh -c` in a new process group and prints `hello` — the Unix behavior is unchanged by this fix.
  #
  # ========================================
  Background: User Story
    As a developer or AI agent using the Bash tool on a Windows build
    I want to execute Windows shell commands (e.g. whoami, dir, PowerShell syntax) via the Bash tool
    So that Windows builds can actually run commands instead of failing because sh does not exist and PATHEXT is not resolved

  Scenario: Windows command is wrapped in PowerShell for PATHEXT resolution
    Given the Bash tool is built for Windows
    When the agent executes the command `whoami`
    Then the command is spawned as `powershell -NoProfile -NonInteractive -Command whoami`
    And the raw command string is never spawned directly via CreateProcess

  Scenario: Windows command falls back to cmd.exe when PowerShell is unavailable
    Given the Bash tool is built for Windows
    And PowerShell cannot be located on the system
    When the agent executes the command `cmd /c dir`
    Then the command is spawned as `cmd /C cmd /c dir`
    And the command succeeds with a directory listing instead of a "program not found" spawn error

  Scenario: Aborting a Windows command kills the entire process tree
    Given the Bash tool is built for Windows
    And a long-running command is executing in a spawned Windows shell
    When the user aborts the command
    Then the process tree is terminated with `taskkill /PID <pid> /T`
    And the forceful variant uses `taskkill /PID <pid> /T /F`
    And the tool returns "Command interrupted by user"

  Scenario: Unix spawn path is unchanged (no regression)
    Given the Bash tool is built for Unix
    When the agent executes the command `echo hello`
    Then the command is spawned as `sh -c echo hello`
    And the command runs in a new process group with the Unix ProcessGroupKiller guard
