//! Feature: spec/features/client-mcp-full-rpc.feature
#[allow(dead_code)]
#[path = "../src/client_mcp_full.rs"]
mod client_mcp_full;
#[allow(dead_code)]
#[path = "../src/client_mcp_results.rs"]
mod client_mcp_results;
#[allow(dead_code)]
#[path = "../src/client_mcp_service.rs"]
mod client_mcp_service;
#[allow(dead_code)]
#[path = "../../sessions/src/interaction_slot.rs"]
mod interaction_slot;
use rmcp::{model::*, ServiceExt};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
fn read() -> client_mcp_service::McpFrontend {
    client_mcp_service::McpFrontend::new(|_| Box::pin(async { Ok(json!({})) }))
}
#[tokio::test]
async fn validated_mutation_is_forwarded_once_without_form_support() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let server = client_mcp_full::FullFrontend::new(read(), true, move |name, args| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            assert_eq!(name, "create_session");
            assert_eq!(args, json!({"role":null}));
            Ok(json!({"value":"fixture-session"}))
        })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    assert!(client
        .call_tool(
            CallToolRequestParams::new("fspec_rpc_create_session")
                .with_arguments(serde_json::from_value(json!({"role":42}))?)
        )
        .await
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let result = client
        .call_tool(
            CallToolRequestParams::new("fspec_rpc_create_session")
                .with_arguments(serde_json::from_value(json!({"role":null}))?),
        )
        .await?;
    assert_eq!(result.is_error, Some(false));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    client.cancel().await?;
    task.await??;
    Ok(())
}
#[tokio::test]
async fn checked_approval_relay_preserves_daemon_rejection() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let server = client_mcp_full::FullFrontend::new(read(), true, move |name, args| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            if name == "get_interaction_snapshot" {
                return Ok(json!(
                    json!({"pause":{"request_id":"fixture-nonce","state":{"message":"fixture"}}})
                        .to_string()
                ));
            }
            assert_eq!(name, "reply_interaction_checked");
            assert_eq!(args["request_id"], "fixture-nonce");
            assert_eq!(args["method"], "pause_confirm");
            Err(codelet_rpc::mcp_dispatch::InvokeError::Daemon(
                "Stale interaction".into(),
            ))
        })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let result = client
        .call_tool(
            CallToolRequestParams::new("fspec_rpc_pause_confirm").with_arguments(
                serde_json::from_value(json!({"session_id":{"value":"fixture"},"accept":false}))?,
            ),
        )
        .await?;
    assert_eq!(result.is_error, Some(true));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    client.cancel().await?;
    task.await??;
    Ok(())
}
#[tokio::test]
async fn discover_all_and_continue_without_reexecution() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let server = client_mcp_full::FullFrontend::new(read(), true, move |_, _| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(json!("🦀".repeat(6000))) })
    });
    let (a, b) = tokio::io::duplex(65536);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let tools = client.list_all_tools().await?;
    assert_eq!(tools.len(), 125);
    assert!(client
        .call_tool(
            CallToolRequestParams::new("fspec_rpc_health")
                .with_arguments(serde_json::from_value(json!({"unexpected":true}))?)
        )
        .await
        .is_err());
    let first = client
        .call_tool(CallToolRequestParams::new("fspec_rpc_health"))
        .await?;
    let value: Value = serde_json::from_str(&first.content[0].as_text().ok_or("text")?.text)?;
    assert_eq!(value["has_more"], true);
    let next = client
        .call_tool(
            CallToolRequestParams::new("fspec_rpc_result").with_arguments(serde_json::from_value(
                json!({"result_id":value["result_id"],"offset":value["next_offset"]}),
            )?),
        )
        .await?;
    assert_eq!(next.is_error, Some(false));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    client.cancel().await?;
    task.await??;
    Ok(())
}
#[tokio::test]
async fn incompatible_daemon_keeps_legacy_reads_only() -> TestResult {
    let server = client_mcp_full::FullFrontend::new(read(), false, |_, _| {
        Box::pin(async { Err(codelet_rpc::mcp_dispatch::InvokeError::Unknown) })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    assert_eq!(client.list_all_tools().await?.len(), 3);
    assert!(client
        .call_tool(CallToolRequestParams::new("fspec_rpc_health"))
        .await
        .is_err());
    client.cancel().await?;
    task.await??;
    Ok(())
}
#[tokio::test]
async fn transport_error_is_uncertain_and_not_retried() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let server = client_mcp_full::FullFrontend::new(read(), true, move |_, _| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(async {
            Err(codelet_rpc::mcp_dispatch::InvokeError::Transport(
                "fixture connection loss".into(),
            ))
        })
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let result = client
        .call_tool(CallToolRequestParams::new("fspec_rpc_health"))
        .await?;
    let value: Value = serde_json::from_str(&result.content[0].as_text().ok_or("text")?.text)?;
    assert_eq!(value["execution"], "unknown");
    assert_eq!(value["safe_to_retry"], false);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    client.cancel().await?;
    task.await??;
    Ok(())
}
#[tokio::test]
async fn deadline_reports_unknown_without_retry() -> TestResult {
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let server = client_mcp_full::FullFrontend::new(read(), true, move |_, _| {
        counter.fetch_add(1, Ordering::SeqCst);
        Box::pin(std::future::pending())
    });
    let (a, b) = tokio::io::duplex(8192);
    let task = tokio::spawn(async move {
        server.serve(a).await?.waiting().await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });
    let client = ClientInfo::default().serve(b).await?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(35),
        client.call_tool(CallToolRequestParams::new("fspec_rpc_health")),
    )
    .await??;
    let value: Value = serde_json::from_str(&result.content[0].as_text().ok_or("text")?.text)?;
    assert_eq!(value["execution"], "unknown");
    assert_eq!(value["safe_to_retry"], false);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    client.cancel().await?;
    task.await??;
    Ok(())
}
#[test]
fn result_capacity_preserves_existing_continuations() -> TestResult {
    let mut results = client_mcp_results::Results::default();
    let first = results.insert(&json!("x".repeat(6000)))?;
    for _ in 1..8 {
        results.insert(&json!("x".repeat(6000)))?;
    }
    assert!(results.insert(&json!("x".repeat(6000))).is_err());
    assert!(results
        .page(first["result_id"].as_str().ok_or("id")?, 4096)
        .is_ok());
    Ok(())
}
#[test]
fn sensitive_categories_retain_state_change_annotations() {
    for name in [
        "send_input",
        "delete_all_checkpoints",
        "set_provider_credentials",
        "oauth_browser_login",
        "schedule_add",
        "loop_add",
        "write_exec_stdin",
        "pause_confirm",
        "send_hitl_response",
        "reply_interaction_checked",
    ] {
        let method = codelet_rpc::mcp_dispatch::methods()
            .into_iter()
            .find(|m| m.name == name)
            .expect("catalogued RPC");
        assert!(!method.read_only, "{name}");
        assert!(
            method.destructive,
            "conservative destructive hint for {name}"
        );
    }
}
