//! Feature: spec/features/compaction-structured-tool-identity.feature
use codelet_cli::session::Session;
use codelet_core::persistence::*;
use codelet_providers::request_admission::AdmissionLimits;
use codelet_providers::selected_request::SelectedRequestPreparer;
use codelet_providers::{ProviderManager, ProviderType};
use rig::{client::CompletionClient, completion::CompletionRequestBuilder, message::Message};
use serde_json::json;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn exercise(explicit: bool) -> TestResult {
    let directory = codelet_test_helpers::setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_owned())?;
    reset_stores_for_tests();
    let mut manifest = create_session_with_provider("tool identity", directory.path(), "openai")?;
    append_message(&mut manifest, "user", "covered original")?;
    let mut manager = ProviderManager::for_testing(ProviderType::OpenAI, Some(200_000), Some(4096));
    manager.set_model_direct("openai", "o3", Some(200_000), Some(4096), None)?;
    let mut session = Session::from_provider_manager(manager);
    let source = session.capture_durable_compaction(manifest.id)?;
    let call_id = if explicit { "call-exact" } else { "item-exact" };
    let mut call =
        json!({"type":"tool_use","id":"item-exact","name":"lookup","input":{"path":"exact"}});
    let mut result =
        json!({"type":"tool_result","tool_use_id":"item-exact","content":"exact result"});
    if explicit {
        call["call_id"] = json!(call_id);
        result["call_id"] = json!(call_id);
    }
    for (role, block) in [("assistant", call), ("user", result)] {
        append_message_with_metadata(
            &mut manifest,
            role,
            "",
            std::collections::HashMap::from([(
                "message".into(),
                json!({"role":role,"content":[block]}),
            )]),
        )?;
    }
    let originals = serde_json::to_value(get_session_messages_full(&manifest)?)?;
    let client: rig::providers::openai::CompletionsClient =
        rig::providers::openai::CompletionsClient::builder()
            .api_key("offline")
            .build()?;
    let model = client.completion_model("o3");
    let template = CompletionRequestBuilder::new(model.clone(), "unused")
        .max_tokens(4096)
        .build();
    let preparer = SelectedRequestPreparer::new(
        model,
        template,
        AdmissionLimits {
            context_capacity: Some(200_000),
            output_reservation: Some(4096),
        },
    );
    let check = |messages: &[Message]| -> Result<(), String> {
        let encoded = serde_json::to_value(messages).map_err(|error| error.to_string())?;
        let encoded = encoded.as_array().ok_or("messages array")?;
        let call = encoded
            .iter()
            .find(|message| message["role"] == "assistant")
            .ok_or("tool call")?;
        let result = encoded
            .iter()
            .find(|message| message["content"][0]["type"] == "toolresult")
            .ok_or("tool result")?;
        if call["content"][0]["id"] != "item-exact"
            || call["content"][0]["call_id"] != call_id
            || result["content"][0]["call_id"] != call_id
        {
            return Err(format!(
                "structured invocation identity was lost: {encoded:?}"
            ));
        }
        Ok(())
    };
    session.commit_durable_compaction(
        source,
        uuid::Uuid::new_v4(),
        "<dag-node depth=\"D1\" turns=\"0-0\" label=\"Work\">exact DAG</dag-node>",
        |messages, generation| {
            check(messages).map_err(
                codelet_providers::request_admission::RequestAdmissionError::Preparation,
            )?;
            preparer.prepare(messages, generation)
        },
        |_| Ok(()),
    )?;
    check(&session.messages)?;
    assert_eq!(
        serde_json::to_value(get_session_messages_full(&load_session(manifest.id)?)?)?,
        originals
    );
    Ok(())
}

/// @step Given a captured source with a structured suffix containing distinct item and call identifiers
/// @step When the runtime admits and installs the durable generation
/// @step Then selected preparation and installed messages retain both tool invocation identifiers
/// @step And immutable suffix envelopes remain unchanged
#[test]
#[serial_test::serial]
fn explicit_tool_call_identifiers_survive_durable_compaction_projection() -> TestResult {
    exercise(true)
}

/// @step Given a captured source with a structured suffix containing legacy tool use identifiers
/// @step When the runtime admits and installs the durable generation
/// @step Then selected preparation and installed messages retain matching tool invocation identifiers
/// @step And immutable suffix envelopes remain unchanged
#[test]
#[serial_test::serial]
fn legacy_tool_use_identifiers_authorize_matching_structured_results() -> TestResult {
    exercise(false)
}
