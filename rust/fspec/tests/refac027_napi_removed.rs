//! REFAC-027: remove the legacy codelet-napi crate.
//!
//! Feature: spec/features/remove-legacy-napi-crate.feature

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default()
}

/// Workspace member name -> its normal (non-dev) workspace dependencies.
fn dependency_graph() -> Result<BTreeMap<String, BTreeSet<String>>, Box<dyn std::error::Error>> {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--manifest-path",
        ])
        .arg(workspace_root().join("Cargo.toml"))
        .output()?;
    assert!(output.status.success(), "cargo metadata failed");
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let packages = metadata["packages"].as_array().ok_or("packages")?;
    let mut graph = BTreeMap::new();
    for package in packages {
        let name = package["name"].as_str().unwrap_or_default().to_string();
        let deps = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|dep| dep["kind"].is_null())
            .filter_map(|dep| dep["name"].as_str().map(str::to_string))
            .collect();
        graph.insert(name, deps);
    }
    Ok(graph)
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if path.is_dir() {
            if name != "target" && name != "node_modules" {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_workspace_has_no_napi_crate() -> TestResult {
    // @step Given the rust workspace manifest and every member manifest
    let root = workspace_root();

    // @step When the workspace members and their dependencies are listed
    let graph = dependency_graph()?;

    // @step Then no member is codelet-napi and no manifest depends on codelet-napi, napi or napi-derive
    assert!(
        !graph.contains_key("codelet-napi"),
        "codelet-napi is still a member"
    );
    for (member, deps) in &graph {
        for banned in ["codelet-napi", "napi", "napi-derive", "napi-build"] {
            assert!(!deps.contains(banned), "{member} still depends on {banned}");
        }
    }

    // @step And the rust/napi directory does not exist
    assert!(!root.join("napi").exists(), "rust/napi must be removed");
    Ok(())
}

#[test]
fn no_code_reads_napi_sources_or_gates_on_a_napi_feature() -> TestResult {
    // @step Given every Rust source and test file in the workspace
    let root = workspace_root();
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    let this_file = file!().rsplit('/').next().unwrap_or_default().to_string();
    // Built at runtime so this file does not match its own needles.
    let needles = [
        format!("rust/{}/", "napi"),
        format!("join(\"{}\")", "napi"),
        format!("feature = \"{}\"", "napi"),
    ];

    // @step When their non-comment code is scanned
    let mut offenders = Vec::new();
    for file in &files {
        if file.ends_with(&this_file) {
            continue;
        }
        let text = std::fs::read_to_string(file)?;
        for (number, line) in text.lines().enumerate() {
            let code = line.trim_start();
            if code.starts_with("//") {
                continue;
            }
            if needles.iter().any(|needle| code.contains(needle.as_str())) {
                offenders.push(format!("{}:{}", file.display(), number + 1));
            }
        }
    }

    // @step Then none reads a rust/napi path or uses feature = "napi"
    assert!(
        offenders.is_empty(),
        "napi references remain: {offenders:#?}"
    );
    Ok(())
}

#[test]
fn the_daemon_and_client_separation_still_holds() -> TestResult {
    // @step Given the workspace dependency graph
    let graph = dependency_graph()?;
    let deps = |name: &str| graph.get(name).cloned().unwrap_or_default();
    let engine = [
        "codelet-core",
        "codelet-sessions",
        "codelet-agent-loop",
        "codelet-tools",
        "codelet-providers",
    ];
    let service = "codelet-rpc";
    let transports = ["codelet-rpc-server", "codelet-rpc-embedded"];
    let client = "codelet-fspec-tui";

    // @step When the arrows between engine, service, transport and client crates are inspected
    for name in engine
        .iter()
        .chain([&service])
        .chain(transports.iter())
        .chain([&client])
    {
        assert!(
            graph.contains_key(*name),
            "{name} must still be a workspace member"
        );
    }

    // @step Then engine crates depend on no service, transport or client crate
    for crate_name in engine {
        let arrows = deps(crate_name);
        for forbidden in [service, transports[0], transports[1], client] {
            assert!(
                !arrows.contains(forbidden),
                "{crate_name} must not depend on {forbidden}"
            );
        }
    }

    // @step And the RPC service depends on no transport or client crate
    let arrows = deps(service);
    for forbidden in [transports[0], transports[1], client, "codelet-sessions"] {
        assert!(
            !arrows.contains(forbidden),
            "{service} must not depend on {forbidden}"
        );
    }

    // @step And the transports depend on no client crate
    for transport in transports {
        assert!(
            !deps(transport).contains(client),
            "{transport} must not depend on {client}"
        );
    }
    Ok(())
}
