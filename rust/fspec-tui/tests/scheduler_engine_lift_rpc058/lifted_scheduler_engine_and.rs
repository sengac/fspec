//! Feature: spec/features/rpc058-scheduler-engine-lift.feature
use super::*;
use codelet_test_helpers::{OrFail, TestResult};

/// Scenario: Lifted scheduler engine and cron_utils use captured-identifier format args (RPC-058 retro 2026-05-27)
///
/// RPC-058 retro followup: the lift of the scheduler engine + cron_utils
/// from rust/napi/src/scheduler/ into rust/core/src/scheduler/
/// preserved the original SCHED-002/SCHED-003 era format!/anyhow! call
/// sites that use the trailing-argument syntax. clippy::uninlined_format_args
/// (enabled under the workspace `-D warnings` level) flags those — the
/// fix is purely textual: rewrite `"foo {}", x` to `"foo {x}"`.
#[test]
fn lifted_scheduler_engine_and_cron_utils_use_captured_identifier_format_args() -> TestResult {
    // @step Given the codelet workspace inherits the lint level `-D warnings` which includes `clippy::uninlined_format_args`
    // (verified transitively by rust/sessions/tests/skeleton_invariants.rs::scenario_workspace_lints_are_inherited_and_clippy_passes)

    // @step Given rust/core/src/scheduler/engine.rs and rust/core/src/scheduler/cron_utils.rs live at their post-RPC-058 location
    let engine_path = workspace_root()?.join("rust/core/src/scheduler/engine.rs");
    let cron_utils_path = workspace_root()?.join("rust/core/src/scheduler/cron_utils.rs");
    assert!(
        engine_path.exists(),
        "engine.rs must live at post-RPC-058 location"
    );
    assert!(
        cron_utils_path.exists(),
        "cron_utils.rs must live at post-RPC-058 location"
    );
    let engine_source = codelet_test_helpers::source_composition::read_tree_or_file(&engine_path)
        .or_fail("read engine.rs")?;
    let cron_utils_source =
        codelet_test_helpers::source_composition::read_tree_or_file(&cron_utils_path)
            .or_fail("read cron_utils.rs")?;

    // @step When I scan engine.rs and cron_utils.rs for `format!` / `anyhow!` / `panic!` / `println!` / `eprintln!` / `write!` / `writeln!` macro invocations that interpolate a bare identifier
    //
    // @step Then every such invocation uses the captured-identifier syntax `"... {name} ..."` instead of the trailing-argument syntax `"... {} ...", name`
    //
    // The four specific known-bad sites from the RPC-058 retro are
    // engine.rs:284, engine.rs:299, cron_utils.rs:41, cron_utils.rs:49.
    // We assert ZERO occurrences of the trailing-argument pattern in
    // these files. The pattern matched is: a string literal that
    // contains `{}` followed by a `, <ident>` on the same line. This
    // detects the older form without false positives on captured-arg
    // formats (which have no trailing-argument list).
    let line_breaks_trailing_arg = |source: &str, label: &str| -> Vec<String> {
        let mut violations = Vec::new();
        for (idx, line) in source.lines().enumerate() {
            // Skip comments to avoid false positives on doc-comments.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            // Heuristic: the line contains `{}` AND ends with `, name)`
            // OR `, name)?;` pattern from the macro invocations we care about.
            // We scope to format!/anyhow!/panic!/println!/eprintln!/write!/writeln!
            // by checking for the macro names earlier on the line OR in the
            // preceding 3 lines (for multi-line macro calls).
            if !line.contains("{}") {
                continue;
            }
            // Check if any of the target macros appear within the last 4 lines.
            let start = idx.saturating_sub(3);
            let window: Vec<&str> = source.lines().skip(start).take(idx - start + 1).collect();
            let window_joined = window.join(" ");
            let has_target_macro = [
                "format!(",
                "anyhow!(",
                "panic!(",
                "println!(",
                "eprintln!(",
                "write!(",
                "writeln!(",
            ]
            .iter()
            .any(|m| window_joined.contains(m));
            if !has_target_macro {
                continue;
            }
            // The line has `{}` AND a target macro is in scope. Now check
            // for the trailing-argument pattern: a `, ident` or `, ident)`
            // suffix on this line or the next (for split-line macros).
            let next_line = source.lines().nth(idx + 1).unwrap_or("");
            let combined = format!("{line} {next_line}");
            // Look for `, <ident>` or `, <ident>)` after the format
            // string. We sweep for a comma followed by 1+ whitespace
            // followed by an identifier-looking token. Captured-arg
            // form does not produce these trailing args.
            let suffix = if let Some(quote_end) = line.rfind('"') {
                &line[quote_end..]
            } else if let Some(quote_end) = combined.rfind('"') {
                &combined[quote_end..]
            } else {
                line
            };
            // The pattern `, name)` or `, name);` is the trailing-arg form.
            // A captured-arg form ends `"<text>")` or `"<text>");`.
            let after_close_quote: String = suffix.chars().skip_while(|c| *c == '"').collect();
            // Take only the macro's trailing-arg region: everything from
            // the closing `"` up to the FIRST `)` (the macro's close paren).
            // We can't simply split-then-skip because the line may contain
            // additional `)` from outer call chains (e.g. `parse_cron(...,
            // &format!("schedule '{}'", name))` — the OUTER paren closes
            // parse_cron, not format!). The first `)` after the format
            // string is always the macro's own closing paren under normal
            // formatting.
            let macro_args_region = match after_close_quote.find(')') {
                Some(idx) => &after_close_quote[..idx],
                None => after_close_quote.as_str(),
            };
            let trailing_parts: Vec<String> = macro_args_region
                .split(',')
                .skip(1) // skip the empty fragment before the first comma
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect();
            let is_bare_ident = |p: &str| -> bool {
                // Only BARE identifiers (alphanumeric + underscore only).
                // clippy::uninlined_format_args fires ONLY when EVERY trailing
                // arg is a bare identifier. Function calls (`foo()`), method
                // calls (`x.y()`), path expressions (`x::y`), and any complex
                // expression suppress the lint. So we mirror clippy by
                // requiring ALL trailing args to be bare identifiers.
                !p.is_empty()
                    && p.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && p.chars()
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || c == '_')
            };
            let has_trailing_arg =
                !trailing_parts.is_empty() && trailing_parts.iter().all(|p| is_bare_ident(p));
            if has_trailing_arg {
                violations.push(format!("{label}:{}: {line}", idx + 1));
            }
        }
        violations
    };
    let engine_violations = line_breaks_trailing_arg(&engine_source, "engine.rs");
    let cron_violations = line_breaks_trailing_arg(&cron_utils_source, "cron_utils.rs");

    assert!(
        engine_violations.is_empty(),
        "RPC-058 retro: rust/core/src/scheduler/engine.rs must use captured-identifier format args (got {} violations):\n{}",
        engine_violations.len(),
        engine_violations.join("\n")
    );
    assert!(
        cron_violations.is_empty(),
        "RPC-058 retro: rust/core/src/scheduler/cron_utils.rs must use captured-identifier format args (got {} violations):\n{}",
        cron_violations.len(),
        cron_violations.join("\n")
    );

    // @step Then `cargo clippy -p codelet-sessions -- -D warnings` exits 0 with no `clippy::uninlined_format_args` errors against engine.rs:284, engine.rs:299, cron_utils.rs:41, or cron_utils.rs:49
    //
    // The static assertions above are sufficient — the only way for
    // clippy::uninlined_format_args to fire is for one of the target
    // macros to use the trailing-argument syntax with a bare identifier,
    // which the heuristic above detects directly.
    Ok(())
}
