//! `answer-question` — Rust port of `src/commands/answer-question.ts` (RPC-196).
//!
//! Marks a question on a work unit as `selected`, optionally records the
//! supplied answer, and optionally promotes the answer into either the
//! `rules` array (as a fresh [`RuleItem`]-shaped record using the
//! work unit's `nextRuleId` counter) or the `assumptions` array (as a
//! raw string). Mirrors the TS implementation at
//! `src/commands/answer-question.ts:24-124` exactly.
//!
//! ## Validation (in order)
//!
//! 1. Work unit exists.
//! 2. Work unit status is `specifying` (Example Mapping phase only).
//! 3. Work unit has a non-empty `questions` array.
//! 4. `index` is within `[0, questions.length)`.
//! 5. `questions[index]` is a JSON object (NOT a legacy raw string).
//!
//! Any failure aborts BEFORE writing — disk state is byte-equal to the
//! pre-call contents.
//!
//! ## Persistence
//!
//! Single `ensure_work_units_file` load + single `write_json_atomic` write.
//! All field-order preservation is handled by the shared `WorkUnit`
//! serializer (see `crate::types::work_unit`).

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

// ─────────────────────────────────────────────────────────────────────────
// Args
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct AnswerQuestionArgs {
    work_unit_id: String,
    index: i64,
    answer: Option<String>,
    add_to: Option<String>,
}

#[derive(Debug, Serialize)]
struct AnswerQuestionResult {
    success: bool,
    question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "addedTo")]
    added_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "addedContent")]
    added_content: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────

pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AnswerQuestionArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "answer-question",
            reason: format!("failed to parse args: {e}"),
        })?;

    let mut data = ensure_work_units_file(project_root)?;

    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "answer-question",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status.as_str())
        .ok_or_else(|| FspecCoreError::InvalidArgs {
            command: "answer-question",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        })?;
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "answer-question",
            reason: format!(
                "Can only answer questions during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    // Validate questions array exists and is non-empty.
    let questions_len = {
        let wu =
            data.work_units
                .get(&args.work_unit_id)
                .ok_or_else(|| FspecCoreError::InvalidArgs {
                    command: "answer-question",
                    reason: format!("Work unit '{}' does not exist", args.work_unit_id),
                })?;
        match wu.extra.get("questions") {
            Some(Value::Array(arr)) if !arr.is_empty() => arr.len(),
            _ => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "answer-question",
                    reason: format!("Work unit {} has no questions", args.work_unit_id),
                });
            }
        }
    };

    if args.index < 0 || (args.index as usize) >= questions_len {
        return Err(FspecCoreError::InvalidArgs {
            command: "answer-question",
            reason: format!(
                "Invalid question index {}. Valid range: 0-{}",
                args.index,
                questions_len - 1
            ),
        });
    }

    // Pull out the question text and validate the entry shape.
    let idx = args.index as usize;
    let question_text = {
        let wu =
            data.work_units
                .get(&args.work_unit_id)
                .ok_or_else(|| FspecCoreError::InvalidArgs {
                    command: "answer-question",
                    reason: format!("Work unit '{}' does not exist", args.work_unit_id),
                })?;
        let arr = match wu.extra.get("questions") {
            Some(Value::Array(a)) => a,
            _ => unreachable!("questions presence already validated"),
        };
        let entry = &arr[idx];
        match entry {
            Value::Object(obj) => match obj.get("text") {
                Some(Value::String(s)) => s.clone(),
                _ => {
                    return Err(FspecCoreError::InvalidArgs {
                        command: "answer-question",
                        reason: "Question format is invalid. Expected QuestionItem object."
                            .to_string(),
                    });
                }
            },
            _ => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "answer-question",
                    reason: "Question format is invalid. Expected QuestionItem object.".to_string(),
                });
            }
        }
    };

    let now = iso8601_now();

    // Mutate the question and (optionally) append to rules/assumptions.
    let mut added_to: Option<String> = None;
    let mut added_content: Option<String> = None;

    let wu =
        data.work_units
            .get_mut(&args.work_unit_id)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "answer-question",
                reason: format!("Work unit '{}' does not exist", args.work_unit_id),
            })?;

    // Mark the question selected and (optionally) record the answer.
    {
        let arr = wu
            .extra
            .get_mut("questions")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| FspecCoreError::InvalidArgs {
                command: "answer-question",
                reason: format!("Work unit {} has no questions", args.work_unit_id),
            })?;
        let entry = &mut arr[idx];
        if let Value::Object(obj) = entry {
            obj.insert("selected".to_string(), Value::Bool(true));
            if let Some(ans) = args.answer.as_ref() {
                obj.insert("answered".to_string(), Value::Bool(true));
                obj.insert("answer".to_string(), Value::String(ans.clone()));
            }
        }
    }

    // Optional promotion: rule | rules → push RuleItem to `rules` and
    // bump `nextRuleId`. assumption | assumptions → push raw string to
    // `assumptions`. Anything else (incl. `none` and missing) skips.
    if let (Some(ans), Some(add_to_raw)) = (args.answer.as_ref(), args.add_to.as_ref()) {
        let add_to_norm = add_to_raw.as_str();
        if add_to_norm != "none" {
            match add_to_norm {
                "rule" | "rules" => {
                    let next_id = wu
                        .extra
                        .get("nextRuleId")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let mut rule_obj = Map::new();
                    rule_obj.insert("id".to_string(), Value::from(next_id));
                    rule_obj.insert("text".to_string(), Value::String(ans.clone()));
                    rule_obj.insert("deleted".to_string(), Value::Bool(false));
                    rule_obj.insert("createdAt".to_string(), Value::String(now.clone()));

                    let rules_entry = wu
                        .extra
                        .entry("rules".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if !rules_entry.is_array() {
                        *rules_entry = Value::Array(Vec::new());
                    }
                    if let Value::Array(arr) = rules_entry {
                        arr.push(Value::Object(rule_obj));
                    }
                    wu.extra
                        .insert("nextRuleId".to_string(), Value::from(next_id + 1));
                    added_to = Some("rules".to_string());
                    added_content = Some(ans.clone());
                }
                "assumption" | "assumptions" => {
                    let assumes_entry = wu
                        .extra
                        .entry("assumptions".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if !assumes_entry.is_array() {
                        *assumes_entry = Value::Array(Vec::new());
                    }
                    if let Value::Array(arr) = assumes_entry {
                        arr.push(Value::String(ans.clone()));
                    }
                    added_to = Some("assumptions".to_string());
                    added_content = Some(ans.clone());
                }
                _ => {}
            }
        }
    }

    wu.updated_at = now;

    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let result = AnswerQuestionResult {
        success: true,
        question: question_text,
        added_to,
        added_content,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "answer-question",
        reason: format!("failed to serialize result: {e}"),
    })
}
