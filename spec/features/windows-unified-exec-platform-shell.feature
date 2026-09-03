@done
@windows
@tools
@tool-execution
@BUG-172
Feature: Windows builds: every Bash command fails 'program not found' after TOOL-022 P4 rewired BashTool onto unified_exec
  """
  BUG-156 introduced the platform invocation builders (build_windows_shell_invocation -> powershell -NoProfile -NonInteractive -Command <cmd>, build_cmd_fallback_invocation -> cmd /C <cmd>, build_unix_shell_invocation -> sh -c <cmd>) in rust/tools/src/bash_process.rs. TOOL-022 P4 rewired BashTool onto the unified exec machinery (unified_exec/spawning.rs), which hardcodes `sh` for every platform and now ignores those builders. This fix routes ExecCommand::Shell through the existing BUG-156 builders in spawn_pipe_process and spawn_pty_process, and replaces the bare `sleep` PTY liveness anchor with a platform-spawnable sleep. No new shell logic is invented; the pure builders are reused and re-wired onto the live spawn path.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. R2: The Unix spawn path MUST remain sh -c <command> with process_group(0) in pipe mode — byte-for-byte unchanged, no Unix regression.
  #   2. R1: On Windows, every ExecCommand::Shell (pipe AND PTY spawn paths) MUST be wrapped in a real Windows shell — powershell -NoProfile -NonInteractive -Command <cmd> preferred, cmd /C <cmd> as the fallback when PowerShell cannot be located — using the existing BUG-156 invocation builders; the raw command string must NEVER be the CreateProcess program.
  #   3. R4: ExecCommand::Argv MUST remain a direct bare-program spawn (no shell wrapping) on all platforms — CreateProcess resolves PATHEXT for bare names on Windows.
  #   4. R3: The PTY liveness anchor MUST be spawnable on every platform (no bare `sleep` on Windows — a platform-appropriate blocking sleep substitute).
  #
  # EXAMPLES:
  #   1. On Windows, the agent runs `whoami` via the Bash tool (pipe mode): the process is spawned as powershell -NoProfile -NonInteractive -Command whoami and exits 0 with the user identity — not 'Failed to spawn: program not found'.
  #   2. On Windows with PowerShell unavailable, `cmd /c dir` spawns as cmd /C <cmd /c dir> and returns a directory listing (cmd fallback).
  #   3. On Linux, the agent runs `echo hello` via the Bash tool exactly as before: it is spawned via sh -c in a new process group and prints hello — the Unix behavior is unchanged by this fix.
  #   4. On Windows, a tty=true run of `cat` opens a PTY session: the PTY child is the Windows shell (cmd /C cat) and the liveness anchor is a Windows-spawnable blocking sleep — no 'program not found' for sh or sleep at spawn time.
  #
  # ========================================
  Background: User Story
    As a developer or AI agent using a Windows build
    I want to execute commands via the Bash tool (unified exec pipe and PTY paths)
    So that commands actually run instead of failing 'program not found' at spawn

  Scenario: Windows shell command falls back to cmd when PowerShell is unavailable
    Given PowerShell cannot be located on the Windows system
    When a shell command `cmd /c dir` is spawned via spawn_pipe_process
    Then the process is spawned as cmd /C cmd /c dir and the raw command string is never the spawned program

  Scenario: Windows shell command spawns via PowerShell in pipe mode
    Given the unified exec pipe spawn path is built for the Windows platform
    When a shell command `whoami` is spawned via spawn_pipe_process
    Then the process is spawned as powershell -NoProfile -NonInteractive -Command whoami

  Scenario: PTY liveness anchor is spawnable on every platform
    Given the unified exec PTY spawn path needs a blocking liveness anchor
    When the anchor program and arguments are resolved for a target platform
    Then the Unix anchor stays `sleep <max-seconds>` and the Windows anchor is a blocking sleep that does not require a `sleep.exe`

  Scenario: Unix pipe spawn path is unchanged (no regression)
    Given the unified exec pipe spawn path is built for the Unix platform
    When a shell command `echo hello` is spawned via spawn_pipe_process
    Then the process is spawned as sh -c echo hello
    And the command runs in a new process group (process_group(0))
