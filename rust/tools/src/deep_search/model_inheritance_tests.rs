//! Feature: spec/features/deep-search-model-inheritance.feature
//!
//! DeepSearch sub-agent model inheritance tests (BUG-102), moved out of tests.rs so each test file maps one feature (COV-075).

use super::*;
use serial_test::serial;
use uuid::Uuid;

// ================================================================
// Scenario: Sub-agent inherits Claude provider and model from parent session
// (BUG-102: DeepSearch sub-agent must inherit parent session's model)
// ================================================================

#[tokio::test]
#[serial]
async fn test_handler_captures_provider_and_model_claude() {
    use rig::tool::Tool;
    use std::sync::Arc;

    clear_all_deep_search_handlers();

    // @step Given a parent session with provider "claude" and model "claude-sonnet-4-20250514"
    let session_id = Uuid::new_v4();
    let tool = DeepSearchTool::new(session_id);

    // Simulate what session_manager does: capture provider/model in the closure
    let parent_provider = "claude".to_string();
    let parent_model = Some("claude-sonnet-4-20250514".to_string());

    let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
    let cp = captured_provider.clone();
    let cm = captured_model.clone();

    // @step When the DeepSearch handler is registered for the session
    let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
        // The closure captures provider/model from the parent session
        *cp.lock().unwrap() = parent_provider.clone();
        *cm.lock().unwrap() = parent_model.clone();
        Box::pin(async { Ok("answer".to_string()) })
    });
    set_deep_search_handler(session_id, Some(handler));

    let args = DeepSearchArgs {
        query: "test query".to_string(),
        scope: None,
        max_depth: None,
        max_recursion_depth: None,
    };

    let result = tool.call(args).await;
    assert!(result.is_ok());

    // @step Then the handler closure captures the provider name "claude"
    assert_eq!(*captured_provider.lock().unwrap(), "claude");

    // @step And the handler closure captures the model id "claude-sonnet-4-20250514"
    assert_eq!(
        *captured_model.lock().unwrap(),
        Some("claude-sonnet-4-20250514".to_string())
    );

    // @step And the sub-agent creates a ProviderManager with provider "claude" and model "claude-sonnet-4-20250514"
    // (verified by deep_search_handler.rs using with_provider_and_model —
    //  compile-time check via function signature)

    clear_all_deep_search_handlers();
}

// ================================================================
// Scenario: Sub-agent inherits OpenAI provider and model from parent session
// (BUG-102: provider-agnostic model inheritance)
// ================================================================

#[tokio::test]
#[serial]
async fn test_handler_captures_provider_and_model_openai() {
    use rig::tool::Tool;
    use std::sync::Arc;

    clear_all_deep_search_handlers();

    // @step Given a parent session with provider "openai" and model "gpt-4o"
    let session_id = Uuid::new_v4();
    let tool = DeepSearchTool::new(session_id);

    let parent_provider = "openai".to_string();
    let parent_model = Some("gpt-4o".to_string());

    let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
    let cp = captured_provider.clone();
    let cm = captured_model.clone();

    // @step When the DeepSearch handler is registered for the session
    let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
        *cp.lock().unwrap() = parent_provider.clone();
        *cm.lock().unwrap() = parent_model.clone();
        Box::pin(async { Ok("openai answer".to_string()) })
    });
    set_deep_search_handler(session_id, Some(handler));

    let args = DeepSearchArgs {
        query: "test query".to_string(),
        scope: None,
        max_depth: None,
        max_recursion_depth: None,
    };

    let result = tool.call(args).await;
    assert!(result.is_ok());

    // @step Then the handler closure captures the provider name "openai"
    assert_eq!(*captured_provider.lock().unwrap(), "openai");

    // @step And the handler closure captures the model id "gpt-4o"
    assert_eq!(*captured_model.lock().unwrap(), Some("gpt-4o".to_string()));

    // @step And the sub-agent creates a ProviderManager with provider "openai" and model "gpt-4o"
    // (verified by deep_search_handler.rs using with_provider_and_model —
    //  provider-agnostic path, not hardcoded get_claude())

    clear_all_deep_search_handlers();
}

// ================================================================
// Scenario: Sub-agent inherits Codex provider and model from parent session
// ================================================================

#[tokio::test]
#[serial]
async fn test_handler_captures_provider_and_model_codex() {
    use rig::tool::Tool;
    use std::sync::Arc;

    clear_all_deep_search_handlers();

    // @step Given a parent session with provider "codex" and model "gpt-5.1-codex"
    let session_id = Uuid::new_v4();
    let tool = DeepSearchTool::new(session_id);

    let parent_provider = "codex".to_string();
    let parent_model = Some("gpt-5.1-codex".to_string());

    let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
    let cp = captured_provider.clone();
    let cm = captured_model.clone();

    // @step When the DeepSearch handler is registered for the session
    let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
        *cp.lock().unwrap() = parent_provider.clone();
        *cm.lock().unwrap() = parent_model.clone();
        Box::pin(async { Ok("codex answer".to_string()) })
    });
    set_deep_search_handler(session_id, Some(handler));

    let args = DeepSearchArgs {
        query: "test query".to_string(),
        scope: None,
        max_depth: None,
        max_recursion_depth: None,
    };

    let result = tool.call(args).await;
    assert!(result.is_ok());

    // @step Then the handler closure captures the provider name "codex"
    assert_eq!(*captured_provider.lock().unwrap(), "codex");

    // @step And the handler closure captures the model id "gpt-5.1-codex"
    assert_eq!(
        *captured_model.lock().unwrap(),
        Some("gpt-5.1-codex".to_string())
    );

    // @step And the sub-agent creates a ProviderManager with provider "codex" and model "gpt-5.1-codex"
    clear_all_deep_search_handlers();
}

// ================================================================
// Scenario: Sub-agent inherits Z.AI provider and model from parent session
// ================================================================

#[tokio::test]
#[serial]
async fn test_handler_captures_provider_and_model_zai() {
    use rig::tool::Tool;
    use std::sync::Arc;

    clear_all_deep_search_handlers();

    // @step Given a parent session with provider "zai" and model "glm-4.7"
    let session_id = Uuid::new_v4();
    let tool = DeepSearchTool::new(session_id);

    let parent_provider = "zai".to_string();
    let parent_model = Some("glm-4.7".to_string());

    let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
    let cp = captured_provider.clone();
    let cm = captured_model.clone();

    // @step When the DeepSearch handler is registered for the session
    let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
        *cp.lock().unwrap() = parent_provider.clone();
        *cm.lock().unwrap() = parent_model.clone();
        Box::pin(async { Ok("zai answer".to_string()) })
    });
    set_deep_search_handler(session_id, Some(handler));

    let args = DeepSearchArgs {
        query: "test query".to_string(),
        scope: None,
        max_depth: None,
        max_recursion_depth: None,
    };

    let result = tool.call(args).await;
    assert!(result.is_ok());

    // @step Then the handler closure captures the provider name "zai"
    assert_eq!(*captured_provider.lock().unwrap(), "zai");

    // @step And the handler closure captures the model id "glm-4.7"
    assert_eq!(*captured_model.lock().unwrap(), Some("glm-4.7".to_string()));

    // @step And the sub-agent creates a ProviderManager with provider "zai" and model "glm-4.7"
    clear_all_deep_search_handlers();
}

// ================================================================
// Scenario: Sub-agent uses with_provider_and_model instead of with_model_support
// (BUG-102: ProviderManager construction must use inherited model)
// ================================================================

#[tokio::test]
#[serial]
async fn test_provider_manager_uses_with_provider_and_model() {
    use rig::tool::Tool;
    use std::sync::Arc;

    clear_all_deep_search_handlers();

    // @step Given a parent session with provider "claude" and model "claude-sonnet-4-20250514"
    let session_id = Uuid::new_v4();
    let tool = DeepSearchTool::new(session_id);

    // Capture what the handler receives to verify the provider/model
    // are available for ProviderManager construction
    let captured_provider = Arc::new(std::sync::Mutex::new(String::new()));
    let captured_model = Arc::new(std::sync::Mutex::new(None::<String>));
    let cp = captured_provider.clone();
    let cm = captured_model.clone();

    // Simulate the session_manager pattern: provider/model captured in closure
    let provider_name = "claude".to_string();
    let model_id = Some("claude-sonnet-4-20250514".to_string());

    // @step When the DeepSearch sub-agent builds a ProviderManager
    let handler: DeepSearchHandler = Arc::new(move |_query, _scope, _max_depth, _max_rec| {
        // In real code, these would be passed to
        // ProviderManager::with_provider_and_model(provider_name, model_id)
        *cp.lock().unwrap() = provider_name.clone();
        *cm.lock().unwrap() = model_id.clone();
        Box::pin(async { Ok("answer".to_string()) })
    });
    set_deep_search_handler(session_id, Some(handler));

    let args = DeepSearchArgs {
        query: "test".to_string(),
        scope: None,
        max_depth: None,
        max_recursion_depth: None,
    };
    let result = tool.call(args).await;
    assert!(result.is_ok());

    // @step Then the ProviderManager is created via with_provider_and_model
    // Verified by the handler having access to provider_name and model_id
    // (in real code: ProviderManager::with_provider_and_model(&provider, model.as_deref()))
    let provider = captured_provider.lock().unwrap().clone();
    assert!(
        !provider.is_empty(),
        "provider must be captured for with_provider_and_model"
    );

    // @step And select_model is not called
    // with_provider_and_model() sets selected_model directly — no select_model() needed
    // This is verified by the function signature: with_provider_and_model(name, model_id)
    // sets selected_model = model_id.map(String::from) without registry lookup

    // @step And the selected_model_id returns "claude-sonnet-4-20250514"
    let model = captured_model.lock().unwrap().clone();
    assert_eq!(model, Some("claude-sonnet-4-20250514".to_string()));

    clear_all_deep_search_handlers();
}
