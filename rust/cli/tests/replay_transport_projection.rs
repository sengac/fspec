//! Feature: spec/features/structured-generation-aware-session-replay.feature
use codelet_cli::session::replay::ReplayInstallation;
use rig::{
    message::{AssistantContent, Message, ToolCall, ToolFunction},
    OneOrMany,
};
use serde_json::json;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn call(signature: Option<&str>) -> Vec<Message> {
    let mut call = ToolCall::new(
        "call".into(),
        ToolFunction::new("lookup".into(), json!({"path":"src/lib.rs"})),
    );
    call.signature = signature.map(str::to_owned);
    vec![
        Message::user("current turn"),
        Message::Assistant {
            id: None,
            content: OneOrMany::one(AssistantContent::ToolCall(call)),
        },
    ]
}

// Scenario: Gemini transport decoration does not change durable replay identity
#[test]
fn gemini_transport_decoration_does_not_change_durable_replay_identity() -> Result {
    // @step Given committed tool history without a provider thought signature
    let original = call(None);
    let installation = ReplayInstallation::new(1, &[], &original)?;
    // @step When Gemini adds its recognized transport-only placeholder before sending
    let mut runtime = original.clone();
    codelet_core::gemini_history_hook::ensure_thought_signatures(
        &mut runtime,
        "gemini-3-pro-preview",
    );
    assert_ne!(
        runtime, original,
        "fixture must exercise actual transport decoration"
    );
    // @step Then repeated replay retains the same durable history identity
    assert_eq!(installation.locate(&runtime, &original)?, (0, 2));
    assert_eq!(installation.locate(&runtime, &original)?, (0, 2));
    // @step And genuine signature or tool argument changes remain explicit conflicts
    let signed = call(Some("real-signed-a"));
    let signed_installation = ReplayInstallation::new(1, &[], &signed)?;
    assert!(signed_installation
        .locate(&call(Some("real-signed-b")), &signed)
        .is_err());
    let mut changed_arguments = original.clone();
    let Message::Assistant { content, .. } = &mut changed_arguments[1] else {
        return Err("assistant fixture".into());
    };
    let AssistantContent::ToolCall(call) = content.iter_mut().next().ok_or("tool fixture")? else {
        return Err("tool fixture".into());
    };
    call.function.arguments =
        json!({"path":"src/lib.rs","signature":"skip_thought_signature_validator"});
    assert!(installation.locate(&changed_arguments, &original).is_err());
    Ok(())
}
