//! Feature: spec/features/session-replay-invocation-identity.feature

#[path = "cmpct060/support.rs"]
pub mod support;

use codelet_core::SessionManagerHandle;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn distinct_invocation_identities_survive_session_replay_onto_the_next_request(
) -> support::TestResult {
    // @step Given structured tool call envelopes with distinct item and invocation identities
    let (_directory, manager, session_id, baseline) = support::manager().await?;
    manager
        .get_session(&session_id.value)?
        .inner
        .lock()
        .await
        .provider_manager_mut()
        .set_model_direct("openai", "test", Some(200_000), Some(4_000), None)?;
    let envelopes = vec![
        json!({"message":{"role":"assistant","content":[{
            "type":"tool_use","id":"fc_saved","call_id":"call_saved",
            "name":"lookup","input":{"query":"original"}
        }]}})
        .to_string(),
        support::envelope(json!([{
            "type":"tool_result","tool_use_id":"fc_saved","call_id":"call_saved",
            "content":"saved result"
        }]))
        .to_string(),
    ];
    // @step When the session restores the structured envelopes and sends its next request
    manager.restore_session_messages(&session_id, envelopes)?;
    let wire = support::next_wire(&manager, &session_id, baseline).await?;
    let input = wire["input"].as_array().ok_or("Responses input")?;
    let call = input
        .iter()
        .find(|item| item["type"] == "function_call")
        .ok_or("physical function call")?;
    let result = input
        .iter()
        .find(|item| item["type"] == "function_call_output")
        .ok_or("physical function result")?;
    // @step Then the physical request preserves the item identity and pairs the original invocation
    assert_eq!(call["id"], "fc_saved");
    assert_eq!(call["call_id"], "call_saved");
    assert_eq!(result["call_id"], "call_saved");
    assert_eq!(result["output"], "saved result");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[serial_test::serial]
async fn invalid_invocation_identity_fails_without_installing_partial_replay() -> support::TestResult
{
    // @step Given a valid replay prefix followed by a tool call with invalid invocation identity
    let (_directory, manager, session_id, _baseline) = support::manager().await?;
    let background = manager.get_session(&session_id.value)?;
    let before = background.inner.lock().await.messages.clone();
    let mut events = background.subscribe_to_stream();
    for invalid in [json!(null), json!(42), json!("")] {
        let envelopes = vec![
            support::envelope(json!("must not install prefix")).to_string(),
            json!({"message":{"role":"assistant","content":[{
                "type":"tool_use","id":"fc_saved","call_id":invalid,
                "name":"lookup","input":{}
            }]}})
            .to_string(),
        ];
        // @step When the session restores the structured envelopes
        let result = manager.restore_session_messages(&session_id, envelopes);
        // @step Then restoration fails without changing history or publishing replay output
        assert!(result.is_err(), "invalid invocation identity was accepted");
        assert_eq!(background.inner.lock().await.messages, before);
        assert!(events.try_recv().is_err());
    }
    Ok(())
}
