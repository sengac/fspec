//! Feature: spec/features/client-mcp-read-only.feature
#[path = "../src/client_mcp_service.rs"]
mod client_mcp_service;

use client_mcp_service::{validate_daemon_url, McpFrontend, ReadOperation};
use rmcp::{
    model::{CallToolRequestParams, ClientInfo},
    ServiceExt,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

// Scenario: Discover and call the read-only tools
#[tokio::test]
async fn discover_and_call_read_only_tools() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
    // @step Given a service client connected over MCP to a legacy daemon
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let server = McpFrontend::new(move |operation| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            Ok(match operation {
                ReadOperation::Health => json!({"healthy": true}),
                ReadOperation::BoundedWorkUnits { .. } => json!(null),
                ReadOperation::WorkUnits => json!([{"id": "TEST-1"}]),
            })
        })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    // @step When it lists tools and requests work units
    let tools = client.list_all_tools().await?;
    let result = client
        .call_tool(CallToolRequestParams::new("fspec_work_units"))
        .await?;
    // @step Then only health and work-unit reads are advertised
    let mut names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
    names.sort();
    assert_eq!(
        names,
        ["fspec_health", "fspec_work_unit_detail", "fspec_work_units"]
    );
    assert!(tools
        .iter()
        .all(|t| t.annotations.as_ref().and_then(|a| a.read_only_hint) == Some(true)));
    // @step And the selected read reaches the configured backend
    assert_eq!(result.is_error, Some(false));
    assert!(serde_json::to_string(&result)?.contains("TEST-1"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    client.cancel().await?;
    task.await??;
    Ok(())
}

// Scenario: Reject mutations and unexpected arguments
#[tokio::test]
async fn reject_mutations_and_unexpected_arguments(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @step Given a service client connected over MCP to a legacy daemon
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let server = McpFrontend::new(move |_| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(json!({})) })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    // @step When it requests send_input or supplies unexpected arguments
    let mutation = client
        .call_tool(CallToolRequestParams::new("send_input"))
        .await;
    let args = serde_json::from_value(json!({"command": "delete"}))?;
    let unexpected = client
        .call_tool(CallToolRequestParams::new("fspec_health").with_arguments(args))
        .await;
    // @step Then the request is rejected before reaching the backend
    assert!(mutation.is_err());
    assert!(unexpected.is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    client.cancel().await?;
    task.await??;
    Ok(())
}

// Scenario: Contain backend failures
#[tokio::test]
async fn contain_backend_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @step Given a backend that fails or returns excessive output
    for excessive in [false, true] {
        let server = McpFrontend::new(move |_| {
            Box::pin(async move {
                if excessive {
                    Ok(json!("x".repeat(300_000)))
                } else {
                    Err(())
                }
            })
        });
        let (a, b) = tokio::io::duplex(8192);
        let task = tokio::spawn(async move {
            server.serve(a).await?.waiting().await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });
        let client = ClientInfo::default().serve(b).await?;
        // @step When the service client requests daemon health
        let result = client
            .call_tool(CallToolRequestParams::new("fspec_health"))
            .await?;
        // @step Then it receives a bounded tool error without backend diagnostics
        assert_eq!(result.is_error, Some(true));
        let wire = serde_json::to_string(&result)?;
        assert!(wire.len() < 1000);
        assert!(!wire.contains("xxxx"), "{wire}");
        client.cancel().await?;
        task.await??;
    }
    Ok(())
}

// Scenario: Reject unprotected remote daemon URLs
#[test]
fn reject_unprotected_remote_daemon_urls() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @step Given an explicitly configured daemon URL
    // @step When MCP client mode validates that URL
    // @step Then only plain WebSocket URLs with numeric loopback hosts and no credentials are accepted
    for good in ["ws://127.0.0.1:1234", "ws://[::1]:1234"] {
        assert!(validate_daemon_url(&url::Url::parse(good)?).is_ok());
    }
    for bad in [
        "ws://example.com",
        "ws://localhost",
        "ws://192.168.1.2",
        "wss://127.0.0.1",
        "ws://user:pass@127.0.0.1",
        "ws://127.0.0.1/?token=secret",
    ] {
        assert!(validate_daemon_url(&url::Url::parse(bad)?).is_err());
    }
    Ok(())
}

// Scenario: Bound stalled daemon calls
#[tokio::test(start_paused = true)]
async fn bound_stalled_daemon_calls() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @step Given a backend that never finishes a read
    let server = McpFrontend::new(|_| Box::pin(std::future::pending()));
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let start = tokio::time::Instant::now();
    // @step When the service client requests daemon health
    let result = client
        .call_tool(CallToolRequestParams::new("fspec_health"))
        .await?;
    // @step Then it receives a tool error after the deadline
    assert_eq!(result.is_error, Some(true));
    assert!(start.elapsed() >= std::time::Duration::from_secs(10));
    client.cancel().await?;
    task.await??;
    Ok(())
}

// Scenario: Page a large workspace without exposing full descriptions
#[tokio::test]
async fn page_oversized_workspace() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @step Given work units whose complete response exceeds the output limit
    let server = McpFrontend::new(|_| {
        Box::pin(async {
            Ok(json!((0..75)
                .rev()
                .map(|i| json!({"id": format!("WU-{i:03}"),
            "title": "😀".repeat(1000), "status": "doing", "workType": "task",
            "description": "private".repeat(10000)}))
                .collect::<Vec<_>>()))
        })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let mut cursor = None;
    let mut revision = None;
    let mut ids = Vec::new();
    // @step When the client requests successive bounded summary pages
    for count in [50, 25] {
        let mut args = json!({"limit": 50});
        if let Some(value) = cursor {
            args["cursor"] = value;
            args["revision"] = revision.clone().ok_or("revision missing")?;
        }
        let result = client
            .call_tool(
                CallToolRequestParams::new("fspec_work_units")
                    .with_arguments(serde_json::from_value(args)?),
            )
            .await?;
        assert_eq!(result.is_error, Some(false));
        let wire = serde_json::to_value(&result)?;
        let text = wire["content"][0]["text"].as_str().ok_or("missing text")?;
        // @step And descriptions are omitted and summary strings are bounded
        assert!(text.len() < 256 * 1024);
        assert!(!text.contains("private"));
        let page: serde_json::Value = serde_json::from_str(text)?;
        let items = page["items"].as_array().ok_or("missing items")?;
        assert_eq!(items.len(), count);
        assert_eq!(page["total"], 75);
        assert!(items[0]["title"].as_str().ok_or("title")?.chars().count() <= 256);
        ids.extend(items.iter().map(|item| item["id"].clone()));
        cursor = Some(page["next_cursor"].clone());
        revision = Some(page["revision"].clone());
        assert_eq!(page["returned_count"], count);
        assert_eq!(items[0]["truncated_fields"], json!(["title"]));
    }
    // @step Then results are ordered by unique ID with a continuation cursor
    assert_eq!(ids.len(), 75);
    assert_eq!(ids[0], "WU-000");
    assert_eq!(ids[74], "WU-074");
    assert_eq!(cursor, Some(serde_json::Value::Null));
    // @step And invalid limits and cursors are rejected
    for args in [
        json!({"limit": 0}),
        json!({"limit": 51}),
        json!({"limit": 1.5}),
        json!({"cursor": ""}),
        json!({"cursor": "unknown"}),
        json!({"cursor": 1}),
        json!({"cursor": "x".repeat(513)}),
        json!({"extra": true}),
    ] {
        assert!(client
            .call_tool(
                CallToolRequestParams::new("fspec_work_units")
                    .with_arguments(serde_json::from_value(args)?)
            )
            .await
            .is_err());
    }
    client.cancel().await?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn negotiated_backend_pages_are_not_paged_twice(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = McpFrontend::new(|op| Box::pin(async move {
        match op {
            ReadOperation::BoundedWorkUnits { detail, arguments } => {
                assert!(!detail);
                assert_eq!(serde_json::from_str::<serde_json::Value>(&arguments).map_err(|_| ())?["limit"], 1);
                Ok(json!({"items":[{"id":"A"}],"returned_count":1,"backend_fetch":"bounded_rpc"}))
            }
            _ => Err(()),
        }
    })).with_bounded_rpc(true);
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let result = client
        .call_tool(
            CallToolRequestParams::new("fspec_work_units")
                .with_arguments(serde_json::from_value(json!({"limit":1}))?),
        )
        .await?;
    assert_eq!(result.is_error, Some(false));
    assert!(serde_json::to_string(&result)?.contains("bounded_rpc"));
    client.cancel().await?;
    task.await??;
    Ok(())
}

/// Call `fspec_work_unit_detail` with `args`; the decoded chunk.
async fn detail_chunk(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>,
    args: serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let result = client
        .call_tool(
            CallToolRequestParams::new("fspec_work_unit_detail")
                .with_arguments(serde_json::from_value(args)?),
        )
        .await?;
    assert_eq!(result.is_error, Some(false));
    let wire = serde_json::to_value(&result)?;
    let text = wire["content"][0]["text"].as_str().ok_or("missing text")?;
    assert!(
        text.len() < 256 * 1024,
        "a chunk stays under the output limit"
    );
    Ok(serde_json::from_str(text)?)
}

// Scenario: Read complete details in explicit chunks
#[tokio::test]
async fn read_complete_details_in_explicit_chunks(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @step Given a record with oversized Unicode text
    let record = json!({"id": "WU-1", "title": "big", "status": "doing",
        "description": format!("{}é{}", "😀".repeat(6000), "日本".repeat(2000))});
    let records = Arc::new(std::sync::Mutex::new(json!([record.clone()])));
    let backend = Arc::clone(&records);
    let server = McpFrontend::new(move |_| {
        let snapshot = backend.lock().map(|value| value.clone()).map_err(|_| ());
        Box::pin(async move { snapshot })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    // @step When the client follows detail offsets and revisions
    let mut chunks = vec![detail_chunk(&client, json!({"id": "WU-1"})).await?];
    while let Some(next) = chunks.last().and_then(|c| c["next_offset"].as_u64()) {
        let revision = chunks[0]["revision"].clone();
        chunks.push(
            detail_chunk(
                &client,
                json!({"id": "WU-1", "offset": next, "revision": revision}),
            )
            .await?,
        );
    }
    // @step Then each response is valid bounded JSON with explicit partial indicators
    assert!(chunks.len() > 1, "the record must need several chunks");
    for chunk in &chunks {
        assert_eq!(chunk["partial"], true);
        assert_eq!(chunk["offset_unit"], "unicode_scalar");
        assert!(chunk["text"].as_str().ok_or("text")?.chars().count() <= 4096);
    }
    assert_eq!(chunks.last().ok_or("chunk")?["next_offset"], json!(null));
    // @step And concatenated text reconstructs the complete record
    let text: String = chunks
        .iter()
        .map(|chunk| chunk["text"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(serde_json::from_str::<serde_json::Value>(&text)?, record);
    // @step And changed records reject continuation
    if let Ok(mut value) = records.lock() {
        value[0]["title"] = json!("changed");
    }
    let stale = client
        .call_tool(
            CallToolRequestParams::new("fspec_work_unit_detail").with_arguments(
                serde_json::from_value(json!({"id": "WU-1", "offset": 4096,
                    "revision": chunks[0]["revision"].clone()}))?,
            ),
        )
        .await;
    assert!(stale.is_err(), "a changed record must reject continuation");
    client.cancel().await?;
    task.await??;
    Ok(())
}

/// Connect raw to `url`, send a close frame at once and report whether the
/// daemon's first frame carries data (a pushed snapshot) rather than ending
/// the connection.
async fn first_frame_is_data(url: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    let (mut raw, _) = tokio_tungstenite::connect_async(url).await?;
    raw.send(Message::Close(None)).await?;
    Ok(matches!(
        raw.next().await,
        Some(Ok(Message::Binary(_) | Message::Text(_)))
    ))
}

// Scenario: Negotiate bounded daemon reads without probing legacy RPC methods
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn negotiate_bounded_daemon_reads_without_probing_legacy_rpc_methods(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // @step Given a daemon supporting the read-only pagination transport
    let workspace = tempfile::tempdir()?;
    std::fs::create_dir_all(workspace.path().join("spec"))?;
    let units: serde_json::Map<String, serde_json::Value> = (1..=5)
        .map(|i| {
            let id = format!("WU-{i}");
            (
                id.clone(),
                json!({"id": id, "title": "t", "type": "task", "status": "backlog"}),
            )
        })
        .collect();
    std::fs::write(
        workspace.path().join("spec/work-units.json"),
        serde_json::to_string(&json!({"workUnits": units}))?,
    )?;
    let watcher = Arc::new(codelet_core::work_units::WorkUnitsWatcher::new(
        workspace.path(),
    )?);
    let service = Arc::new(codelet_rpc::SharedFspecService::new(watcher));
    let (address, stats, _server) =
        codelet_rpc_server::bind_and_serve("127.0.0.1:0", Arc::clone(&service)).await?;
    let url = url::Url::parse(&format!("ws://{address}"))?;
    // @step When the MCP frontend connects
    let (backend, bounded, full) =
        codelet_fspec_tui::WebSocketFspecBackend::connect_mcp_full(url).await?;
    // @step Then the handshake advertises bounded reads and no unsolicited snapshots
    assert!(bounded && full, "read-v2 and the RPC schema are negotiated");
    // A request-only connection pushes nothing: asked to close straight after
    // the handshake, the daemon sends no data frame first.
    assert!(!first_frame_is_data(&format!("ws://{address}/fspec-read-v2")).await?);
    // @step And work-unit pages and details are sliced before RPC serialization
    let before = stats.service.list_work_units_calls();
    let page: serde_json::Value = serde_json::from_str(
        &backend
            .read_work_units_bounded(false, json!({"limit": 2}).to_string())
            .await?
            .map_err(|error| format!("bounded page: {error}"))?,
    )?;
    assert_eq!(page["backend_fetch"], "bounded_rpc");
    assert_eq!(page["returned_count"], 2);
    assert_eq!(page["items"].as_array().map(Vec::len), Some(2));
    let detail: serde_json::Value = serde_json::from_str(
        &backend
            .read_work_units_bounded(true, json!({"id": "WU-3"}).to_string())
            .await?
            .map_err(|error| format!("bounded detail: {error}"))?,
    )?;
    assert_eq!(detail["backend_fetch"], "bounded_rpc");
    assert_eq!(detail["id"], "WU-3");
    assert_eq!(stats.service.list_work_units_calls(), before + 2);
    // @step And a legacy daemon uses the explicitly marked full-list fallback
    let legacy = McpFrontend::new(|_| {
        Box::pin(async { Ok(json!([{"id": "A", "title": "t"}, {"id": "B", "title": "t"}])) })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        legacy.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let result = client
        .call_tool(
            CallToolRequestParams::new("fspec_work_units")
                .with_arguments(serde_json::from_value(json!({"limit": 1}))?),
        )
        .await?;
    let wire = serde_json::to_value(&result)?;
    let text = wire["content"][0]["text"].as_str().ok_or("missing text")?;
    let page: serde_json::Value = serde_json::from_str(text)?;
    assert_eq!(page["backend_fetch"], "full_list");
    client.cancel().await?;
    task.await??;
    Ok(())
}
