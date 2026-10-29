/// Scenario: New shared types live in rpc-types
#[test]
fn new_shared_types_live_in_rpc_types() {
    // @step Given rust/rpc-types/src/lib.rs after RPC-022 lands
    let src = read_raw(&rpc_types_lib());
    // @step Then the file contains the substring "pub struct ProviderInfo"
    assert!(src.contains("pub struct ProviderInfo"));
    // @step And the file contains the substring "pub key: String"
    assert!(src.contains("pub key: String"));
    // @step And the file contains the substring "pub display_name: String"
    assert!(src.contains("pub display_name: String"));
    // @step And the file contains the substring "pub models: Vec<ModelEntry>"
    assert!(src.contains("pub models: Vec<ModelEntry>"));
    // @step And the file contains the substring "pub struct ModelEntry"
    assert!(src.contains("pub struct ModelEntry"));
    // @step And the file contains the substring "pub id: String"
    assert!(src.contains("pub id: String"));
    // @step And the file contains the substring "pub context_window: u32"
    assert!(src.contains("pub context_window: u32"));
    // @step And the file contains the substring "pub supports_reasoning: bool"
    assert!(src.contains("pub supports_reasoning: bool"));
    // @step And the file contains the substring "pub supports_vision: bool"
    assert!(src.contains("pub supports_vision: bool"));
    // @step And the file contains the substring "pub is_custom: bool"
    assert!(src.contains("pub is_custom: bool"));
}

/// Scenario: FspecService trait gains five new RPC methods
#[test]
fn fspec_service_trait_gains_five_new_rpc_methods() {
    // @step Given rust/rpc/src/lib.rs after RPC-022 lands
    let raw = read_raw(&rpc_lib());
    // Whitespace-collapsed view so the multi-line trait declarations
    // (which the rustfmt'd file wraps across 4-5 lines per method)
    // still match the canonical Gherkin signature.
    let collapsed: String = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    // @step Then the file contains the substring "async fn list_providers() -> Vec<ProviderInfo>"
    assert!(collapsed.contains("async fn list_providers() -> Vec<ProviderInfo>"));
    // @step And the file contains the substring "async fn set_session_model(session_id: SessionId, provider_id: String, model_id: String) -> Result<(), String>"
    assert!(collapsed.contains(
        "async fn set_session_model( session_id: SessionId, provider_id: String, model_id: String, ) -> Result<(), String>"
    ) || collapsed.contains(
        "async fn set_session_model(session_id: SessionId, provider_id: String, model_id: String) -> Result<(), String>"
    ));
    // @step And the file contains the substring "async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> Result<(), String>"
    assert!(collapsed.contains(
        "async fn set_thinking_level( session_id: SessionId, level: ThinkingLevel, ) -> Result<(), String>"
    ) || collapsed.contains(
        "async fn set_thinking_level(session_id: SessionId, level: ThinkingLevel) -> Result<(), String>"
    ));
    // @step And the file contains the substring "async fn get_session_role(session_id: SessionId) -> Option<String>"
    assert!(
        collapsed.contains("async fn get_session_role(session_id: SessionId) -> Option<String>")
    );
    // @step And the file contains the substring "async fn set_session_role(session_id: SessionId, role: Option<String>) -> Result<(), String>"
    assert!(collapsed.contains(
        "async fn set_session_role( session_id: SessionId, role: Option<String>, ) -> Result<(), String>"
    ) || collapsed.contains(
        "async fn set_session_role(session_id: SessionId, role: Option<String>) -> Result<(), String>"
    ));
}
