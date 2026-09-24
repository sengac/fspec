//! RLCD-002 — Decision arg validation (pure, no I/O).
//!
//! Feature: spec/features/decision-first-class-rig-tool-for-typed-rlcd-decisions-choice-score-noul.feature
//!
//! Validates the raw `questions` value into typed `RlcdQuestion`s BEFORE any
//! network I/O: 1..=100 questions, per-type criteria bounds (choice/score:
//! 2..=50; noul: optional {true,false}), unknown fields rejected.

use serde_json::Value;

use crate::rlcd::client::RlcdQuestion;

/// Hard cap on questions per request (the protocol's default branch limit).
pub(crate) const MAX_QUESTIONS: usize = 100;
/// Criteria bound for choice/score questions.
pub(crate) const CRITERIA_BOUNDS: (usize, usize) = (2, 50);

/// Validate the raw `questions` value into typed questions.
///
/// Returns an error string naming the offending field (the tool wraps it in
/// a `ToolError::Validation`).
pub fn validate_questions(questions: &Value) -> Result<Vec<RlcdQuestion>, String> {
    let Some(map) = questions.as_object() else {
        return Err("questions must be a JSON object mapping qid -> question spec".to_string());
    };
    if map.is_empty() {
        return Err(format!(
            "questions must contain 1 to {MAX_QUESTIONS} questions (got 0)"
        ));
    }
    if map.len() > MAX_QUESTIONS {
        return Err(format!(
            "questions must contain 1 to {MAX_QUESTIONS} questions (got {})",
            map.len()
        ));
    }
    let mut out = Vec::with_capacity(map.len());
    for (qid, spec) in map {
        let Some(obj) = spec.as_object() else {
            return Err(format!(
                "questions.{qid} must be an object with a 'type' field (choice, score or noul)"
            ));
        };
        for key in obj.keys() {
            if !matches!(key.as_str(), "type" | "instructions" | "criteria") {
                return Err(format!("questions.{qid} has unknown field '{key}'"));
            }
        }
        let qtype = obj
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("choice")
            .to_string();
        if !matches!(qtype.as_str(), "choice" | "score" | "noul") {
            return Err(format!(
                "questions.{qid} type must be one of choice, score, noul (got {qtype:?})"
            ));
        }
        let instructions = match obj.get("instructions") {
            Some(Value::String(s)) => Some(s.clone()),
            None | Some(Value::Null) => None,
            Some(other) => {
                return Err(format!(
                    "questions.{qid} instructions must be a string or null (got {other:?})"
                ))
            }
        };
        let criteria = obj.get("criteria").cloned();
        match qtype.as_str() {
            "choice" => {
                let Some(map) = criteria.as_ref().and_then(Value::as_object) else {
                    return Err(format!(
                        "questions.{qid} criteria must be an object with 2 to {} entries (choice)",
                        CRITERIA_BOUNDS.1
                    ));
                };
                check_criteria_count(map.len())?;
            }
            "score" => {
                let Some(arr) = criteria.as_ref().and_then(Value::as_array) else {
                    return Err(format!(
                        "questions.{qid} criteria must be an array with 2 to {} entries in rubric order (score)",
                        CRITERIA_BOUNDS.1
                    ));
                };
                check_criteria_count(arr.len())?;
            }
            "noul" => {
                if let Some(obj) = criteria.as_ref().and_then(Value::as_object) {
                    for key in obj.keys() {
                        if !matches!(key.as_str(), "true" | "false") {
                            return Err(format!(
                                "questions.{qid} criteria (noul) may only contain the keys 'true'/'false' (got {key:?})"
                            ));
                        }
                    }
                }
            }
            _ => unreachable!("qtype validated above"),
        }
        out.push(RlcdQuestion {
            qid: qid.clone(),
            r#type: qtype,
            instructions,
            criteria,
        });
    }
    Ok(out)
}

/// Enforce the 2..=50 criteria bound (names the field for the caller).
fn check_criteria_count(count: usize) -> Result<(), String> {
    if count < CRITERIA_BOUNDS.0 || count > CRITERIA_BOUNDS.1 {
        return Err(format!(
            "criteria must have 2 to {} entries (got {count})",
            CRITERIA_BOUNDS.1
        ));
    }
    Ok(())
}
