//! Anchor Point NAPI Layer Tests

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::persistence::{add_anchor_point, create_session, get_anchor_points, load_session};
use codelet_napi::test_support::{
    create_error_resolution_anchor, create_feature_milestone_anchor, create_task_completion_anchor,
    create_user_checkpoint_anchor, setup_test_env,
};
use std::path::PathBuf;

#[test]
fn test_persisted_anchor_point_serializes_for_napi() {
    let anchor = create_error_resolution_anchor(10);
    let json = serde_json::to_string(&anchor).expect("serialize");

    assert!(json.contains("\"turn_index\":10"));
    assert!(json.contains("\"anchor_type\":\"ErrorResolution\""));
    assert!(json.contains("\"weight\":0.9"));
    assert!(json.contains("\"confidence\":0.95"));
    assert!(json.contains("\"description\":"));
    assert!(json.contains("\"timestamp_ms\":"));
    assert!(json.contains("\"user_message\":"));
    assert!(json.contains("\"assistant_response\":"));
    assert!(json.contains("\"tool_calls\":"));
}

#[test]
fn test_persisted_anchor_point_deserializes_for_napi() {
    let json = r#"{
        "turn_index": 25,
        "anchor_type": "TaskCompletion",
        "weight": 0.85,
        "confidence": 0.9,
        "description": "Test from NAPI",
        "timestamp_ms": 1738713600000,
        "user_message": "User asked something",
        "assistant_response": "Assistant responded",
        "tool_calls": [{"tool": "Read", "success": true}]
    }"#;

    let anchor: codelet_napi::persistence::PersistedAnchorPoint =
        serde_json::from_str(json).expect("deserialize");

    assert_eq!(anchor.turn_index, 25);
    assert_eq!(anchor.anchor_type, "TaskCompletion");
    assert_eq!(anchor.description, "Test from NAPI");
    assert_eq!(anchor.timestamp_ms, 1738713600000);
}

#[test]
fn test_anchor_type_enum_values_serialize_correctly() {
    let error_resolution = create_error_resolution_anchor(0);
    assert_eq!(error_resolution.anchor_type, "ErrorResolution");

    let task_completion = create_task_completion_anchor(0);
    assert_eq!(task_completion.anchor_type, "TaskCompletion");

    let user_checkpoint = create_user_checkpoint_anchor(0);
    assert_eq!(user_checkpoint.anchor_type, "UserCheckpoint");

    let feature_milestone = create_feature_milestone_anchor(0);
    assert_eq!(feature_milestone.anchor_type, "FeatureMilestone");
}

#[test]
fn test_anchor_weights_are_reasonable() {
    let error = create_error_resolution_anchor(0);
    let task = create_task_completion_anchor(0);
    let milestone = create_feature_milestone_anchor(0);
    let checkpoint = create_user_checkpoint_anchor(0);

    assert!(error.weight >= 0.0 && error.weight <= 1.0);
    assert!(task.weight >= 0.0 && task.weight <= 1.0);
    assert!(milestone.weight >= 0.0 && milestone.weight <= 1.0);
    assert!(checkpoint.weight >= 0.0 && checkpoint.weight <= 1.0);

    assert!(error.weight >= task.weight);
}

#[test]
fn test_get_anchor_points_returns_correct_format() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/napi_format");

    let mut session = create_session("NAPI Format Test", &project).expect("create");
    add_anchor_point(&mut session, create_error_resolution_anchor(5)).expect("add");
    add_anchor_point(&mut session, create_task_completion_anchor(15)).expect("add");

    let anchors = get_anchor_points(&session);
    assert_eq!(anchors.len(), 2);

    for anchor in &anchors {
        assert!(anchor.turn_index > 0 || anchor.turn_index == 5);
        assert!(!anchor.anchor_type.is_empty());
        assert!(!anchor.description.is_empty());
        assert!(anchor.timestamp_ms > 0);
    }
}

#[test]
fn test_anchors_order_preserved_for_napi() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/napi_order");

    let mut session = create_session("NAPI Order Test", &project).expect("create");

    add_anchor_point(&mut session, create_task_completion_anchor(20)).expect("add");
    add_anchor_point(&mut session, create_error_resolution_anchor(5)).expect("add");
    add_anchor_point(&mut session, create_feature_milestone_anchor(35)).expect("add");

    let anchors = get_anchor_points(&session);

    assert_eq!(anchors[0].turn_index, 20);
    assert_eq!(anchors[1].turn_index, 5);
    assert_eq!(anchors[2].turn_index, 35);
}

#[test]
fn test_timestamp_milliseconds_conversion() {
    let anchor = create_task_completion_anchor(10);
    let timestamp_ms = anchor.timestamp_ms;

    assert!(timestamp_ms > 1700000000000);
    assert!(timestamp_ms < 2000000000000);

    let json = serde_json::to_string(&anchor).unwrap();
    let restored: codelet_napi::persistence::PersistedAnchorPoint =
        serde_json::from_str(&json).unwrap();
    let back_to_ms = restored.timestamp_ms;

    assert_eq!(back_to_ms, timestamp_ms);
}

#[test]
fn test_anchors_survive_session_reload_for_napi() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/napi_reload");

    let session_id = {
        let mut session = create_session("NAPI Reload Test", &project).expect("create");
        add_anchor_point(&mut session, create_error_resolution_anchor(10)).expect("add");
        add_anchor_point(&mut session, create_task_completion_anchor(25)).expect("add");
        session.id
    };

    let reloaded = load_session(session_id).expect("reload");
    let anchors = get_anchor_points(&reloaded);

    assert_eq!(anchors.len(), 2);
    assert_eq!(anchors[0].turn_index, 10);
    assert_eq!(anchors[1].turn_index, 25);
}

#[test]
fn test_empty_anchors_should_not_cause_errors() {
    let (_guard, _temp_dir) = setup_test_env();
    let project = PathBuf::from("/test/project/napi_empty");

    let session = create_session("NAPI Empty Test", &project).expect("create");
    let anchors = get_anchor_points(&session);

    assert!(anchors.is_empty());

    let json = serde_json::to_string(&anchors).expect("serialize empty");
    assert_eq!(json, "[]");
}

#[test]
fn test_tool_calls_array_serializes() {
    let anchor = create_error_resolution_anchor(5);

    assert!(!anchor.tool_calls.is_empty());

    let json = serde_json::to_string(&anchor).unwrap();
    assert!(json.contains("\"tool_calls\""));
    assert!(json.contains("\"tool\":\"Edit\""));
    assert!(json.contains("\"success\":true"));
}

#[test]
fn test_optional_fields_serialize_as_null_or_missing() {
    use codelet_napi::persistence::PersistedAnchorPoint;

    let anchor = PersistedAnchorPoint {
        turn_index: 5,
        anchor_type: "TaskCompletion".to_string(),
        weight: 0.8,
        confidence: 0.9,
        description: "Minimal anchor".to_string(),
        timestamp_ms: 1738713600000,
        user_message: None,
        assistant_response: None,
        tool_calls: vec![],
    };

    let json = serde_json::to_string(&anchor).unwrap();
    let restored: PersistedAnchorPoint = serde_json::from_str(&json).unwrap();

    assert!(restored.user_message.is_none());
    assert!(restored.assistant_response.is_none());
    assert!(restored.tool_calls.is_empty());
}
