//! Feature: spec/features/move-background-session-into-codelet-sessions.feature
use super::*;

/// BUG-298: type-check a workspace crate with the test's own ci-test
/// profile so the nested command reuses already-built artifacts instead
/// of compiling a second dev-profile copy of the dependency graph.
fn nested_check(package: &str) -> std::process::Output {
    Command::new(env!("CARGO"))
        .args([
            "check",
            "-p",
            package,
            "--profile",
            "ci-test",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo check must run")
}

/// The last 40 stderr lines, past the "Blocking waiting for file lock" noise.
fn stderr_tail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let lines: Vec<&str> = stderr.lines().collect();
    lines[lines.len().saturating_sub(40)..].join("\n")
}

#[test]
fn scenario_codelet_sessions_builds_standalone_with_background_session_at_its_new_home() {
    // @step Given the BackgroundSession struct and impl have been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    assert!(moved.contains("impl BackgroundSession"));
    // @step When I run `cargo check -p codelet-sessions --profile ci-test`
    let output = nested_check("codelet-sessions");
    // @step Then the check completes successfully with no errors
    assert!(output.status.success(), "{}", stderr_tail(&output));
    // @step And the public path `codelet_sessions::background_session::BackgroundSession` resolves to the moved struct
    let _shape_witness: fn() = || {
        fn _accept<T>(_: std::marker::PhantomData<T>) {}
        _accept::<BackgroundSession>(std::marker::PhantomData);
    };
}

#[test]
fn scenario_codelet_sessions_has_no_transitive_dependency_on_codelet_napi() {
    // @step Given the BackgroundSession code has been moved into rust/sessions/src/background_session.rs
    let moved = read(&moved_file_path());
    assert!(moved.contains("pub struct BackgroundSession"));
    // @step When I run `cargo metadata -p codelet-sessions --format-version 1`
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(workspace_root().join("Cargo.toml"))
        .output()
        .expect("cargo metadata must run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("metadata JSON must parse");
    let resolve = json
        .get("resolve")
        .expect("cargo metadata must include a resolve");
    let nodes = resolve
        .get("nodes")
        .and_then(|v| v.as_array())
        .expect("resolve.nodes must be an array");
    let packages = json
        .get("packages")
        .and_then(|v| v.as_array())
        .expect("packages must be an array");
    let sessions_id = packages
        .iter()
        .find(|p| p.get("name").and_then(|n| n.as_str()) == Some("codelet-sessions"))
        .and_then(|p| p.get("id").and_then(|i| i.as_str()))
        .expect("codelet-sessions package must exist in metadata")
        .to_string();
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut stack: Vec<String> = vec![sessions_id];
    while let Some(id) = stack.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        for node in nodes {
            if node.get("id").and_then(|i| i.as_str()) == Some(&id) {
                if let Some(deps) = node.get("dependencies").and_then(|d| d.as_array()) {
                    for d in deps {
                        if let Some(s) = d.as_str() {
                            stack.push(s.to_string());
                        }
                    }
                }
                break;
            }
        }
    }
    let mut transitive_names: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for id in &seen {
        if let Some(pkg) = packages
            .iter()
            .find(|p| p.get("id").and_then(|i| i.as_str()) == Some(id.as_str()))
        {
            if let Some(name) = pkg.get("name").and_then(|n| n.as_str()) {
                transitive_names.insert(name.to_string());
            }
        }
    }
    // @step Then the resulting JSON contains zero packages with name `codelet-napi`
    assert!(!transitive_names.contains("codelet-napi"));
}
