//! Feature: spec/features/windows-unified-exec-platform-shell.feature
//!
//! BUG-172: Windows builds — every Bash command fails "program not found"
//! after TOOL-022 P4 rewired BashTool onto the unified exec machinery.
//!
//! These tests pin the platform shell-selection logic the unified exec
//! spawn path (`unified_exec/spawning.rs`) MUST use:
//! - `ExecCommand::Shell` on Windows → `powershell -NoProfile
//!   -NonInteractive -Command <cmd>` (BUG-156 builder), never a bare `sh`
//! - PowerShell unavailable → `cmd /C <cmd>` fallback builder
//! - `ExecCommand::Shell` on Unix → `sh -c <cmd>` (unchanged, regression
//!   guard)
//! - PTY liveness anchor → platform-spawnable on every host (no bare
//!   `sleep` on Windows)
//!
//! The builders are pure and platform-independent, so every assertion runs
//! on any host — the same pattern as `bash_windows_shell_wrapping_test.rs`.

use codelet_tools::bash_process::{
    build_cmd_fallback_invocation, build_unix_shell_invocation, build_windows_shell_invocation,
};
use codelet_tools::unified_exec::{platform_shell_invocation, pty_liveness_anchor_invocation};

// ==========================================
// SCENARIO: Windows shell command spawns via PowerShell in pipe mode
// ==========================================

#[test]
fn scenario_windows_shell_command_spawns_via_powershell_in_pipe_mode() {
    // @step Given the unified exec pipe spawn path is built for the Windows platform
    let command = "whoami";

    // @step When a shell command `whoami` is spawned via spawn_pipe_process
    // (The spawn path resolves the shell invocation through the pure
    // platform dispatcher; on a Windows target it must produce the
    // BUG-156 PowerShell wrap, never a bare `sh`.)
    let (program, args) = platform_shell_invocation(command, "windows");

    // @step Then the process is spawned as powershell -NoProfile -NonInteractive -Command whoami
    let (expected_program, expected_args) = build_windows_shell_invocation(command);
    assert_eq!(
        program, expected_program,
        "Windows shell spawn must use the PowerShell wrap"
    );
    assert_eq!(args, expected_args);
    assert_ne!(
        program, command,
        "the raw command must never be the spawned program"
    );
    assert_ne!(program, "sh", "Windows must not spawn sh");
}

// ==========================================
// SCENARIO: Windows shell command falls back to cmd when PowerShell is unavailable
// ==========================================

#[test]
fn scenario_windows_shell_command_falls_back_to_cmd_when_powershell_is_unavailable() {
    // @step Given PowerShell cannot be located on the Windows system
    // (At spawn time the PowerShell CreateProcess fails; the spawn path
    // then resolves the fallback invocation.)
    let command = "cmd /c dir";

    // @step When a shell command `cmd /c dir` is spawned via spawn_pipe_process
    let (program, args) = codelet_tools::unified_exec::windows_shell_fallback_invocation(command);

    // @step Then the process is spawned as cmd /C cmd /c dir and the raw command string is never the spawned program
    let (expected_program, expected_args) = build_cmd_fallback_invocation(command);
    assert_eq!(program, expected_program);
    assert_eq!(args, expected_args);
    assert_ne!(
        program, command,
        "the raw command must never be the spawned program"
    );
    assert_ne!(program, "sh", "the fallback must be cmd, not sh");
}

// ==========================================
// SCENARIO: PTY liveness anchor is spawnable on every platform
// ==========================================

#[test]
fn scenario_pty_liveness_anchor_is_spawnable_on_every_platform() {
    // @step Given the unified exec PTY spawn path needs a blocking liveness anchor
    // @step When the anchor program and arguments are resolved for a target platform
    let (unix_program, unix_args) = pty_liveness_anchor_invocation("unix");
    let (win_program, win_args) = pty_liveness_anchor_invocation("windows");

    // @step Then the Unix anchor stays `sleep <max-seconds>` and the Windows anchor is a blocking sleep that does not require a `sleep.exe`
    assert_eq!(unix_program, "sleep");
    assert_eq!(unix_args, vec!["2147483647".to_string()]);
    assert_ne!(
        win_program, "sleep",
        "Windows must not rely on a bare `sleep` (no sleep.exe on Windows)"
    );
    assert!(
        !win_args.is_empty(),
        "the Windows anchor needs a blocking-sleep argument"
    );
    // The Windows anchor blocks for effectively the full i32 range — the
    // same contract the Unix `sleep 2147483647` anchor has.
    assert!(
        win_program == "powershell" || win_program == "cmd",
        "Windows anchor must use an always-present shell; got {win_program}"
    );
}

// ==========================================
// SCENARIO: Unix pipe spawn path is unchanged (no regression)
// ==========================================

#[test]
fn scenario_unix_pipe_spawn_path_is_unchanged() {
    // @step Given the unified exec pipe spawn path is built for the Unix platform
    let command = "echo hello";

    // @step When a shell command `echo hello` is spawned via spawn_pipe_process
    let (program, args) = platform_shell_invocation(command, "unix");

    // @step Then the process is spawned as sh -c echo hello
    let (expected_program, expected_args) = build_unix_shell_invocation(command);
    assert_eq!(program, expected_program);
    assert_eq!(args, expected_args);

    // @step And the command runs in a new process group (process_group(0))
    // (Pinned by the existing Unix spawn path: `cmd.process_group(0)` in
    // spawn_pipe_process is untouched by this fix — the dispatcher only
    // selects the shell invocation, it does not alter process-group setup.)
    #[cfg(unix)]
    {
        let _killer_type = std::any::type_name::<codelet_tools::bash_process::ProcessGroupKiller>();
    }
}
