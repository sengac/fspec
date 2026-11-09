//! REFAC-032: the dynamic MCP module is split into small files.
//!
//! Feature: spec/features/mcp-module-split.feature

use codelet_tools::mcp::{
    cleanup_mcp_session, get_mcp_connections, init_mcp_session, parse_mcp_tool_name,
    qualified_mcp_tool_name,
};
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `mcp.rs` plus every Rust file under the `mcp/` directory.
fn mcp_files(src: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = vec![src.join("mcp.rs")];
    for entry in std::fs::read_dir(src.join("mcp"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

#[test]
fn dynamic_mcp_source_files_stay_under_300_lines() -> TestResult {
    // @step Given the dynamic MCP source files in rust/tools/src
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = mcp_files(&src)?;

    // @step When the line count of each file is measured
    let mut oversized = Vec::new();
    for file in &files {
        let lines = std::fs::read_to_string(file)?.lines().count();
        if lines >= 300 {
            oversized.push(format!("{} ({lines})", file.display()));
        }
    }

    // @step Then no file has 300 or more lines
    assert!(oversized.is_empty(), "oversized files: {oversized:?}");
    Ok(())
}

#[tokio::test]
async fn the_dynamic_mcp_surface_is_unchanged() -> TestResult {
    // @step Given code that names the MCP session registry and tool-name helpers through codelet_tools::mcp
    let session = uuid::Uuid::new_v4();

    // @step When a session's MCP state is initialised, queried and cleaned up and a tool name is qualified and parsed
    let (_injections, connections) = init_mcp_session(session);
    let registered = get_mcp_connections(session).ok_or("session must be registered")?;
    let empty = registered.read().await.is_empty();
    cleanup_mcp_session(session);
    let after_cleanup = get_mcp_connections(session);
    let qualified = qualified_mcp_tool_name("sonar", "analyze");

    // @step Then the registry reports the session's empty connection map and then none
    assert!(empty && connections.read().await.is_empty());
    assert!(after_cleanup.is_none());

    // @step And the qualified name parses back to its server and tool
    assert_eq!(parse_mcp_tool_name(&qualified), Some(("sonar", "analyze")));
    Ok(())
}

/// Whether `source` defines a function called `name` (sync or async, any visibility).
fn defines_fn(source: &str, name: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim_start();
        let line = line.strip_prefix("pub ").map_or(line, str::trim_start);
        let line = line.strip_prefix("async ").map_or(line, str::trim_start);
        line.strip_prefix("fn ")
            .is_some_and(|rest| rest.starts_with(&format!("{name}(")))
    })
}

#[test]
fn the_host_mcp_configurations_per_server_connect_helpers_live_in_their_own_fragment() -> TestResult
{
    // @step Given the host MCP configuration source rust/tools/src/mcp/host_config.rs
    let mcp = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("mcp");
    let host_config = std::fs::read_to_string(mcp.join("host_config.rs"))?;

    // @step When its per-server connect helpers are looked up
    let fragment = std::fs::read_to_string(mcp.join("host_establish.rs"));

    // @step Then host_config.rs includes host_establish.rs and defines neither establish nor redact
    assert!(host_config.contains("include!(\"host_establish.rs\");"));
    assert!(!defines_fn(&host_config, "establish") && !defines_fn(&host_config, "redact"));

    // @step And host_establish.rs defines both establish and redact
    let fragment = fragment?;
    assert!(defines_fn(&fragment, "establish") && defines_fn(&fragment, "redact"));
    Ok(())
}

#[test]
fn the_connect_mcp_tool_server_handle_helpers_live_in_their_own_fragment() -> TestResult {
    // @step Given the ConnectMCP tool source rust/tools/src/mcp/connect_tool.rs
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let connect_tool = std::fs::read_to_string(src.join("mcp").join("connect_tool.rs"))?;

    // @step When its tool-server handle helpers are looked up
    let mcp = std::fs::read_to_string(src.join("mcp.rs"))?;
    let fragment = std::fs::read_to_string(src.join("mcp").join("handle_registration.rs"));

    // @step Then mcp.rs includes handle_registration.rs and connect_tool.rs defines neither add_wrappers_to_handle nor remove_server_tools_from_handle
    assert!(mcp.contains("include!(\"mcp/handle_registration.rs\");"));
    assert!(
        !defines_fn(&connect_tool, "add_wrappers_to_handle")
            && !defines_fn(&connect_tool, "remove_server_tools_from_handle")
    );

    // @step And handle_registration.rs defines both add_wrappers_to_handle and remove_server_tools_from_handle
    let fragment = fragment?;
    assert!(
        defines_fn(&fragment, "add_wrappers_to_handle")
            && defines_fn(&fragment, "remove_server_tools_from_handle")
    );
    Ok(())
}
