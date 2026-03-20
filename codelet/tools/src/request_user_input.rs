//! Request User Input HITL Tool — Provider-agnostic human-in-the-loop tool
//!
//! Feature: spec/features/request-user-input-hitl-tool.feature
//!
//! Allows any LLM to request structured user input mid-turn. The tool pauses
//! the agent loop, presents a structured question form to the user via the TUI,
//! and resumes with the user's answers.
//!
//! Uses the per-session handler pattern (like InjectSummaryHandler, SessionSearchHandler):
//! - Tool definition and JSON schema live here in codelet-tools
//! - A handler type alias is defined for the actual TUI interaction
//! - A global per-session handler registry stores handlers
//! - The actual TUI modal rendering lives in the NAPI handler
//! - When call() is invoked, the tool dispatches to the registered handler

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::ToolError;

// ============================================================================
// Data Types
// ============================================================================

/// An option presented to the user for a question
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HitlOption {
    /// User-facing label (1-5 words)
    pub label: String,
    /// One sentence explaining impact
    pub description: String,
}

/// A question to present to the user
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HitlQuestion {
    /// Stable snake_case identifier for mapping answers
    pub id: String,
    /// Short UI label (≤12 chars)
    pub header: String,
    /// Single-sentence prompt shown to user
    pub question: String,
    /// Optional mutually exclusive choices (2-3 items)
    pub options: Option<Vec<HitlOption>>,
}

/// Request sent to the HITL handler containing validated questions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HitlRequest {
    /// Array of 1-3 questions to present to the user
    pub questions: Vec<HitlQuestion>,
}

/// A single answer from the user for one question
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HitlAnswer {
    /// Labels of selected options
    pub selected: Vec<String>,
    /// Optional freeform text
    pub other: Option<String>,
}

/// Response from the HITL handler
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum HitlResponse {
    /// User provided answers
    Answered {
        /// Answers keyed by question id
        answers: HashMap<String, HitlAnswer>,
    },
    /// User cancelled the input modal
    Cancelled {
        /// Always true when cancelled
        cancelled: bool,
    },
}

// ============================================================================
// Validation
// ============================================================================

/// Maximum number of questions allowed
const MAX_QUESTIONS: usize = 3;
/// Maximum header length in characters
const MAX_HEADER_LEN: usize = 12;
/// Minimum options per question (when options are provided)
const MIN_OPTIONS: usize = 2;
/// Maximum options per question
const MAX_OPTIONS: usize = 3;

/// Check if a string is valid snake_case
fn is_snake_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        && !s.starts_with('_')
        && !s.ends_with('_')
        && !s.contains("__")
}

/// Validate HITL questions and return a descriptive error if invalid
fn validate_questions(questions: &[HitlQuestion]) -> Result<(), String> {
    if questions.is_empty() {
        return Err("questions array must not be empty".to_string());
    }
    if questions.len() > MAX_QUESTIONS {
        return Err(format!(
            "questions array exceeds maximum of {MAX_QUESTIONS} questions (got {})",
            questions.len()
        ));
    }

    for (i, q) in questions.iter().enumerate() {
        if q.id.is_empty() {
            return Err(format!("question[{i}].id must not be empty"));
        }
        if !is_snake_case(&q.id) {
            return Err(format!(
                "question[{i}].id '{}' must be snake_case (lowercase letters, digits, underscores)",
                q.id
            ));
        }
        if q.header.is_empty() {
            return Err(format!("question[{i}].header must not be empty"));
        }
        if q.header.chars().count() > MAX_HEADER_LEN {
            return Err(format!(
                "question[{i}].header '{}' exceeds maximum of {MAX_HEADER_LEN} characters (got {})",
                q.header,
                q.header.chars().count()
            ));
        }
        if q.question.is_empty() {
            return Err(format!("question[{i}].question must not be empty"));
        }

        if let Some(opts) = &q.options {
            if opts.len() < MIN_OPTIONS {
                return Err(format!(
                    "question[{i}].options requires at least {MIN_OPTIONS} items (got {})",
                    opts.len()
                ));
            }
            if opts.len() > MAX_OPTIONS {
                return Err(format!(
                    "question[{i}].options exceeds maximum of {MAX_OPTIONS} items (got {})",
                    opts.len()
                ));
            }
        }
    }

    Ok(())
}

// ============================================================================
// Per-Session Handler Registry
// ============================================================================

/// Handler function type for HITL execution.
/// Takes session_id and validated HitlRequest, returns HitlResponse or error.
/// The handler blocks synchronously until the TUI sends back the user's answers.
pub type HitlHandler =
    Arc<dyn Fn(Uuid, HitlRequest) -> Result<HitlResponse, String> + Send + Sync>;

/// Per-session handler storage
static HITL_HANDLERS: once_cell::sync::Lazy<RwLock<HashMap<Uuid, HitlHandler>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Set the HITL handler for a specific session
///
/// Called by session manager before agent run to configure how HITL
/// operations are executed for this session.
pub fn set_hitl_handler(session_id: Uuid, handler: Option<HitlHandler>) {
    if let Ok(mut guard) = HITL_HANDLERS.write() {
        match handler {
            Some(h) => {
                guard.insert(session_id, h);
            }
            None => {
                guard.remove(&session_id);
            }
        }
    }
}

/// Check if a HITL handler is configured for a specific session
pub fn has_hitl_handler(session_id: Uuid) -> bool {
    HITL_HANDLERS
        .read()
        .map(|guard| guard.contains_key(&session_id))
        .unwrap_or(false)
}

/// Execute HITL via the handler for a specific session
///
/// Called by RequestUserInputTool when the LLM invokes the tool.
/// Validates questions first, then dispatches to the registered handler.
pub fn execute_hitl(
    session_id: Uuid,
    request: HitlRequest,
) -> Result<HitlResponse, String> {
    // Validate questions before dispatching
    validate_questions(&request.questions)?;

    let handler = match HITL_HANDLERS.read() {
        Ok(guard) => guard.get(&session_id).cloned(),
        Err(_) => {
            return Err("Failed to acquire HITL handlers lock".to_string());
        }
    };

    match handler {
        Some(h) => h(session_id, request),
        None => Err(
            "request_user_input is unavailable in the current session mode".to_string(),
        ),
    }
}

/// Clear all HITL handlers (for testing)
pub fn clear_all_hitl_handlers() {
    if let Ok(mut guard) = HITL_HANDLERS.write() {
        guard.clear();
    }
}

// ============================================================================
// Tool Arguments and Rig Tool Implementation
// ============================================================================

/// Arguments for the request_user_input tool (deserialized from LLM JSON)
#[derive(Debug, Deserialize, Serialize)]
pub struct RequestUserInputArgs {
    /// Array of 1-3 questions to present to the user
    pub questions: Vec<HitlQuestion>,
}

/// RequestUserInputTool — Rig Tool implementation
///
/// Provider-agnostic HITL tool that pauses the agent loop to request
/// structured user input. Uses the per-session handler pattern.
#[derive(Clone, Debug)]
pub struct RequestUserInputTool {
    session_id: Uuid,
}

impl RequestUserInputTool {
    /// Create a new RequestUserInputTool instance
    ///
    /// # Arguments
    /// * `session_id` - The session ID for per-session handler lookup
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

impl Tool for RequestUserInputTool {
    const NAME: &'static str = "request_user_input";

    type Error = ToolError;
    type Args = RequestUserInputArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "request_user_input".to_string(),
            description: concat!(
                "Request structured input from the user. Presents a modal with ",
                "1-3 questions, each with optional multiple-choice options and ",
                "freeform text input. The agent loop pauses until the user responds. ",
                "Use when you need user preferences, decisions, or clarifications ",
                "that cannot be inferred from context."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "RequestUserInputArgs",
                "type": "object",
                "required": ["questions"],
                "properties": {
                    "questions": {
                        "type": "array",
                        "description": "Array of 1-3 questions to present to the user.",
                        "minItems": 1,
                        "maxItems": 3,
                        "items": {
                            "type": "object",
                            "required": ["id", "header", "question"],
                            "properties": {
                                "id": {
                                    "type": "string",
                                    "description": "Stable snake_case identifier for mapping answers."
                                },
                                "header": {
                                    "type": "string",
                                    "description": "Short label shown in UI (max 12 chars)."
                                },
                                "question": {
                                    "type": "string",
                                    "description": "Single-sentence prompt shown to user."
                                },
                                "options": {
                                    "type": "array",
                                    "description": "Optional mutually exclusive choices (2-3 items).",
                                    "minItems": 2,
                                    "maxItems": 3,
                                    "items": {
                                        "type": "object",
                                        "required": ["label", "description"],
                                        "properties": {
                                            "label": {
                                                "type": "string",
                                                "description": "1-5 word label. Suffix recommended option with '(Recommended)'."
                                            },
                                            "description": {
                                                "type": "string",
                                                "description": "One sentence explaining impact."
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-013: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            &self.name(),
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "request_user_input",
                message: reason,
            });
        }

        let request = HitlRequest {
            questions: args.questions,
        };

        let response = execute_hitl(self.session_id, request).map_err(|e| {
            ToolError::Execution {
                tool: "request_user_input",
                message: e,
            }
        })?;

        serde_json::to_string_pretty(&response).map_err(|e| ToolError::Execution {
            tool: "request_user_input",
            message: format!("Failed to serialize response: {e}"),
        })
    }
}


#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Feature: spec/features/request-user-input-hitl-tool.feature

    // ====================================================================
    // Helper: build a valid question with options
    // ====================================================================
    fn make_question(id: &str, header: &str, question: &str) -> HitlQuestion {
        HitlQuestion {
            id: id.to_string(),
            header: header.to_string(),
            question: question.to_string(),
            options: Some(vec![
                HitlOption {
                    label: "Option A".to_string(),
                    description: "First choice".to_string(),
                },
                HitlOption {
                    label: "Option B".to_string(),
                    description: "Second choice".to_string(),
                },
            ]),
        }
    }

    fn make_question_no_options(id: &str, header: &str, question: &str) -> HitlQuestion {
        HitlQuestion {
            id: id.to_string(),
            header: header.to_string(),
            question: question.to_string(),
            options: None,
        }
    }

    // ====================================================================
    // Scenario: Request user input with questions and options returns answers
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_request_with_questions_and_options_returns_answers() {
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
                        other: Some("Additional notes".to_string()),
                    },
                );
            }
            Ok(HitlResponse::Answered { answers })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with 2 questions each having 2 options
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![
                    make_question("approach", "Approach", "Which approach do you prefer?"),
                    make_question("priority", "Priority", "What is the priority?"),
                ],
            })
            .await;

        // @step Then the tool should block until the handler returns
        assert!(result.is_ok());

        // @step And the response should contain answers keyed by question id
        let output = result.unwrap();
        let response: HitlResponse = serde_json::from_str(&output).unwrap();

        // @step And each answer should contain selected labels and optional freeform text
        match response {
            HitlResponse::Answered { answers } => {
                assert_eq!(answers.len(), 2);
                let approach = answers.get("approach").unwrap();
                assert_eq!(approach.selected, vec!["Option A"]);
                assert_eq!(approach.other, Some("Additional notes".to_string()));
                let priority = answers.get("priority").unwrap();
                assert_eq!(priority.selected, vec!["Option A"]);
            }
            _ => panic!("Expected Answered response"),
        }

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Request user input in headless mode returns error
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_headless_mode_returns_error() {
        clear_all_hitl_handlers();

        // @step Given no HITL handler is registered for the current session
        let session_id = Uuid::new_v4();

        // @step When the agent calls request_user_input with valid questions
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question("test_q", "Test", "Test question?")],
            })
            .await;

        // @step Then the tool should return error "request_user_input is unavailable in the current session mode"
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("request_user_input is unavailable in the current session mode"),
            "Expected mode error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects header longer than 12 characters
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_long_header() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question header "This Is Too Long"
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question("test_q", "This Is Too Long", "A question?")],
            })
            .await;

        // @step Then the tool should return a validation error about header length exceeding 12 characters
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("exceeds maximum of 12 characters"),
            "Expected header length error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects non-snake_case question id
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_non_snake_case_id() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question id "camelCase"
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question("camelCase", "Test", "A question?")],
            })
            .await;

        // @step Then the tool should return a validation error about id not being snake_case
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("must be snake_case"),
            "Expected snake_case error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: User cancellation returns cancelled response
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_cancellation_returns_cancelled_response() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        // @step And the handler will return a cancellation
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with valid questions
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question("test_q", "Test", "A question?")],
            })
            .await;
        assert!(result.is_ok());

        // @step Then the response should contain "cancelled" set to true
        // @step And the response should not contain "answers"
        let output = result.unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["cancelled"], true);
        assert!(value.get("answers").is_none());

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects more than 3 questions
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_too_many_questions() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with 4 questions
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![
                    make_question("q1", "Q1", "Question 1?"),
                    make_question("q2", "Q2", "Question 2?"),
                    make_question("q3", "Q3", "Question 3?"),
                    make_question("q4", "Q4", "Question 4?"),
                ],
            })
            .await;

        // @step Then the tool should return a validation error about exceeding the maximum of 3 questions
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("maximum of 3 questions"),
            "Expected max questions error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects fewer than 2 options per question
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_too_few_options() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question having 1 option
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![HitlQuestion {
                    id: "test_q".to_string(),
                    header: "Test".to_string(),
                    question: "A question?".to_string(),
                    options: Some(vec![HitlOption {
                        label: "Only One".to_string(),
                        description: "The only option".to_string(),
                    }]),
                }],
            })
            .await;

        // @step Then the tool should return a validation error about options requiring at least 2 items
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("at least 2 items"),
            "Expected min options error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Question without options accepts freeform-only input
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_freeform_only_question() {
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
                    other: Some("User typed this freeform text".to_string()),
                },
            );
            Ok(HitlResponse::Answered { answers })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question without options
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question_no_options(
                    "feedback",
                    "Feedback",
                    "Any additional feedback?",
                )],
            })
            .await;
        assert!(result.is_ok());

        // @step Then the response should contain an answer with empty selected array
        // @step And the answer should contain populated freeform text in the other field
        let output = result.unwrap();
        let response: HitlResponse = serde_json::from_str(&output).unwrap();
        match response {
            HitlResponse::Answered { answers } => {
                let feedback = answers.get("feedback").unwrap();
                assert!(feedback.selected.is_empty());
                assert_eq!(
                    feedback.other,
                    Some("User typed this freeform text".to_string())
                );
            }
            _ => panic!("Expected Answered response"),
        }

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Handler registry lifecycle management
    // ====================================================================
    #[test]
    #[serial]
    fn test_handler_registry_lifecycle() {
        clear_all_hitl_handlers();

        let session_id = Uuid::parse_str("00000000-0000-0000-0000-0000000abc23").unwrap();

        // @step Given no HITL handler is registered for session "abc-123"
        assert!(!has_hitl_handler(session_id));

        // @step When set_hitl_handler is called for session "abc-123"
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step Then has_hitl_handler should return true for session "abc-123"
        assert!(has_hitl_handler(session_id));

        // @step When clear_all_hitl_handlers is called
        clear_all_hitl_handlers();

        // @step Then has_hitl_handler should return false for session "abc-123"
        assert!(!has_hitl_handler(session_id));
    }

    // ====================================================================
    // Scenario: Validation rejects empty questions array
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_empty_questions() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with an empty questions array
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![],
            })
            .await;

        // @step Then the tool should return a validation error about questions being required
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("must not be empty"),
            "Expected empty questions error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects question with empty id
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_empty_id() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question having an empty id
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question("", "Test", "A question?")],
            })
            .await;

        // @step Then the tool should return a validation error about id being required
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("id must not be empty"),
            "Expected empty id error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects question with empty header
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_empty_header() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question having an empty header
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question_no_options("test_q", "", "A question?")],
            })
            .await;

        // @step Then the tool should return a validation error about header being required
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("header must not be empty"),
            "Expected empty header error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects question with empty question text
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_empty_question_text() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question having an empty question text
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![make_question_no_options("test_q", "Test", "")],
            })
            .await;

        // @step Then the tool should return a validation error about question text being required
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("question must not be empty"),
            "Expected empty question error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Scenario: Validation rejects more than 3 options per question
    // ====================================================================
    #[tokio::test]
    #[serial]
    async fn test_validation_rejects_too_many_options() {
        clear_all_hitl_handlers();

        let session_id = Uuid::new_v4();

        // @step Given a HITL handler is registered for the current session
        let handler: HitlHandler = Arc::new(|_, _| {
            Ok(HitlResponse::Cancelled { cancelled: true })
        });
        set_hitl_handler(session_id, Some(handler));

        // @step When the agent calls request_user_input with a question having 4 options
        let tool = RequestUserInputTool::new(session_id);
        let result = tool
            .call(RequestUserInputArgs {
                questions: vec![HitlQuestion {
                    id: "test_q".to_string(),
                    header: "Test".to_string(),
                    question: "A question?".to_string(),
                    options: Some(vec![
                        HitlOption { label: "A".to_string(), description: "Opt A".to_string() },
                        HitlOption { label: "B".to_string(), description: "Opt B".to_string() },
                        HitlOption { label: "C".to_string(), description: "Opt C".to_string() },
                        HitlOption { label: "D".to_string(), description: "Opt D".to_string() },
                    ]),
                }],
            })
            .await;

        // @step Then the tool should return a validation error about options exceeding the maximum of 3 items
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("maximum of 3 items"),
            "Expected max options error, got: {err_msg}"
        );

        clear_all_hitl_handlers();
    }

    // ====================================================================
    // Additional: Tool definition has correct name and schema
    // ====================================================================
    #[tokio::test]
    async fn test_tool_definition_name_and_schema() {
        // @step Given the RequestUserInputTool is compiled
        let tool = RequestUserInputTool::new(Uuid::new_v4());

        // @step When the tool definition is requested
        let definition = tool.definition("".to_string()).await;

        // @step Then the tool name should be "request_user_input"
        assert_eq!(definition.name, "request_user_input");

        // @step And the JSON schema should have "questions" as a required parameter
        let params = &definition.parameters;
        let required = params["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("questions")));

        // @step And the questions items should have id, header, question as required
        let items = &params["properties"]["questions"]["items"];
        let item_required = items["required"].as_array().unwrap();
        assert!(item_required.iter().any(|v| v.as_str() == Some("id")));
        assert!(item_required.iter().any(|v| v.as_str() == Some("header")));
        assert!(item_required.iter().any(|v| v.as_str() == Some("question")));
    }

    // ====================================================================
    // Additional: is_snake_case validation
    // ====================================================================
    #[test]
    fn test_is_snake_case() {
        assert!(is_snake_case("hello"));
        assert!(is_snake_case("hello_world"));
        assert!(is_snake_case("test_123"));
        assert!(is_snake_case("a"));
        assert!(!is_snake_case(""));
        assert!(!is_snake_case("camelCase"));
        assert!(!is_snake_case("PascalCase"));
        assert!(!is_snake_case("kebab-case"));
        assert!(!is_snake_case("_leading"));
        assert!(!is_snake_case("trailing_"));
        assert!(!is_snake_case("double__under"));
        assert!(!is_snake_case("UPPER_CASE"));
    }
}
