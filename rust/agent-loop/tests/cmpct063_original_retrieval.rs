//! Feature: spec/features/lossless-original-envelope-retrieval.feature

#[path = "fixtures/cmpct063.rs"]
mod fixture;

use codelet_core::persistence::{
    append_message, create_session, load_session, make_blob_reference, save_session, store_blob,
    MessageSource,
};
use fixture::{assert_error, blob_envelope, envelope, Fixture};
use proptest::prelude::*;
use rig::tool::Tool;
use serde_json::json;
use serial_test::serial;
use uuid::Uuid;

#[test]
#[serial]
fn scenario_retrieve_exact_structured_evidence_beyond_previews() {
    // @step Given a persisted tool result with large UTF-8 content and a sentinel beyond its preview
    let mut fixture = Fixture::new();
    let original = envelope(&format!("{}SENTINEL", "証拠🦀".repeat(1500)));
    // @step And original envelope metadata includes provider-specific nested fields
    let stored_id = fixture.append(original.clone());
    // @step When original retrieval is requested while compaction trimming is enabled
    let result = fixture.request(json!({}));
    // @step Then the exact structured envelope and sentinel are returned without truncation
    assert_eq!(result["result_type"], "originals");
    assert_eq!(result["source"], "original");
    assert_eq!(result["truncated"], false);
    assert_eq!(result["records"][0]["envelope"], original);
    // @step And the persisted identity is separate from the unchanged envelope identity
    assert_eq!(result["records"][0]["message_id"], stored_id.to_string());
    assert_ne!(
        result["records"][0]["envelope"]["uuid"],
        stored_id.to_string()
    );
    // @step And ordinary show still returns its existing preview
    let preview = fixture.call(json!({"action_type":"show", "session_id":fixture.session.id}));
    assert_eq!(preview["messages"][0]["content"], "searchable preview");
    assert!(!preview.to_string().contains("SENTINEL"));
}

#[test]
#[serial]
fn scenario_traverse_bounded_original_pages_without_mixing_sessions() {
    // @step Given two persisted sessions containing multiple structured envelopes
    let mut fixture = Fixture::new();
    let other =
        create_session("unrelated", fixture.directory.path()).expect("create other session");
    let other_id = other.id;
    let mut other = other;
    append_message(&mut other, "user", "unrelated preview").expect("persist unrelated message");
    let expected: Vec<_> = (0..12)
        .map(|index| fixture.append(envelope(&index.to_string())))
        .collect();
    let strategy = (0usize..12, 0usize..12, 1usize..8);
    let mut runner = proptest::test_runner::TestRunner::default();
    runner
        .run(&strategy, |(first, last, limit)| {
            let start = first.min(last);
            let end = first.max(last);
            let mut next = start;
            let mut actual = Vec::new();
            loop {
                // @step When bounded forward pages are retrieved for one session and inclusive turn range
                let page = fixture.request(
                    json!({"start_turn":start, "end_turn":end, "next_turn":next, "limit":limit}),
                );
                // @step Then each selected original appears exactly once in persisted order
                prop_assert_eq!(&page["session_id"], &json!(fixture.session.id));
                prop_assert_ne!(&page["session_id"], &json!(other_id));
                let records = page["records"].as_array().expect("original records");
                prop_assert!(!records.is_empty() && records.len() <= limit);
                for record in records {
                    prop_assert_eq!(
                        record["turn_index"].as_u64(),
                        Some((start + actual.len()) as u64)
                    );
                    actual.push(
                        record["message_id"]
                            .as_str()
                            .expect("persisted id")
                            .to_string(),
                    );
                }
                // @step And the final page has no continuation
                if page["next_turn"].is_null() {
                    break;
                }
                // @step And continuation identifies the next unread position within that range
                let continuation = page["next_turn"].as_u64().expect("next position") as usize;
                prop_assert_eq!(continuation, next + records.len());
                next = continuation;
                prop_assert!(actual.len() <= expected.len());
            }
            prop_assert_eq!(
                actual,
                expected[start..=end]
                    .iter()
                    .map(Uuid::to_string)
                    .collect::<Vec<_>>()
            );
            Ok(())
        })
        .expect("bounded pages preserve order, range, and exact-once traversal");
}

#[test]
#[serial]
fn scenario_reject_missing_original_messages_without_partial_success() {
    // @step Given a session references a missing original after a valid envelope
    let mut fixture = Fixture::new();
    fixture.append(envelope("valid first"));
    fixture
        .session
        .add_message(Uuid::new_v4(), MessageSource::Native);
    save_session(&fixture.session).expect("persist missing reference");
    let before = serde_json::to_value(load_session(fixture.session.id).expect("load before"))
        .expect("serialize before");
    // @step When original retrieval includes both references
    let result = fixture.request(json!({}));
    // @step Then a typed missing-message failure is returned without any partial page
    assert_error(&result, "missing_message");
    // @step And persisted source state remains unchanged
    let after = serde_json::to_value(load_session(fixture.session.id).expect("load after"))
        .expect("serialize after");
    assert_eq!(before, after);
}

#[test]
#[serial]
fn scenario_resolve_original_blobs_and_reject_unavailable_blob_evidence() {
    // @step Given structured originals contain stored tool result and tool input blobs
    let mut fixture = Fixture::new();
    let text = "blob evidence 証拠".repeat(100);
    let hash = store_blob(text.as_bytes()).expect("store tool result blob");
    let input = json!({"nested":{"unknown":[true, "🦀"]}});
    let input_hash = store_blob(input.to_string().as_bytes()).expect("store input blob");
    let mut original = blob_envelope(&hash);
    original["message"]["content"]
        .as_array_mut()
        .expect("blocks")
        .push(json!({
            "type":"tool_use", "id":"call-2", "name":"write", "provider_field":99,
            "input":{"_blob_ref":make_blob_reference(&input_hash)}
        }));
    original["_blobRefs"]
        .as_array_mut()
        .expect("recorded blob references")
        .push(json!({"key":"tool_use:1:call-2", "hash":input_hash}));
    fixture.append(original.clone());
    original["message"]["content"][0]["content"] = json!(text);
    original["message"]["content"][1]["input"] = input;
    // @step When original retrieval is requested
    let result = fixture.request(json!({}));
    // @step Then blob content is restored exactly while unknown nested fields remain unchanged
    assert_eq!(result["records"][0]["envelope"], original);
    // @step But a missing or invalid blob returns a typed failure without a partial page
    let malformed = store_blob(&[0xff, 0xfe]).expect("store invalid UTF-8 blob");
    fixture.append(blob_envelope(&malformed));
    assert_error(&fixture.request(json!({})), "invalid_blob");
    fixture.session.messages.pop();
    save_session(&fixture.session).expect("restore valid manifest");
    fixture.append(blob_envelope(&"a".repeat(64)));
    assert_error(&fixture.request(json!({})), "invalid_blob");
}

#[test]
#[serial]
fn scenario_reject_preview_only_sources_instead_of_fabricating_envelopes() {
    // @step Given a persisted message has only searchable preview content
    let mut fixture = Fixture::new();
    append_message(&mut fixture.session, "user", "only a preview").expect("persist legacy message");
    // @step When original retrieval is requested
    let result = fixture.request(json!({}));
    // @step Then a typed missing-envelope failure is returned instead of a fabricated original
    assert_error(&result, "missing_envelope");
    assert_error(
        &fixture.request(json!({"session_id":Uuid::new_v4()})),
        "source_unavailable",
    );
}

#[test]
#[serial]
fn scenario_enforce_original_page_bounds_without_truncating_evidence() {
    // @step Given persisted originals and explicit page size and byte limits
    let mut fixture = Fixture::new();
    fixture.append(envelope(&"証".repeat(150)));
    fixture.append(envelope(&"🦀".repeat(150)));
    // @step When invalid ranges or limits are requested
    for invalid in [
        json!({"start_turn":1,"end_turn":0}),
        json!({"limit":0}),
        json!({"limit":101}),
        json!({"max_bytes":0}),
        json!({"max_bytes":61441}),
        json!({"next_turn":3}),
        json!({"session_id":"not-a-uuid"}),
    ] {
        // @step Then a typed invalid-request failure is returned
        assert_error(&fixture.request(invalid), "invalid_request");
    }
    let first = fixture.request(json!({"limit":1}));
    let bound = serde_json::to_vec(&first)
        .expect("measure serialized response")
        .len();
    let page = fixture.request(json!({"max_bytes":bound}));
    // @step But valid byte-limited pages stop before the next complete envelope
    assert_eq!(page["records"].as_array().expect("records").len(), 1);
    assert_eq!(page["next_turn"], 1);
    assert!(serde_json::to_vec(&page).expect("serialize page").len() <= bound);
    // @step And an envelope exceeding the byte budget returns a typed oversized-envelope failure
    assert_error(
        &fixture.request(json!({"max_bytes":100})),
        "oversized_envelope",
    );
    let empty = fixture.request(json!({"start_turn":2}));
    assert_eq!(empty["records"], json!([]));
    assert!(empty["next_turn"].is_null());
}

#[tokio::test]
#[serial]
async fn scenario_advertise_original_retrieval_and_source_selection_to_dag_builders() {
    // @step Given the SessionSearch tool definition and DAG builder instructions
    let definition = codelet_tools::session_search::SessionSearchTool::new(Uuid::new_v4())
        .definition(String::new())
        .await;
    let schema = definition.parameters;
    // @step When the original retrieval contract is inspected
    // @step Then explicit session-scoped original retrieval and its bounds are discoverable
    assert!(schema["properties"]["action_type"]["enum"]
        .as_array()
        .expect("action choices")
        .contains(&json!("originals")));
    for field in [
        "session_id",
        "start_turn",
        "end_turn",
        "next_turn",
        "limit",
        "max_bytes",
    ] {
        assert!(schema["properties"].get(field).is_some(), "missing {field}");
    }
    let mut fixture = Fixture::new();
    fixture.append(envelope("wire evidence"));
    let tool = codelet_tools::session_search::SessionSearchTool::new(fixture.session.id);
    let page = fixture.request(json!({}));
    let bound = serde_json::to_vec(&page)
        .expect("measure wire budget")
        .len();
    let args = serde_json::from_value(json!({
        "action_type":"originals", "session_id":fixture.session.id, "max_bytes":bound
    }))
    .expect("parse original tool arguments");
    let wire = tool
        .call(args)
        .await
        .expect("retrieve original through the registered tool");
    assert!(wire.len() <= bound);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&wire).expect("wire JSON"),
        page
    );
    // @step And builders are instructed to select originals rather than treating previews as exact evidence
    for existing in [
        None,
        Some(("<dag-node>existing</dag-node>".to_string(), 5usize)),
    ] {
        let prompt =
            codelet_cli::compaction_dag::build_generate_compaction_prompt(Uuid::new_v4(), existing);
        assert!(prompt.contains("originals"));
        assert!(prompt.contains("preview"));
        assert!(prompt.contains("max_bytes"));
        assert!(!prompt.contains("<session UUID>"));
    }
}
