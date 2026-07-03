//! HITL wire↔internal mapping (RPC-410).
//!
//! Feature: spec/features/hitl-wire-protocol-parity.feature
//!
//! Pure pass-through conversions between the TS-parity wire shapes in
//! `codelet_rpc_types` and the internal tool types in
//! `codelet_tools::request_user_input`. NO inference: answer
//! classification (selected vs other) comes verbatim from the wire
//! payload — the RPC-408 option-label heuristic is deleted.

use std::collections::HashMap;

use codelet_rpc_types::{HitlAnswer, HitlOption, HitlQuestion, HitlRequest};
use codelet_tools::request_user_input as internal;

/// Map the internal HITL request to the wire shape, converting EVERY
/// question (full multi-question surface — no first-question slicing).
/// An internal question with `options: None` surfaces `options: []`.
pub fn internal_request_to_wire(request: internal::HitlRequest) -> HitlRequest {
    HitlRequest {
        questions: request
            .questions
            .into_iter()
            .map(|q| HitlQuestion {
                id: q.id,
                header: q.header,
                question: q.question,
                options: q
                    .options
                    .unwrap_or_default()
                    .into_iter()
                    .map(|o| HitlOption {
                        label: o.label,
                        description: o.description,
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Map the wire response to the internal tool response — direct
/// pass-through:
/// - `cancelled: true` → `Cancelled { cancelled: true }`.
/// - else → `Answered` with the answers vec keyed by each answer's id,
///   preserving `selected`/`other` EXACTLY.
pub fn wire_response_to_internal(
    cancelled: bool,
    answers: Vec<HitlAnswer>,
) -> internal::HitlResponse {
    if cancelled {
        return internal::HitlResponse::Cancelled { cancelled: true };
    }
    let answers: HashMap<String, internal::HitlAnswer> = answers
        .into_iter()
        .map(|a| {
            (
                a.id,
                internal::HitlAnswer {
                    selected: a.selected,
                    other: a.other,
                },
            )
        })
        .collect();
    internal::HitlResponse::Answered { answers }
}
