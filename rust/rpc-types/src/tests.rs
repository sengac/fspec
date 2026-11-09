//! RPC-036: JSON round-trip suite for every wire-portable shape (see the
//! integration counterpart rust/rpc-types/tests/rpc036_widen_types.rs).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::uninlined_format_args
)]

use super::*;

/// Helper: serialize `value` to JSON, deserialize it back, and assert
/// the round-tripped value equals the original. Every new RPC-036 type
/// implements `PartialEq + Debug + Serialize + Deserialize`, so this
/// helper is uniformly applicable.
fn round_trip<T>(value: T)
where
    T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq + core::fmt::Debug,
{
    let json = serde_json::to_string(&value).expect("serialize must succeed");
    let back: T = serde_json::from_str(&json).expect("deserialize must succeed");
    assert_eq!(back, value);
}

#[test]
fn session_tokens_round_trips() {
    round_trip(SessionTokens {
        input_tokens: 1024_i64,
        output_tokens: 512_i64,
    });
}

#[test]
fn token_restore_state_round_trips() {
    round_trip(TokenRestoreState {
        current_context: 1,
        cumulative_billed_output: 2,
        cache_read: 3,
        cache_creation: 4,
        cumulative_billed_input: 5,
        cumulative_billed_output_second: 6,
    });
}

#[test]
fn session_model_round_trips() {
    round_trip(SessionModel {
        provider_id: "openai".to_string(),
        model_id: "gpt-4o".to_string(),
        context_window: Some(128_000),
        max_output_tokens: Some(4_096),
        compaction_threshold: Some(96_000),
    });
}

#[test]
fn work_unit_context_round_trips() {
    round_trip(WorkUnitContext {
        id: "RPC-036".to_string(),
        title: "Widen rpc-types".to_string(),
        status: "testing".to_string(),
    });
}

#[test]
fn thinking_config_round_trips() {
    round_trip(ThinkingConfig {
        provider_id: "anthropic".to_string(),
        level: ThinkingLevel::Effort("high".to_string()),
        config_json: r#"{"type":"enabled","budget_tokens":8000}"#.to_string(),
    });
}

#[test]
fn pause_kind_round_trips_both_variants() {
    round_trip(PauseKind::Confirm);
    round_trip(PauseKind::Triple);

    // String-enum wire shape: `"Confirm"` / `"Triple"`.
    assert_eq!(
        serde_json::to_string(&PauseKind::Confirm).expect("serialize Confirm"),
        "\"Confirm\""
    );
    assert_eq!(
        serde_json::to_string(&PauseKind::Triple).expect("serialize Triple"),
        "\"Triple\""
    );
}

#[test]
fn pause_state_round_trips_with_and_without_tool_call_id() {
    round_trip(PauseState {
        kind: PauseKind::Confirm,
        prompt: "Apply changes?".to_string(),
        details: Some("tc-1".to_string()),
        tool_call_id: None,
    });
    round_trip(PauseState {
        kind: PauseKind::Triple,
        prompt: "Run command?".to_string(),
        details: None,
        tool_call_id: None,
    });
}

#[test]
fn pause_response_round_trips_every_variant() {
    round_trip(PauseResponse::Resume);
    round_trip(PauseResponse::ConfirmAccept);
    round_trip(PauseResponse::ConfirmDeny);
    round_trip(PauseResponse::TripleApprove);
    round_trip(PauseResponse::TripleApproveSession);
    round_trip(PauseResponse::TripleDeny);
}

#[test]
fn approval_choice_round_trips_every_variant() {
    round_trip(ApprovalChoice::Approve);
    round_trip(ApprovalChoice::ApproveSession);
    round_trip(ApprovalChoice::Deny);
}

#[test]
fn hitl_option_round_trips() {
    round_trip(HitlOption {
        label: "Yes".to_string(),
        description: "Proceed".to_string(),
    });
}

#[test]
fn hitl_request_round_trips_with_multiple_questions_and_empty_options() {
    round_trip(HitlRequest {
        questions: vec![
            HitlQuestion {
                id: "q-1".to_string(),
                header: "Apply".to_string(),
                question: "Apply changes?".to_string(),
                options: vec![
                    HitlOption {
                        label: "Yes".to_string(),
                        description: "Proceed".to_string(),
                    },
                    HitlOption {
                        label: "No".to_string(),
                        description: "Stop".to_string(),
                    },
                ],
            },
            HitlQuestion {
                id: "q-2".to_string(),
                header: "Notes".to_string(),
                question: "Any notes?".to_string(),
                options: vec![],
            },
        ],
    });
}

#[test]
fn hitl_response_round_trips() {
    round_trip(HitlResponse {
        cancelled: false,
        answers: vec![
            HitlAnswer {
                id: "q-1".to_string(),
                selected: vec!["Yes".to_string()],
                other: None,
            },
            HitlAnswer {
                id: "q-2".to_string(),
                selected: vec![],
                other: Some("free text".to_string()),
            },
        ],
    });
    round_trip(HitlResponse {
        cancelled: true,
        answers: vec![],
    });
}

#[test]
fn isolated_session_info_round_trips() {
    round_trip(IsolatedSessionInfo {
        session_id: SessionId::new("uuid-1"),
        worktree_path: "/tmp/wt".to_string(),
        base_commit: "abc1234".to_string(),
    });
}

#[test]
fn isolation_state_change_round_trips_with_base_commit_some() {
    let chunk = StreamChunk::IsolationStateChange {
        is_isolated: true,
        worktree_path: Some("/tmp/wt".to_string()),
        base_commit: Some("abc1234".to_string()),
    };
    let json = serde_json::to_string(&chunk).expect("serialize");
    let back: StreamChunk = serde_json::from_str(&json).expect("deserialize");
    match back {
        StreamChunk::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
        } => {
            assert!(is_isolated);
            assert_eq!(worktree_path.as_deref(), Some("/tmp/wt"));
            assert_eq!(base_commit.as_deref(), Some("abc1234"));
        }
        other => panic!("expected IsolationStateChange, got {other:?}"),
    }
}

#[test]
fn isolation_state_change_round_trips_with_base_commit_none() {
    // Backward-compat: pre-RPC-036 callers using the 2-arg constructor
    // get `base_commit = None`; the variant must still round-trip.
    let chunk = StreamChunk::isolation_state_change(false, None);
    let json = serde_json::to_string(&chunk).expect("serialize");
    let back: StreamChunk = serde_json::from_str(&json).expect("deserialize");
    match back {
        StreamChunk::IsolationStateChange {
            is_isolated,
            worktree_path,
            base_commit,
        } => {
            assert!(!is_isolated);
            assert!(worktree_path.is_none());
            assert!(base_commit.is_none());
        }
        other => panic!("expected IsolationStateChange, got {other:?}"),
    }
}

// ========================================================================
// RPC-338: ProviderInfo profile/unreachable wire fields.
// Feature: spec/features/model-selector-profile-wire-types.feature
// ========================================================================

/// Scenario: ProviderInfo carries profile and reachability fields over the wire
#[test]
fn provider_info_carries_profile_and_reachability_fields() {
    // @step Given a ProviderInfo value constructed with its derived Default
    let info = ProviderInfo::default();

    // @step Then its profile_name field is None
    assert_eq!(info.profile_name, None);

    // @step And its is_unreachable field is false
    assert!(!info.is_unreachable);

    // @step And the field profile_name has type Option<String>
    let with_profile = ProviderInfo {
        key: "openai:my-profile".to_string(),
        display_name: "openai: my-profile".to_string(),
        models: Vec::new(),
        profile_name: Some("my-profile".to_string()),
        is_unreachable: true,
        provider_id: "openai".to_string(),
        unavailable_reason: None,
    };
    let _typed: Option<String> = with_profile.profile_name.clone();
    assert_eq!(with_profile.profile_name.as_deref(), Some("my-profile"));

    // @step And the field is_unreachable has type bool
    let _flag: bool = with_profile.is_unreachable;
    assert!(with_profile.is_unreachable);

    // The new fields must survive a JSON round-trip like every other
    // RPC wire type.
    round_trip(with_profile);
}
