//! Feature: spec/features/lossless-original-envelope-retrieval.feature

#[path = "fixtures/cmpct063.rs"]
mod fixture;

use codelet_core::persistence::{
    make_blob_reference, process_envelope_for_blob_storage, save_session, store_blob,
    MessageEnvelope,
};
use fixture::{assert_error, blob_envelope, envelope, Fixture};
use serde_json::{json, Value};
use serial_test::serial;

#[test]
#[serial]
fn scenario_preserve_literal_blob_like_content_without_recorded_storage_references() {
    // @step Given original messages contain literal blob-like text and tool input fields
    let mut fixture = Fixture::new();
    let hash = store_blob(b"must not replace literal evidence").expect("store unrelated blob");
    let reference = make_blob_reference(&hash);
    let mut original = envelope(&reference);
    original["message"]["content"] = json!([
        {"type":"tool_result", "content":reference},
        {"type":"thinking", "thinking":"blob:sha256:not-a-hash"},
        {"type":"image", "source":{"type":"url", "data":reference}},
        {"type":"document", "source":{"data":reference}},
        {"type":"tool_use", "input":{"_blob_ref":reference, "retain":{"unknown":true}}},
        {"type":"tool_use", "input":{"_blob_ref":"ordinary input"}},
        {"type":"tool_use", "input":{"_blob_ref":42}}
    ]);
    // @step And those fields have no recorded storage references
    assert!(original.get("_blobRefs").is_none());
    fixture.append(original.clone());
    // @step When original retrieval is requested
    let page = fixture.request(json!({}));
    // @step Then every literal value and unknown input field is returned unchanged
    assert_eq!(page["result_type"], "originals", "{page}");
    assert_eq!(page["records"][0]["envelope"], original);
}

#[test]
#[serial]
fn scenario_validate_recorded_blob_references_before_returning_original_evidence() {
    // @step Given originals externalized by the persistence writer with recorded storage references
    let mut fixture = Fixture::new();
    assert!(fixture.directory.path().exists());
    let large = "証拠🦀".repeat(1200);
    let mut user = envelope(&large);
    user["message"]["content"] = json!([
        {"type":"tool_result", "tool_use_id":"call-1", "content":large},
        {"type":"image", "source":{"type":"base64", "media_type":"image/png", "data":large}},
        {"type":"document", "source":{"type":"base64", "media_type":"application/pdf", "data":large}}
    ]);
    let mut assistant = envelope("");
    assistant["type"] = json!("assistant");
    assistant["message"] = json!({"role":"assistant", "content":[
        {"type":"thinking", "thinking":large, "signature":"retain"},
        {"type":"tool_use", "id":"call:with:colons", "name":"write", "input":{"content":large}}
    ]});
    for original in [user, assistant] {
        let typed: MessageEnvelope = serde_json::from_value(original).expect("writer envelope");
        let (processed, references) =
            process_envelope_for_blob_storage(&typed).expect("externalize real envelope");
        assert!(!references.is_empty());
        let registry: Value = references
            .iter()
            .map(|(key, hash)| json!({"key":key, "hash":hash}))
            .collect();
        let mut stored = serde_json::to_value(processed).expect("stored envelope");
        stored["_blobRefs"] = registry.clone();
        stored["provider_extension"] = json!({"unknown":[1,true]});
        let mut expected = serde_json::to_value(typed).expect("original envelope");
        expected["_blobRefs"] = registry;
        expected["provider_extension"] = stored["provider_extension"].clone();
        fixture.append(stored);
        // @step When original retrieval is requested
        let page = fixture.request(json!({"start_turn":fixture.session.messages.len()-1}));
        // @step Then all recorded content is restored without changing other envelope fields
        assert_eq!(page["result_type"], "originals", "{page}");
        assert_eq!(page["records"][0]["envelope"], expected);
    }
    let hash = store_blob(b"registered evidence").expect("store blob");
    let baseline = blob_envelope(&hash);
    let mut invalid = Vec::new();
    for registry in [
        json!("invalid registry"),
        json!([{"key":"tool_result:0:wrong-id", "hash":hash}]),
        json!([{"key":"tool_result:0:call-1", "hash":"a".repeat(64)}]),
        json!([{"key":"tool_result:99:call-1", "hash":hash}]),
        json!([{"key":"unknown:0", "hash":hash}]),
        json!([{"hash":hash}]),
        json!([
            {"key":"tool_result:0:call-1", "hash":hash},
            {"key":"tool_result:0:call-1", "hash":hash}
        ]),
        json!([{"key":"tool_result:not-an-index:call-1", "hash":hash}]),
    ] {
        let mut original = baseline.clone();
        original["_blobRefs"] = registry;
        invalid.push(original);
    }
    let mut extended_input = envelope("");
    extended_input["message"]["content"] = json!([{
        "type":"tool_use", "id":"call-1", "input":{
            "_blob_ref":make_blob_reference(&hash), "must_not_be_discarded":true
        }
    }]);
    extended_input["_blobRefs"] = json!([{"key":"tool_use:0:call-1", "hash":hash}]);
    invalid.push(extended_input);
    // @step But inconsistent or unsupported recorded references return a typed failure without partial evidence
    for original in invalid {
        fixture.append(original);
        assert_error(
            &fixture.request(json!({"start_turn":fixture.session.messages.len()-1})),
            "invalid_blob",
        );
        fixture.session.messages.pop();
        save_session(&fixture.session).expect("remove invalid reference");
    }
}
