//! Feature: spec/features/hitl-wire-protocol-parity-static-checks.feature
//!
//! RPC-410 static checks, split out of rpc410_hitl_wire_protocol_parity.rs
//! by REFAC-052: the send_hitl_response source contains no option-label
//! comparison, and the TS-parity wire shapes round-trip through serde_json.
//! Neither scenario creates a session.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_rpc_types::{
    HitlAnswer as WireHitlAnswer, HitlOption as WireHitlOption, HitlQuestion as WireHitlQuestion,
    HitlRequest as WireHitlRequest, HitlResponse as WireHitlResponse,
};

// ============================================================================
// Scenario: The send path contains no option-label comparison
// ============================================================================

#[test]
fn send_path_contains_no_option_label_comparison() {
    // @step Given the source of handle_impl.rs and the hitl mapping module
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let handle_src = codelet_test_helpers::source_composition::read_tree_or_file(
        manifest.join("src/handle_impl.rs"),
    )
    .expect("handle_impl.rs must exist");
    let mapping_path = manifest.join("src/hitl_mapping.rs");
    let mapping_src = codelet_test_helpers::source_composition::read_tree_or_file(&mapping_path)
        .unwrap_or_else(|e| {
            panic!(
                "sessions/src/hitl_mapping.rs must exist (dossier §3.2/§3.3 \
             extraction): {e}"
            )
        });

    // @step When the send_hitl_response path is inspected
    let fn_start = handle_src
        .find("fn send_hitl_response(")
        .expect("handle_impl.rs must define send_hitl_response");
    let fn_end = handle_src[fn_start..]
        .find("fn get_pause_state(")
        .map(|off| fn_start + off)
        .expect("handle_impl.rs must define get_pause_state after send_hitl_response (the send path's end marker)");
    let body = &handle_src[fn_start..fn_end];
    let send_path = format!("{body}\n{mapping_src}");

    // @step Then it contains no comparison of answer values against option labels
    for needle in ["is_option_label", ".label ==", "label == "] {
        assert!(
            !send_path.contains(needle),
            "the send path must not compare answer values against option \
             labels (found {needle:?}) — the RPC-408 heuristic is deleted"
        );
    }

    // @step And it does not read the pending request to classify answers
    assert!(
        !body.contains("get_hitl_request"),
        "send_hitl_response must not read the pending HITL request to \
         classify answers — the wire payload is authoritative"
    );
}

// ============================================================================
// Scenario: New wire shapes round-trip through serde_json
// ============================================================================

fn round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize");
    let back: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(&back, value, "round-trip must preserve the value exactly");
}

fn wire_option(label: &str) -> WireHitlOption {
    WireHitlOption {
        label: label.to_string(),
        description: format!("{label} description"),
    }
}

#[test]
fn new_wire_shapes_round_trip_through_serde_json() {
    // @step Given a wire HitlRequest with 3 questions including one without options and a wire HitlResponse with cancelled false and mixed answers
    let request = WireHitlRequest {
        questions: vec![
            WireHitlQuestion {
                id: "approach".to_string(),
                header: "Approach".to_string(),
                question: "Which approach do you prefer?".to_string(),
                options: vec![wire_option("Option A"), wire_option("Option B")],
            },
            WireHitlQuestion {
                id: "priority".to_string(),
                header: "Priority".to_string(),
                question: "What is the priority?".to_string(),
                options: vec![wire_option("High"), wire_option("Low")],
            },
            WireHitlQuestion {
                id: "notes".to_string(),
                header: "Notes".to_string(),
                question: "Any additional notes?".to_string(),
                options: vec![],
            },
        ],
    };
    let response = WireHitlResponse {
        cancelled: false,
        answers: vec![
            WireHitlAnswer {
                id: "approach".to_string(),
                selected: vec!["Option A".to_string()],
                other: None,
            },
            WireHitlAnswer {
                id: "notes".to_string(),
                selected: vec![],
                other: Some("free text".to_string()),
            },
        ],
    };

    // @step When each value is serialized to JSON and deserialized back
    // @step Then each deserialized value equals the original
    round_trip(&request);
    round_trip(&response);
    assert_eq!(request.questions.len(), 3);
    assert!(request.questions[2].options.is_empty());
    assert!(!response.cancelled);
    assert_eq!(response.answers.len(), 2);

    // @step And a cancelled wire HitlResponse with empty answers also round-trips equal
    let cancelled = WireHitlResponse {
        cancelled: true,
        answers: vec![],
    };
    round_trip(&cancelled);
    assert!(cancelled.cancelled);
    assert!(cancelled.answers.is_empty());
}
