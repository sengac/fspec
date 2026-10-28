//! Feature: spec/features/client-mcp-full-rpc.feature
#[test]
fn every_rpc_has_a_typed_mcp_mapping() -> Result<(), Box<dyn std::error::Error>> {
    let source = include_str!("../src/lib.rs");
    let body = source
        .split("pub trait FspecService {")
        .nth(1)
        .ok_or("RPC trait missing")?
        .split("\n}\n")
        .next()
        .ok_or("trait body missing")?;
    let expected: std::collections::BTreeSet<_> = body
        .lines()
        .filter_map(|l| l.trim().strip_prefix("async fn "))
        .filter_map(|s| s.split('(').next())
        .collect();
    let actual: std::collections::BTreeSet<_> = codelet_rpc::mcp_dispatch::methods()
        .iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(actual, expected);
    for method in codelet_rpc::mcp_dispatch::methods() {
        assert_eq!(method.schema["type"], "object");
        assert_eq!(method.schema["additionalProperties"], false);
        assert!(codelet_rpc::mcp_dispatch::validate(
            method.name,
            serde_json::json!({"unexpected":true})
        )
        .is_err());
    }
    Ok(())
}
