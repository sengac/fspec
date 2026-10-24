//! Feature: spec/features/codex-custom-tool-dialect.feature
use codelet_providers::codex::custom_tools::{CustomToolRegistry, ToolRegistration};
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn definitions() -> Value {
    json!([
        {"type":"function","name":"apply_patch","description":"Patch files","parameters":{"type":"object","properties":{"patch":{"type":"string"}}}},
        {"type":"function","name":"read_file","description":"Read files","parameters":{"type":"object"}},
        {"type":"function","name":"list_dir","description":"List files","parameters":{"type":"object"}}
    ])
}

fn patch_registration() -> ToolRegistration {
    ToolRegistration {
        namespace: Some("edit".into()),
        name: "apply_patch".into(),
        executor: "apply_patch".into(),
        grammar: Some("start: /[\\s\\S]+/".into()),
        raw_input: true,
    }
}

#[test]
fn registered_raw_patch_input_retains_its_grammar_and_text() -> TestResult {
    // @step Given an enabled namespaced raw patch tool with a grammar
    let registry = CustomToolRegistry::new(vec![patch_registration()])?;
    let mut tools = definitions();
    let patch = "*** Begin Patch\n*** Add File: café.txt\n+你好\n*** End Patch";
    // @step When Codex registers the tool and completes a Unicode patch call
    registry.register(&mut tools)?;
    let call = registry.normalize_call(&json!({"type":"custom_tool_call","id":"item1","call_id":"call1","namespace":"edit","name":"apply_patch","input":patch,"status":"completed"}))?;
    // @step Then the wire definition retains its grammar and the existing executor receives the exact patch
    let namespace = tools
        .as_array()
        .ok_or("missing tools")?
        .iter()
        .find(|tool| tool["type"] == "namespace")
        .ok_or("missing namespace")?;
    assert_eq!(
        namespace["tools"][0]["format"],
        json!({"type":"grammar","syntax":"lark","definition":"start: /[\\s\\S]+/"})
    );
    assert_eq!(call["name"], "apply_patch");
    assert_eq!(
        serde_json::from_str::<Value>(call["arguments"].as_str().ok_or("missing arguments")?)?,
        json!({"patch":patch})
    );
    assert_eq!(call["call_id"], "call1");
    Ok(())
}

#[test]
fn namespace_identity_survives_independent_local_tool_names() -> TestResult {
    // @step Given two namespaces exposing the same local function name
    let registry = CustomToolRegistry::new(vec![
        ToolRegistration {
            namespace: Some("files".into()),
            name: "inspect".into(),
            executor: "read_file".into(),
            grammar: None,
            raw_input: false,
        },
        ToolRegistration {
            namespace: Some("directories".into()),
            name: "inspect".into(),
            executor: "list_dir".into(),
            grammar: None,
            raw_input: false,
        },
    ])?;
    let mut tools = definitions();
    registry.register(&mut tools)?;
    // @step When completed calls arrive for both namespaces
    for (namespace, executor) in [("files", "read_file"), ("directories", "list_dir")] {
        let call = registry.normalize_call(&json!({"type":"function_call","id":namespace,"call_id":format!("call_{namespace}"),"namespace":namespace,"name":"inspect","arguments":"{}"}))?;
        // @step Then each call selects its registered executor and keeps its result identity
        assert_eq!(call["name"], executor);
        assert_eq!(call["call_id"], format!("call_{namespace}"));
    }
    Ok(())
}

#[test]
fn unregistered_or_malformed_dialects_cannot_execute() -> TestResult {
    // @step Given disabled custom tools or an invalid namespace registration
    let disabled = CustomToolRegistry::default();
    let mut invalid = patch_registration();
    invalid.namespace = Some("bad.namespace".into());
    // @step When a custom call or invalid configuration is supplied
    let rejected = disabled.normalize_call(&json!({"type":"custom_tool_call","id":"item","call_id":"call","name":"apply_patch","input":"raw"}));
    // @step Then an actionable error occurs before local tool execution
    assert!(rejected.is_err());
    assert!(CustomToolRegistry::new(vec![invalid]).is_err());
    let registry = CustomToolRegistry::new(vec![patch_registration()])?;
    assert!(registry.register(&mut json!([])).is_err());
    let mut forbidden = patch_registration();
    forbidden.executor = "shell".into();
    assert!(CustomToolRegistry::new(vec![forbidden]).is_err());
    Ok(())
}

#[test]
fn custom_results_use_the_matching_protocol_result_type() -> TestResult {
    // @step Given a completed custom call and its local execution result
    let registry = CustomToolRegistry::new(vec![patch_registration()])?;
    let original = json!({"type":"custom_tool_call","id":"item1","call_id":"call1","namespace":"edit","name":"apply_patch","input":"raw patch"});
    let normalized = registry.normalize_call(&original)?;
    let mut input = json!([normalized, {"type":"function_call_output","call_id":"call1","output":"applied"}, {"type":"function_call_output","call_id":"ordinary","output":"text"}]);
    // @step When the next Codex request is serialized
    registry.restore_history(&mut input, std::slice::from_ref(&original))?;
    // @step Then the raw call and custom result retain the original call identifier
    assert_eq!(input[0], original);
    assert_eq!(
        input[1],
        json!({"type":"custom_tool_call_output","call_id":"call1","output":"applied"})
    );
    assert_eq!(input[2]["type"], "function_call_output");
    Ok(())
}

#[test]
fn disabled_dialect_preserves_ordinary_tool_registration() -> TestResult {
    // @step Given an ordinary Codex or non-Codex function tool configuration
    let registry = CustomToolRegistry::default();
    let mut tools = definitions();
    let original = tools.clone();
    let call = json!({"type":"function_call","id":"item","call_id":"call","name":"read_file","arguments":"{\"path\":\"file\"}"});
    // @step When its request is serialized without dialect opt-in
    registry.register(&mut tools)?;
    let normalized = registry.normalize_call(&call)?;
    // @step Then the ordinary function definition and arguments remain unchanged
    assert_eq!(tools, original);
    assert_eq!(normalized, call);
    Ok(())
}
