//! Feature: spec/features/mcp-rpc-catalog.feature
#[test]
fn every_rpc_has_a_typed_mcp_mapping() -> Result<(), Box<dyn std::error::Error>> {
    // @step Given the FspecService trait declaration
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
    // @step When the MCP method catalog is built
    let methods = codelet_rpc::mcp_dispatch::methods()?;
    // @step Then it lists exactly the declared methods, apart from the removed methods that only keep their frozen index
    let actual: std::collections::BTreeSet<_> = methods
        .iter()
        .map(|method| method.name.to_owned())
        .collect();
    // AMGR-023: pause_resume is removed; its tarpc variant only keeps its
    // frozen index (RPC-449), so the client MCP does not offer it.
    let removed = std::collections::BTreeSet::from(["pause_resume".to_string()]);
    assert_eq!(actual, &expected - &removed);
    // @step And every method's schema is a closed JSON object that rejects unknown arguments
    for method in methods {
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

#[test]
fn a_schema_that_fails_to_serialize_is_reported_with_its_method() -> Result<(), String> {
    // @step Given a method whose argument schema fails to serialize
    let failure = serde_json::from_str::<serde_json::Value>("{")
        .err()
        .ok_or("a truncated document must fail to parse")?;
    // @step When the MCP catalog attaches the method name to the failure
    let result = codelet_rpc::mcp_dispatch::method_schema("list_work_units", Err(failure));
    // @step Then the catalog error names the method and the serialization failure
    let error = result.err().ok_or("a failed schema must be an error")?;
    assert_eq!(error.method, "list_work_units");
    let message = error.to_string();
    assert!(
        message.starts_with("schema for RPC method list_work_units failed to serialize: "),
        "{message}"
    );
    Ok(())
}
