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

#[test]
fn gemini_transport_placeholder_does_not_rewrite_replay_identity() -> Result {
    let original = call(None);
    let installation = ReplayInstallation::new(1, &[], &original)?;
    let mut runtime = original.clone();
    codelet_core::gemini_history_hook::ensure_thought_signatures(
        &mut runtime,
        "gemini-3-pro-preview",
    );
    assert_ne!(
        runtime, original,
        "fixture must exercise actual transport decoration"
    );
    assert_eq!(installation.locate(&runtime, &original)?, (0, 2));
    Ok(())
}

#[test]
fn changed_genuine_provider_signature_remains_a_conflict() -> Result {
    let original = call(Some("real-signed-a"));
    let installation = ReplayInstallation::new(1, &[], &original)?;
    assert!(installation
        .locate(&call(Some("real-signed-b")), &original)
        .is_err());
    Ok(())
}

#[test]
fn tool_input_changes_are_not_transport_decoration() -> Result {
    let original = call(None);
    let installation = ReplayInstallation::new(1, &[], &original)?;
    let mut runtime = original.clone();
    let Message::Assistant { content, .. } = &mut runtime[1] else {
        return Err("assistant fixture".into());
    };
    let AssistantContent::ToolCall(call) = content.iter_mut().next().ok_or("tool fixture")? else {
        return Err("tool fixture".into());
    };
    call.function.arguments =
        json!({"path":"src/lib.rs","signature":"skip_thought_signature_validator"});
    assert!(installation.locate(&runtime, &original).is_err());
    Ok(())
}
