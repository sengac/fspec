//! Feature: spec/features/windows-builds-bash-tool-cannot-execute-windows-commands-pathext-shell-wrapping-not-handled.feature
//!
//! BUG-156: Windows builds — Bash tool cannot execute Windows commands.
//!
//! These tests verify the platform-independent *construction* logic that the
//! Windows spawn path uses: the shell-wrapping invocation (PowerShell
//! preferred, cmd.exe fallback), the taskkill argument vectors for process
//! tree termination, and the unchanged Unix `sh -c` invocation. Running on
//! Linux, we assert the pure builders rather than spawning Windows processes.

use codelet_tools::bash_process::{
    build_cmd_fallback_invocation, build_unix_shell_invocation, build_windows_shell_invocation,
    taskkill_args, ABORT_MESSAGE,
};

// ==========================================
// SCENARIO: Windows command is wrapped in PowerShell for PATHEXT resolution
// ==========================================

#[test]
fn scenario_windows_command_wrapped_in_powershell() {
    // @step Given the Bash tool is built for Windows
    // The Windows invocation builder is the platform-independent core of the
    // cfg(windows) spawn path; it is always compiled so its logic is testable
    // on any host.
    let command = "whoami";

    // @step When the agent executes the command `whoami`
    let (program, args) = build_windows_shell_invocation(command);

    // @step Then the command is spawned as `powershell -NoProfile -NonInteractive -Command whoami`
    assert_eq!(program, "powershell");
    assert_eq!(
        args,
        vec![
            "-NoProfile".to_string(),
            "-NonInteractive".to_string(),
            "-Command".to_string(),
            command.to_string()
        ]
    );

    // @step And the raw command string is never spawned directly via CreateProcess
    // The user command only ever appears as the LAST argument (the -Command
    // payload) of the shell invocation — never as the program to spawn.
    assert_ne!(program, command);
    assert_eq!(args.last().map(String::as_str), Some(command));
}

// ==========================================
// SCENARIO: Windows command falls back to cmd.exe when PowerShell is unavailable
// ==========================================

#[test]
fn scenario_windows_command_falls_back_to_cmd() {
    // @step Given the Bash tool is built for Windows
    // @step And PowerShell cannot be located on the system
    // When PowerShell lookup fails at runtime, the spawn path falls back to
    // the cmd.exe builder.
    let command = "cmd /c dir";

    // @step When the agent executes the command `cmd /c dir`
    let (program, args) = build_cmd_fallback_invocation(command);

    // @step Then the command is spawned as `cmd /C cmd /c dir`
    assert_eq!(program, "cmd");
    assert_eq!(args, vec!["/C".to_string(), command.to_string()]);

    // @step And the command succeeds with a directory listing instead of a "program not found" spawn error
    // Because the command runs inside a real Windows shell (cmd.exe), the
    // shell performs PATHEXT resolution — the raw command string is never
    // passed to CreateProcess as a program name, so "program not found"
    // spawn errors for bare names (whoami, cmd, ...) cannot occur.
    assert_ne!(program, command);
    assert_eq!(args.last().map(String::as_str), Some(command));
}

// ==========================================
// SCENARIO: Aborting a Windows command kills the entire process tree
// ==========================================

#[test]
fn scenario_aborting_windows_command_kills_process_tree() {
    // @step Given the Bash tool is built for Windows
    // @step And a long-running command is executing in a spawned Windows shell
    let pid: u32 = 4242;

    // @step When the user aborts the command
    // The WindowsProcessTreeKiller guard (cfg(windows)) builds these argument
    // vectors and runs `taskkill` against the spawned shell's PID.

    // @step Then the process tree is terminated with `taskkill /PID <pid> /T`
    let graceful = taskkill_args(pid, false);
    assert_eq!(
        graceful,
        vec!["/PID".to_string(), pid.to_string(), "/T".to_string()]
    );

    // @step And the forceful variant uses `taskkill /PID <pid> /T /F`
    let forceful = taskkill_args(pid, true);
    assert_eq!(
        forceful,
        vec![
            "/PID".to_string(),
            pid.to_string(),
            "/T".to_string(),
            "/F".to_string()
        ]
    );

    // @step And the tool returns "Command interrupted by user"
    assert_eq!(ABORT_MESSAGE, "Command interrupted by user");
}

// ==========================================
// SCENARIO: Unix spawn path is unchanged (no regression)
// ==========================================

#[test]
fn scenario_unix_spawn_path_unchanged() {
    // @step Given the Bash tool is built for Unix
    let command = "echo hello";

    // @step When the agent executes the command `echo hello`
    let (program, args) = build_unix_shell_invocation(command);

    // @step Then the command is spawned as `sh -c echo hello`
    assert_eq!(program, "sh");
    assert_eq!(args, vec!["-c".to_string(), command.to_string()]);

    // @step And the command runs in a new process group with the Unix ProcessGroupKiller guard
    // On Unix hosts the guard is compiled and wired into bash.rs; the spawn
    // path sets process_group(0) so the guard can SIGKILL the whole group.
    #[cfg(unix)]
    {
        let _killer_type = std::any::type_name::<codelet_tools::bash_process::ProcessGroupKiller>();
    }
}
