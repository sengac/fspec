//! System-reminder formatting of hook results (HOOK-006).

use super::executor::HookResult;

/// Format one hook result for people and AI agents.
///
/// stdout is shown. A non-blocking hook's stderr is shown as-is, followed by
/// a one-line warning when it failed. A failing blocking hook adds a
/// `<system-reminder>` with `Hook: <name>`, `Exit code: <code>` and its
/// stderr (or `(Hook failed with no error output)`).
pub fn format_hook_output(result: &HookResult) -> String {
    let mut parts: Vec<String> = Vec::new();
    let stdout = result.stdout.trim_end();
    let stderr = result.stderr.trim_end();
    if !stdout.is_empty() {
        parts.push(stdout.to_string());
    }
    if !result.blocking {
        if !stderr.is_empty() {
            parts.push(stderr.to_string());
        }
        if !result.success() {
            parts.push(failure_line(result));
        }
    } else if !result.success() {
        let mut lines = vec![
            "<system-reminder>".to_string(),
            format!("Hook: {}", result.hook_name),
            format!("Exit code: {}", exit_code(result)),
        ];
        if result.timed_out {
            lines.push(failure_line(result));
        }
        lines.push(String::new());
        lines.push(if stderr.is_empty() {
            "(Hook failed with no error output)".to_string()
        } else {
            stderr.to_string()
        });
        lines.push("</system-reminder>".to_string());
        parts.push(lines.join("\n"));
    }
    parts.join("\n")
}

fn exit_code(result: &HookResult) -> String {
    result
        .exit_code
        .map_or_else(|| "none".to_string(), |code| code.to_string())
}

fn failure_line(result: &HookResult) -> String {
    if result.timed_out {
        format!(
            "Hook '{}' timed out after {}s",
            result.hook_name, result.timeout_secs
        )
    } else {
        format!(
            "Hook '{}' failed (exit code {})",
            result.hook_name,
            exit_code(result)
        )
    }
}
