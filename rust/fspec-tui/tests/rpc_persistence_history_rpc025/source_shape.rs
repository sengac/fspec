/// Scenario: FspecService trait declares the three new history RPC methods
#[test]
fn fspec_service_trait_declares_three_new_history_rpc_methods() {
    // @step Given the rust/rpc crate
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("parent")
        .join("rpc")
        .join("src")
        .join("lib.rs");
    let body = codelet_test_helpers::source_composition::read_source(&path)
        .expect("read rust/rpc/src/lib.rs");
    // @step Then the FspecService trait declares "async fn persistence_add_history(session: SessionId, text: String) -> Result<(), String>"
    assert!(
        body.contains("async fn persistence_add_history(session: SessionId, text: String)")
            || body.contains("async fn persistence_add_history(\n        session: SessionId"),
        "FspecService must declare persistence_add_history(session: SessionId, text: String)"
    );
    // @step And the FspecService trait declares "async fn persistence_get_history(session: SessionId, limit: u32) -> Result<Vec<String>, String>"
    assert!(
        body.contains("async fn persistence_get_history(session: SessionId, limit: u32)")
            || body.contains("async fn persistence_get_history(\n        session: SessionId"),
        "FspecService must declare persistence_get_history(session: SessionId, limit: u32)"
    );
    // @step And the FspecService trait declares "async fn persistence_search_history(query: String) -> Result<Vec<HistoryMatch>, String>"
    assert!(
        body.contains("async fn persistence_search_history(query: String)")
            || body.contains("async fn persistence_search_history(\n        query: String"),
        "FspecService must declare persistence_search_history(query: String) -> Vec<HistoryMatch>"
    );
}

/// Scenario: FspecBackend trait declares the three new history methods with matching signatures
#[test]
fn fspec_backend_trait_declares_three_new_history_methods() {
    // @step Given the rust/fspec-tui crate
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("transport")
        .join("mod.rs");
    let body = codelet_test_helpers::source_composition::read_source(&path)
        .expect("read transport/mod.rs");
    // @step Then the FspecBackend trait in transport/mod.rs declares "async fn persistence_add_history(&self, session: SessionId, text: String) -> Result<()>"
    assert!(
        body.contains("async fn persistence_add_history(&self, session: SessionId, text: String)"),
        "FspecBackend must declare persistence_add_history(session, text) -> Result<()>"
    );
    // @step And the FspecBackend trait declares "async fn persistence_get_history(&self, session: SessionId, limit: u32) -> Result<Vec<String>>"
    assert!(
        body.contains("async fn persistence_get_history(&self, session: SessionId, limit: u32)"),
        "FspecBackend must declare persistence_get_history(session, limit) -> Result<Vec<String>>"
    );
    // @step And the FspecBackend trait declares "async fn persistence_search_history(&self, query: String) -> Result<Vec<HistoryMatch>>"
    assert!(
        body.contains("async fn persistence_search_history(&self, query: String)"),
        "FspecBackend must declare persistence_search_history(query) -> Result<Vec<HistoryMatch>>"
    );
}
