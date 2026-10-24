//! Feature: spec/features/durable-structured-tool-identity.feature

#[path = "support/compaction_boundary.rs"]
mod support;

use codelet_core::persistence::*;
use codelet_test_helpers::setup_temp_directory;
use serde_json::{json, Value};

#[test]
#[serial_test::serial]
fn committed_replay_preserves_distinct_tool_item_and_invocation_identities(
) -> Result<(), Box<dyn std::error::Error>> {
    // @step Given a committed generation with blob-backed tool call and result suffixes using distinct item and invocation identities
    let directory = setup_temp_directory()?;
    codelet_common::set_data_directory(directory.path().to_path_buf())?;
    reset_stores_for_tests();
    let mut session = create_session_with_provider("tool identity", directory.path(), "openai")?;
    append_message(&mut session, "user", "covered original")?;
    let candidate = support::candidate(&session)?;
    let input = json!({"path":"original.txt","body":"large input".repeat(4096)});
    let output = "large output".repeat(4096);
    let input_hash = store_blob(serde_json::to_string(&input)?.as_bytes())?;
    let output_hash = store_blob(output.as_bytes())?;
    let suffixes = [
        (
            "assistant",
            json!({"type":"tool_use","id":"item-exact","call_id":"call-exact",
            "name":"read","input":{"_blob_ref":format!("blob:sha256:{input_hash}")},
            "provider_metadata":{"opaque":"call"}}),
        ),
        (
            "user",
            json!({"type":"tool_result","tool_use_id":"item-exact","call_id":"call-exact",
            "content":format!("blob:sha256:{output_hash}"),
            "provider_metadata":{"opaque":"result"}}),
        ),
    ];
    for (role, content) in suffixes {
        append_message_with_metadata(
            &mut session,
            role,
            "structured suffix",
            [("message".into(), json!({"role":role,"content":[content]}))]
                .into_iter()
                .collect(),
        )?;
    }
    let original = get_session_messages_full(&session)?;
    let committed = compaction_commit::commit_compaction(candidate, "model-a", |_, _| Ok(()))?;
    compaction_commit::publish_compaction(&committed, |_| Ok(()))?;
    // @step When a cold reader rehydrates the committed structured replay
    reset_stores_for_tests();
    let replay = get_session_message_envelopes(session.id)?;
    let call: Value = serde_json::from_str(&replay[1])?;
    let result: Value = serde_json::from_str(&replay[2])?;
    // @step Then both suffixes retain their explicit invocation identity and original provider metadata
    for envelope in [&call, &result] {
        assert_eq!(envelope["message"]["content"][0]["call_id"], "call-exact");
    }
    assert_eq!(call["message"]["content"][0]["id"], "item-exact");
    assert_eq!(result["message"]["content"][0]["tool_use_id"], "item-exact");
    assert_eq!(
        call["message"]["content"][0]["provider_metadata"]["opaque"],
        "call"
    );
    assert_eq!(
        result["message"]["content"][0]["provider_metadata"]["opaque"],
        "result"
    );
    // @step And blob content is restored without modifying immutable originals
    assert_eq!(call["message"]["content"][0]["input"], input);
    assert_eq!(result["message"]["content"][0]["content"], output);
    let retained = get_session_messages_full(&load_session(session.id)?)?;
    assert_eq!(
        serde_json::to_value(retained)?,
        serde_json::to_value(original)?
    );
    Ok(())
}
