//! Feature: spec/features/client-mcp-full-rpc.feature
#[test]
fn every_rpc_has_a_typed_mcp_mapping() -> Result<(), Box<dyn std::error::Error>> {
    let source = codelet_test_helpers::source_composition::read_source(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )?;
    let parsed = syn::parse_file(&source)?;
    let declaration = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(declaration) if declaration.ident == "FspecService" => {
                Some(declaration)
            }
            _ => None,
        })
        .ok_or("RPC trait missing")?;
    let expected: std::collections::BTreeSet<_> = declaration
        .items
        .iter()
        .filter_map(|item| match item {
            syn::TraitItem::Fn(method) => Some(method.sig.ident.to_string()),
            _ => None,
        })
        .collect();
    let actual: std::collections::BTreeSet<_> = codelet_rpc::mcp_dispatch::methods()
        .iter()
        .map(|method| method.name.to_owned())
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
