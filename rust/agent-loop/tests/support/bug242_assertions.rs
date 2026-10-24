use serde_json::Value;

pub fn synthesis(
    bodies: &[Value],
    unknown_calls: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    let synthesis = bodies
        .iter()
        .find(|body| !super::timing::has_tools(body))
        .ok_or("physical synthesis")?;
    assert_eq!(synthesis["model"], "selected-codex-model");
    assert_eq!(synthesis["max_output_tokens"], 4096);
    let items = synthesis["input"].as_array().ok_or("native input")?;
    assert_eq!(
        items
            .iter()
            .filter(|item| item["id"] == "native-reasoning")
            .count(),
        1
    );
    assert!(items
        .iter()
        .any(|item| item["encrypted_content"] == "preserve-native-ciphertext"));
    assert_eq!(
        items
            .iter()
            .filter(|item| item["id"] == "native-commentary")
            .count(),
        1
    );
    for call_id in unknown_calls {
        let outputs: Vec<_> = items
            .iter()
            .filter(|item| item["type"] == "function_call_output" && item["call_id"] == *call_id)
            .collect();
        assert_eq!(outputs.len(), 1);
        let record: Value =
            serde_json::from_str(outputs[0]["output"].as_str().ok_or("handoff output")?)?;
        assert_eq!(record["status"], "outcome_unknown");
        assert_eq!(record["source"], "compaction_handoff");
        assert_eq!(record["tool_result_available"], false);
        assert_eq!(record["execution_may_continue"], true);
        assert_eq!(record["retry_permitted"], false);
        assert!(record.get("success").is_none());
        assert!(record.get("cancelled").is_none());
        assert!(record.get("not_executed").is_none());
        let call = items
            .iter()
            .find(|item| item["type"] == "function_call" && item["call_id"] == *call_id)
            .ok_or("native call")?;
        assert_eq!(call["id"], call_id.replace("-call", "-item"));
    }
    for body in bodies {
        codelet_providers::request_admission::admit_complete_request(
            &serde_json::to_vec(body)?,
            super::codex::LIMITS,
        )?;
    }
    Ok(())
}
