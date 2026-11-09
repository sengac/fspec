//! Feature: spec/features/board-card-details-additive-api-contract.feature

use codelet_test_helpers::TestResult;
use std::collections::BTreeSet;

use codelet_rpc::mcp_dispatch::{self, InvokeError};
use serde_json::json;

#[path = "board024_additive_api_contract/contracts.rs"]
mod contracts;

#[test]
fn canonical_detail_api_appends_exactly_one_method_without_changing_frozen_prefixes() -> TestResult
{
    // @step Given the frozen original RPC and backend contract snapshots and actual composed sources
    let surfaces = [
        (
            "rpc_declaration",
            "rpc/src/lib.rs",
            false,
            "async fn get_work_unit_details(workspace: String, id: String) -> Result<String, String>;",
        ),
        (
            "rpc_implementation",
            "rpc/src/lib.rs",
            true,
            "async fn get_work_unit_details(self, _ctx: Context, workspace: String, id: String,) -> Result<String, String>;",
        ),
        (
            "backend",
            "fspec-tui/src/transport/mod.rs",
            false,
            "async fn get_work_unit_details(&self, workspace: String, id: String,) -> std::result::Result<String, BackendError>;",
        ),
        (
            "embedded",
            "fspec-tui/src/transport/embedded.rs",
            true,
            "async fn get_work_unit_details(&self, workspace: String, id: String,) -> std::result::Result<String, super::BackendError>;",
        ),
        (
            "websocket",
            "fspec-tui/src/transport/websocket.rs",
            true,
            "async fn get_work_unit_details(&self, workspace: String, id: String,) -> std::result::Result<String, BackendError>;",
        ),
    ];
    let mut failures = Vec::new();
    // @step When the canonical details method is inspected on the RPC and backend surfaces
    for (label, path, implementation, signature) in surfaces {
        let actual = contracts::surface(path, implementation)?;
        let expected = contracts::frozen(label)?;
        // @step Then every frozen header and ordered member is unchanged and the exact expected details method follows them
        if let Err(error) = contracts::assert_append_only(&actual, &expected, signature) {
            failures.push(format!("{label}: {error}"));
        }
        if label == "backend" {
            if let Some(tail) = actual
                .members
                .iter()
                .find(|member| member.name == "get_work_unit_details")
            {
                let body = tail
                    .body
                    .as_deref()
                    .ok_or("details must have an optional backend default")?;
                assert!(body.contains("BackendError :: WorkUnitDetails"));
                assert!(body.contains("unsupported backend"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "additive contract violations:\n{}",
        failures.join("\n")
    );
    contracts::assert_backend_error_variant()?;
    Ok(())
}

#[test]
fn canonical_detail_mcp_registration_matches_the_service_with_strict_read_only_arguments(
) -> TestResult {
    // @step Given the actual composed service declaration and typed MCP registry
    let declaration = contracts::surface("rpc/src/lib.rs", false)?;
    // AMGR-023: pause_resume is removed; its tarpc variant only keeps its
    // frozen index (RPC-449), so the client MCP does not offer it.
    let expected: BTreeSet<_> = declaration
        .members
        .iter()
        .map(|member| member.name.as_str())
        .filter(|name| *name != "pause_resume")
        .collect();
    let registry = mcp_dispatch::methods()?;
    let actual: BTreeSet<_> = registry.iter().map(|method| method.name).collect();
    // @step When method sets flags and canonical detail arguments are checked
    assert_eq!(
        actual, expected,
        "MCP methods must equal the actual service, not a fixed count"
    );
    assert_eq!(
        actual.len(),
        registry.len(),
        "duplicate MCP registrations are forbidden"
    );
    let method = registry
        .iter()
        .find(|method| method.name == "get_work_unit_details")
        .ok_or("canonical detail MCP method is not registered")?;
    assert!(method.read_only);
    assert!(!method.destructive);
    let properties = method.schema["properties"]
        .as_object()
        .ok_or("MCP argument properties missing")?;
    // RPC-450: the details read names its workspace.
    let mut keys = properties.keys().map(String::as_str).collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(keys, ["id", "workspace"]);
    assert_eq!(properties["id"]["type"], "string");
    assert_eq!(properties["workspace"]["type"], "string");
    assert_eq!(method.schema["additionalProperties"], false);
    let mut required: Vec<_> = method.schema["required"]
        .as_array()
        .ok_or("required arguments missing")?
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    required.sort_unstable();
    assert_eq!(required, ["id", "workspace"]);
    mcp_dispatch::validate(
        "get_work_unit_details",
        json!({"workspace": "/work/alpha", "id": "AUTH-001"}),
    )?;
    // @step Then the registry equals the service exactly, apart from the removed methods that only keep their frozen index, and the read-only details method rejects missing invalid and unknown arguments
    for invalid in [
        json!({}),
        json!({"id": "AUTH-001"}),
        json!({"workspace": "/work/alpha", "id": null}),
        json!({"workspace": "/work/alpha", "id": 7}),
        json!({"workspace": "/work/alpha", "id": "AUTH-001", "extra": true}),
        json!([]),
        json!(null),
    ] {
        assert!(
            matches!(
                mcp_dispatch::validate("get_work_unit_details", invalid.clone()),
                Err(InvokeError::Arguments(_))
            ),
            "invalid detail arguments must be rejected as arguments: {invalid}"
        );
    }
    Ok(())
}
