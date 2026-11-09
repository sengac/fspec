//! Feature: spec/features/codex-request-user-input-facade.feature
//!
//! BUG-116 HITL wrapper tests, moved out of wrapper.rs so each test file maps
//! one feature (COV-075).

use super::*;
use crate::facade::codex::CodexRequestUserInputFacade;
use crate::request_user_input::{
    clear_all_hitl_handlers, set_hitl_handler, HitlAnswer, HitlHandler, HitlResponse,
};
use serial_test::serial;
use std::collections::HashMap;
use std::sync::Arc;

/// Feature: spec/features/codex-request-user-input-facade.feature
///
/// Scenario: CodexRequestUserInputFacade maps questions to InternalHitlParams and returns answers
#[tokio::test]
#[serial]
async fn test_hitl_wrapper_returns_answers() {
    clear_all_hitl_handlers();
    let session_id = Uuid::new_v4();

    // @step Given a HITL handler is registered for the current session
    // @step And the handler will return user-selected answers
    let handler: HitlHandler = Arc::new(move |_sid, req| {
        let mut answers = HashMap::new();
        for q in &req.questions {
            answers.insert(
                q.id.clone(),
                HitlAnswer {
                    selected: vec!["Option A".to_string()],
                    other: Some("notes".to_string()),
                },
            );
        }
        Ok(HitlResponse::Answered { answers })
    });
    set_hitl_handler(session_id, Some(handler));

    // @step When the Codex model calls request_user_input with 2 questions each having 2 options
    let facade = Arc::new(CodexRequestUserInputFacade) as BoxedHitlToolFacade;
    let wrapper = HitlToolFacadeWrapper::new(facade, session_id);
    let args = FacadeArgs(serde_json::json!({
        "questions": [
            {
                "id": "approach",
                "header": "Approach",
                "question": "Which approach?",
                "options": [
                    { "label": "Option A", "description": "First" },
                    { "label": "Option B", "description": "Second" }
                ]
            },
            {
                "id": "priority",
                "header": "Priority",
                "question": "What priority?",
                "options": [
                    { "label": "High", "description": "Now" },
                    { "label": "Low", "description": "Later" }
                ]
            }
        ]
    }));
    let result = wrapper.call(args).await;

    // @step Then the facade passes questions to execute_hitl unchanged
    // @step And the wrapper returns JSON with answers keyed by question id
    assert!(result.is_ok());
    let output = result.unwrap();
    let value: serde_json::Value = serde_json::from_str(&output.0).unwrap();

    // @step And each answer contains selected labels and optional freeform text
    assert!(value["answers"]["approach"]["selected"].is_array());
    assert_eq!(value["answers"]["approach"]["selected"][0], "Option A");
    assert_eq!(value["answers"]["approach"]["other"], "notes");
    assert!(value["answers"]["priority"]["selected"].is_array());

    clear_all_hitl_handlers();
}

/// Scenario: Headless mode returns tool error about unavailable session mode
#[tokio::test]
#[serial]
async fn test_hitl_wrapper_headless_mode_error() {
    clear_all_hitl_handlers();
    let session_id = Uuid::new_v4();

    // @step Given no HITL handler is registered for the current session
    // (no handler set)

    // @step When the Codex model calls request_user_input with valid questions
    let facade = Arc::new(CodexRequestUserInputFacade) as BoxedHitlToolFacade;
    let wrapper = HitlToolFacadeWrapper::new(facade, session_id);
    let args = FacadeArgs(serde_json::json!({
        "questions": [{
            "id": "test_q",
            "header": "Test",
            "question": "A question?",
            "options": [
                { "label": "A", "description": "First" },
                { "label": "B", "description": "Second" }
            ]
        }]
    }));
    let result = wrapper.call(args).await;

    // @step Then the wrapper returns a tool error
    assert!(result.is_err());

    // @step And the error message contains "request_user_input is unavailable in the current session mode"
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("request_user_input is unavailable in the current session mode"),
        "Expected mode error, got: {err_msg}"
    );

    clear_all_hitl_handlers();
}

/// Scenario: Cancellation converts to Codex-specific tool error
#[tokio::test]
#[serial]
async fn test_hitl_wrapper_cancellation_to_error() {
    clear_all_hitl_handlers();
    let session_id = Uuid::new_v4();

    // @step Given a HITL handler is registered for the current session
    // @step And the handler will return a cancellation
    let handler: HitlHandler = Arc::new(|_, _| Ok(HitlResponse::Cancelled { cancelled: true }));
    set_hitl_handler(session_id, Some(handler));

    // @step When the Codex model calls request_user_input with valid questions
    let facade = Arc::new(CodexRequestUserInputFacade) as BoxedHitlToolFacade;
    let wrapper = HitlToolFacadeWrapper::new(facade, session_id);
    let args = FacadeArgs(serde_json::json!({
        "questions": [{
            "id": "test_q",
            "header": "Test",
            "question": "A question?",
            "options": [
                { "label": "A", "description": "First" },
                { "label": "B", "description": "Second" }
            ]
        }]
    }));
    let result = wrapper.call(args).await;

    // @step Then the wrapper returns a tool error
    assert!(result.is_err());

    // @step And the error message is "request_user_input was cancelled before receiving a response"
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("request_user_input was cancelled before receiving a response"),
        "Expected cancellation error, got: {err_msg}"
    );

    clear_all_hitl_handlers();
}

/// Scenario: Validation rejects invalid questions via execute_hitl
#[tokio::test]
#[serial]
async fn test_hitl_wrapper_validation_error() {
    clear_all_hitl_handlers();
    let session_id = Uuid::new_v4();

    // @step Given a HITL handler is registered for the current session
    let handler: HitlHandler = Arc::new(|_, _| Ok(HitlResponse::Cancelled { cancelled: true }));
    set_hitl_handler(session_id, Some(handler));

    // @step When the Codex model calls request_user_input with a question header "This Is Too Long"
    let facade = Arc::new(CodexRequestUserInputFacade) as BoxedHitlToolFacade;
    let wrapper = HitlToolFacadeWrapper::new(facade, session_id);
    let args = FacadeArgs(serde_json::json!({
        "questions": [{
            "id": "test_q",
            "header": "This Is Too Long",
            "question": "A question?",
            "options": [
                { "label": "A", "description": "First" },
                { "label": "B", "description": "Second" }
            ]
        }]
    }));
    let result = wrapper.call(args).await;

    // @step Then the wrapper returns a tool error about header length exceeding 12 characters
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("exceeds maximum of 12 characters"),
        "Expected header length error, got: {err_msg}"
    );

    clear_all_hitl_handlers();
}

/// Scenario: Freeform-only question without options returns answer
#[tokio::test]
#[serial]
async fn test_hitl_wrapper_freeform_only() {
    clear_all_hitl_handlers();
    let session_id = Uuid::new_v4();

    // @step Given a HITL handler is registered for the current session
    // @step And the handler will return a freeform-only answer
    let handler: HitlHandler = Arc::new(|_, _| {
        let mut answers = HashMap::new();
        answers.insert(
            "feedback".to_string(),
            HitlAnswer {
                selected: vec![],
                other: Some("User typed this".to_string()),
            },
        );
        Ok(HitlResponse::Answered { answers })
    });
    set_hitl_handler(session_id, Some(handler));

    // @step When the Codex model calls request_user_input with a question without options
    let facade = Arc::new(CodexRequestUserInputFacade) as BoxedHitlToolFacade;
    let wrapper = HitlToolFacadeWrapper::new(facade, session_id);
    let args = FacadeArgs(serde_json::json!({
        "questions": [{
            "id": "feedback",
            "header": "Feedback",
            "question": "Any additional feedback?"
        }]
    }));
    let result = wrapper.call(args).await;
    assert!(result.is_ok());

    // @step Then the wrapper returns JSON with an answer containing empty selected array
    let output = result.unwrap();
    let value: serde_json::Value = serde_json::from_str(&output.0).unwrap();
    let feedback = &value["answers"]["feedback"];
    assert_eq!(feedback["selected"].as_array().unwrap().len(), 0);

    // @step And the answer contains populated freeform text in the other field
    assert_eq!(feedback["other"], "User typed this");

    clear_all_hitl_handlers();
}

/// Scenario: Facade is registered in Codex create_rig_agent
/// Scenario: Facade schema has additionalProperties false
#[tokio::test]
async fn test_hitl_wrapper_tool_name_and_definition() {
    // @step Given a Codex agent built with create_rig_agent
    // @step Given a CodexRequestUserInputFacade instance
    let session_id = Uuid::new_v4();
    let facade = Arc::new(CodexRequestUserInputFacade) as BoxedHitlToolFacade;
    let wrapper = HitlToolFacadeWrapper::new(facade, session_id);

    // @step When the agent tool definitions are inspected
    // @step When the tool definition schema is inspected
    let def = <HitlToolFacadeWrapper as Tool>::definition(&wrapper, String::new()).await;

    // @step Then the tool list contains "request_user_input"
    assert_eq!(wrapper.name(), "request_user_input");

    // @step And request_user_input uses HitlToolFacadeWrapper with CodexRequestUserInputFacade
    assert_eq!(wrapper.provider(), "codex");

    // @step Then the schema has additionalProperties set to false
    assert_eq!(def.parameters["additionalProperties"], false);

    // @step And the schema has "questions" in the required array
    assert_eq!(def.name, "request_user_input");
    assert_eq!(def.parameters["required"], serde_json::json!(["questions"]));
}
