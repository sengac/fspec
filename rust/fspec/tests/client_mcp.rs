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

#[tokio::test]
async fn discover_and_call_read_only_tools() -> Result<(), Box<dyn std::error::Error + Send + Sync>>
{
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
    let tools = client.list_all_tools().await?;
    assert_eq!(tools.len(), 3);
    assert!(tools
        .iter()
        .all(|t| t.annotations.as_ref().and_then(|a| a.read_only_hint) == Some(true)));
    let result = client
        .call_tool(CallToolRequestParams::new("fspec_work_units"))
        .await?;
    assert_eq!(result.is_error, Some(false));
    assert!(serde_json::to_string(&result)?.contains("TEST-1"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    client.cancel().await?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn reject_mutations_and_unexpected_arguments(
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    assert!(client
        .call_tool(CallToolRequestParams::new("send_input"))
        .await
        .is_err());
    let args = serde_json::from_value(json!({"command": "delete"}))?;
    assert!(client
        .call_tool(CallToolRequestParams::new("fspec_health").with_arguments(args))
        .await
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    client.cancel().await?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn contain_backend_failures() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        let result = client
            .call_tool(CallToolRequestParams::new("fspec_health"))
            .await?;
        assert_eq!(result.is_error, Some(true));
        assert!(serde_json::to_string(&result)?.len() < 1000);
        client.cancel().await?;
        task.await??;
    }
    Ok(())
}

#[test]
fn reject_unprotected_remote_daemon_urls() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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

#[tokio::test(start_paused = true)]
async fn bound_stalled_daemon_calls() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = McpFrontend::new(|_| Box::pin(std::future::pending()));
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let start = tokio::time::Instant::now();
    let result = client
        .call_tool(CallToolRequestParams::new("fspec_health"))
        .await?;
    assert_eq!(result.is_error, Some(true));
    assert!(start.elapsed() >= std::time::Duration::from_secs(10));
    client.cancel().await?;
    task.await??;
    Ok(())
}

#[tokio::test]
async fn page_oversized_workspace() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
    assert_eq!(ids.len(), 75);
    assert_eq!(ids[0], "WU-000");
    assert_eq!(ids[74], "WU-074");
    assert_eq!(cursor, Some(serde_json::Value::Null));
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
