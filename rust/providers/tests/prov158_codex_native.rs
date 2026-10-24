use serde_json::json;

#[path = "../src/codex/memory_policy.rs"]
mod memory_policy;
#[path = "../src/codex/native.rs"]
pub mod native;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn negotiated_hosted_search_presents_attributable_status_and_sources() -> TestResult {
    // @step Given a Codex session explicitly negotiating hosted search with replay support
    // @step When hosted search progress completion and source citations arrive
    // @step Then visible output attributes status and sources to the hosted item
    // @step And no hosted call is dispatched as a local tool
    // @step And its typed output survives continuation
    let mut request = json!({"input": []});
    native::NativeCapabilities {
        hosted_search: true,
    }
    .apply(&mut request);
    let mut output = native::NativeOutput::from_request(&request);
    let progress = output
        .event(
            "response.web_search_call.searching",
            &json!({
                "item_id":"ws_1"
            }),
        )?
        .ok_or("visible hosted progress missing")?;
    assert!(progress.contains("ws_1") && progress.contains("searching"));
    let item = json!({"type":"web_search_call","id":"ws_1","status":"completed",
        "action":{"type":"search","query":"Rust"},"encrypted_content":"secret"});
    let notices = output.terminal(&[
        item.clone(),
        json!({"type":"message","id":"msg_1",
        "role":"assistant","content":[{"type":"output_text","text":"Result",
        "annotations":[{"type":"url_citation","url":"https://rust-lang.org",
        "title":"Rust","start_index":0,"end_index":6}]}]}),
    ])?;
    let visible = notices.join("\n");
    assert!(visible.contains("ws_1") && visible.contains("completed"));
    assert!(visible.contains("msg_1") && visible.contains("https://rust-lang.org"));
    assert!(!visible.contains("secret"));
    assert!(output.terminal(&[item])?.is_empty());
    Ok(())
}

#[test]
fn native_compaction_is_unsupported_even_with_a_trigger() -> TestResult {
    let mut output = native::NativeOutput::from_request(&json!({
        "input":[{"type":"compaction_trigger"}]
    }));
    assert!(memory_policy::validate_request(&json!({"context_management":[]})).is_err());
    for family in ["compaction", "compaction_summary", "context_compaction"] {
        let error = output
            .terminal(&[json!({"type":family,"id":"cmp_1",
            "encrypted_content":"opaque+/=秘密"})])
            .err()
            .ok_or("native memory accepted")?;
        assert!(
            matches!(error, rig::completion::CompletionError::RequestError(ref source)
            if source.downcast_ref::<memory_policy::NativeMemoryUnsupported>().is_some())
        );
        assert!(!error.to_string().contains("opaque"));
    }
    Ok(())
}

#[test]
fn unsupported_and_unnegotiated_native_items_fail_closed() -> TestResult {
    for family in [
        "tool_search_call",
        "tool_search_output",
        "image_generation_call",
        "local_shell_call",
        "custom_tool_call",
        "custom_tool_call_output",
        "computer_call",
        "computer_call_output",
        "file_search_call",
        "code_interpreter_call",
        "configuration_update",
        "compaction_trigger",
        "additional_tools",
        "future_executor",
        "web_search_call",
        "compaction",
        "context_compaction",
    ] {
        let mut output = native::NativeOutput::from_request(&json!({}));
        let error = output
            .terminal(&[json!({"type":family,"encrypted_content":"never-display"})])
            .err()
            .ok_or("native family must fail closed")?
            .to_string();
        assert!(error.contains(family), "{error}");
        assert!(
            error.contains("disable") || error.contains("Disable"),
            "{error}"
        );
        assert!(!error.contains("never-display"));
    }
    Ok(())
}

#[test]
fn optional_metadata_does_not_invalidate_usable_text() -> TestResult {
    let mut output = native::NativeOutput::from_request(&json!({}));
    assert!(output
        .terminal(&[json!({"type":"message","role":"assistant",
        "content":[{"type":"output_text","text":"usable"}],
        "unknown_optional":{"future":"metadata"}})])?
        .is_empty());
    assert!(output
        .event("response.future_metadata", &json!({"opaque":"ignored"}))?
        .is_none());
    Ok(())
}
