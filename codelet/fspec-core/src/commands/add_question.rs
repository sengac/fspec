//! `add-question` — Rust port of `src/commands/add-question.ts` (RPC-188).
//!
//! Appends a `QuestionItem`-shaped record to a work unit's `questions`
//! array during the specifying phase of Example Mapping. The work unit
//! must exist and be in `specifying` status; otherwise the dispatcher
//! returns a canonical validation error and disk state is left
//! untouched.
//!
//! Reuses existing shared infrastructure:
//! * [`crate::io::ensure::ensure_work_units_file`] — auto-create + load
//!   `spec/work-units.json` (parity with TS `ensureWorkUnitsFile`).
//! * [`crate::io::locked_file::write_json_atomic`] — single atomic write
//!   at the end (the TS implementation uses `fileManager.transaction`).
//! * [`crate::io::time::iso8601_now`] — millisecond-precision ISO-8601
//!   timestamps (parity with TS `new Date().toISOString()`).
//!
//! ## On-disk shape
//!
//! Per the TS `QuestionItem` interface (`src/types/index.ts`), each
//! question is:
//!
//! ```json
//! {
//!   "id": 0,
//!   "text": "@human: Support OAuth?",
//!   "deleted": false,
//!   "createdAt": "2026-06-11T12:00:00.000Z",
//!   "selected": false
//! }
//! ```
//!
//! The `questions` array and the `nextQuestionId` counter both live in
//! the work unit's `extra` map (round-tripped via `#[serde(flatten)]`
//! on [`crate::types::work_unit::WorkUnit`]).
//!
//! ## Mention extraction
//!
//! TS uses `/@\w+/g` where `\w` is `[A-Za-z0-9_]` (ASCII semantics in
//! non-`u` flag mode). The Rust port performs a hand-rolled scan
//! collecting all sequences `@[A-Za-z0-9_]+` from the question text and
//! stripping the leading `@`. An empty list is omitted from the JSON
//! result (matches `…&& { mentionedPeople }` spread).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge
//! at `codelet/fspec/src/add_question.rs` is JSON marshalling only — no
//! domain logic.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::ensure::ensure_work_units_file;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;

/// CLI arguments accepted by `add-question`. Mirrors the TS
/// `AddQuestionOptions` interface at `src/commands/add-question.ts:9-13`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddQuestionArgs {
    work_unit_id: String,
    question: String,
}

#[derive(Debug, Serialize)]
struct AddQuestionResult {
    success: bool,
    #[serde(rename = "questionCount")]
    question_count: usize,
    #[serde(rename = "mentionedPeople", skip_serializing_if = "Option::is_none")]
    mentioned_people: Option<Vec<String>>,
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: AddQuestionArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "add-question",
            reason: format!("failed to parse args: {e}"),
        })?;

    // Load (auto-create on first run).
    let mut data = ensure_work_units_file(project_root)?;

    // Validate work unit exists.
    if !data.work_units.contains_key(&args.work_unit_id) {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-question",
            reason: format!("Work unit '{}' does not exist", args.work_unit_id),
        });
    }

    // Validate work unit is in specifying state.
    let status_str = data
        .work_units
        .get(&args.work_unit_id)
        .map(|w| w.status.as_str())
        .expect("work unit exists");
    if status_str != "specifying" {
        return Err(FspecCoreError::InvalidArgs {
            command: "add-question",
            reason: format!(
                "Can only add questions during discovery/specification phase. {} is in '{}' state.",
                args.work_unit_id, status_str
            ),
        });
    }

    let now = iso8601_now();

    // Extract @mentions BEFORE mutating the data store so we have a
    // single immutable borrow on `args.question`.
    let mentioned_people = extract_mentions(&args.question);

    // Mutate: ensure `questions` and `nextQuestionId` exist on the
    // WorkUnit's extra map, then post-increment the counter and push the
    // new question.
    let wu = data
        .work_units
        .get_mut(&args.work_unit_id)
        .expect("work unit exists");

    let next_id = wu
        .extra
        .get("nextQuestionId")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Build the QuestionItem with explicit field declaration order
    // (id, text, deleted, createdAt, selected) so on-disk JSON matches
    // the TS object-literal insertion order at
    // `src/commands/add-question.ts:55-61`.
    let mut q = Map::new();
    q.insert("id".to_string(), Value::from(next_id));
    q.insert("text".to_string(), Value::String(args.question.clone()));
    q.insert("deleted".to_string(), Value::Bool(false));
    q.insert("createdAt".to_string(), Value::String(now.clone()));
    q.insert("selected".to_string(), Value::Bool(false));

    let questions_entry = wu
        .extra
        .entry("questions".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    if !questions_entry.is_array() {
        *questions_entry = Value::Array(Vec::new());
    }
    let q_count = if let Value::Array(arr) = questions_entry {
        arr.push(Value::Object(q));
        arr.len()
    } else {
        0
    };

    // Post-increment nextQuestionId.
    wu.extra
        .insert("nextQuestionId".to_string(), Value::from(next_id + 1));

    // Bump updatedAt.
    wu.updated_at = now;

    // Single atomic write.
    let path = project_root.join("spec").join("work-units.json");
    write_json_atomic(&path, &data)?;

    let result = AddQuestionResult {
        success: true,
        question_count: q_count,
        mentioned_people: if mentioned_people.is_empty() {
            None
        } else {
            Some(mentioned_people)
        },
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "add-question",
        reason: format!("failed to serialize result: {e}"),
    })
}

/// Hand-rolled scan extracting `@[A-Za-z0-9_]+` matches from `text`,
/// stripping the leading `@`. Mirrors JS `text.match(/@\w+/g) || []` in
/// non-`u` flag mode. The returned `Vec` preserves source order and may
/// contain duplicates — TS does NOT deduplicate.
fn extract_mentions(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() {
                let c = bytes[end];
                let is_word = c.is_ascii_alphanumeric() || c == b'_';
                if !is_word {
                    break;
                }
                end += 1;
            }
            if end > start {
                // Safe: ASCII slice across word chars.
                out.push(std::str::from_utf8(&bytes[start..end]).unwrap_or("").to_string());
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::useless_vec)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: AddQuestionArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001","question":"q?"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
        assert_eq!(a.question, "q?");
    }

    #[test]
    fn args_parse_fails_without_work_unit_id() {
        let err = serde_json::from_str::<AddQuestionArgs>(r#"{"question":"q"}"#).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("workunitid"),
            "missing-field error must mention workUnitId; got: {msg}"
        );
    }

    #[test]
    fn extract_mentions_finds_single_at_mention() {
        let v = extract_mentions("@human: Should we support OAuth?");
        assert_eq!(v, vec!["human"]);
    }

    #[test]
    fn extract_mentions_returns_empty_when_no_at_present() {
        let v = extract_mentions("Should we add caching?");
        assert!(v.is_empty());
    }

    #[test]
    fn extract_mentions_finds_multiple_mentions_in_order() {
        let v = extract_mentions("@alice and @bob disagree with @carol");
        assert_eq!(v, vec!["alice", "bob", "carol"]);
    }

    #[test]
    fn extract_mentions_ignores_lone_at_with_no_word_chars() {
        let v = extract_mentions("Email me @ alice@example.com");
        // The first `@` (followed by space) is skipped; the second `@`
        // captures `example` per `\w+`.
        assert_eq!(v, vec!["example"]);
    }

    #[test]
    fn extract_mentions_accepts_digits_and_underscores() {
        let v = extract_mentions("@user_1 vs @user2");
        assert_eq!(v, vec!["user_1", "user2"]);
    }

    #[test]
    fn result_omits_mentioned_people_when_none() {
        let r = AddQuestionResult {
            success: true,
            question_count: 1,
            mentioned_people: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(!s.contains("mentionedPeople"), "got: {s}");
    }

    #[test]
    fn result_includes_mentioned_people_when_present() {
        let r = AddQuestionResult {
            success: true,
            question_count: 1,
            mentioned_people: Some(vec!["human".to_string()]),
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"mentionedPeople\""), "got: {s}");
        assert!(s.contains("\"human\""), "got: {s}");
    }
}
