//! `generate-example-mapping-from-event-storm` — Rust port of
//! `src/commands/generate-example-mapping-from-event-storm.ts` (RPC-232).
//!
//! Transforms a work unit's Event Storm artifacts into Example Mapping
//! entries, then persists the mutated work unit atomically:
//!
//! * policy (with `when` + `then`) → rule
//!   `"System must <then-sentence> after <when-sentence>"` (via
//!   [`pascal_case_to_sentence`]).
//! * event → NOTHING. Per **BUG-089** auto-generated examples were generic
//!   and unhelpful, so `examplesAdded` is always `0` and the `examples`
//!   array is left for humans to populate.
//! * hotspot (with `concern`) → question `"@human: <concern>?"`. Per
//!   **BUG-088** the concern text is preserved verbatim (trimmed) and a
//!   trailing `?` is appended ONLY when absent.
//!
//! Soft-deleted Event Storm items (`deleted: true`) are skipped.
//!
//! ## Missing-file behaviour — Option B (INLINE existsSync + read, NO auto-create)
//!
//! The TS implementation checks `existsSync(workUnitsFile)` and returns
//! `spec/work-units.json not found. Run fspec init first.` WITHOUT creating
//! the file (`src/commands/generate-example-mapping-from-event-storm.ts:41-46`).
//! We inline the existence check, read, and parse rather than using
//! [`crate::io::ensure::ensure_work_units_file`] (which auto-creates).
//!
//! ## Inlined helper (per port decisions)
//!
//! [`pascal_case_to_sentence`] — private analog of the TS
//! `pascalCaseToSentence` (`src/utils/text-formatting.ts`).
//!
//! ## Two-front-doors
//!
//! Both the LLM-facing dispatcher AND the standalone Rust binary's clap
//! subcommand call this single function (RPC-003 §7/§11). The CLI bridge at
//! `codelet/fspec/src/generate_example_mapping_from_event_storm.rs` is JSON
//! marshalling + stdout/stderr rendering only — no domain logic.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::FspecCoreError;
use crate::io::locked_file::write_json_atomic;
use crate::io::time::iso8601_now;
use crate::types::work_unit::WorkUnitsData;

/// CLI arguments accepted by `generate-example-mapping-from-event-storm`.
/// Mirrors the TS `GenerateExampleMappingOptions` interface.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateExampleMappingArgs {
    work_unit_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateExampleMappingResult {
    success: bool,
    rules_added: usize,
    examples_added: usize,
    questions_added: usize,
}

/// Convert PascalCase/camelCase to a lowercase space-separated sentence.
/// Private analog of the TS `pascalCaseToSentence`
/// (`src/utils/text-formatting.ts`): insert a space before every ASCII
/// uppercase letter, trim, lowercase.
fn pascal_case_to_sentence(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        if ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out.trim().to_lowercase()
}

/// Dispatcher entry point.
pub async fn run(args_json: &str, project_root: &Path) -> Result<String, FspecCoreError> {
    let args: GenerateExampleMappingArgs =
        serde_json::from_str(args_json).map_err(|e| FspecCoreError::InvalidArgs {
            command: "generate-example-mapping-from-event-storm",
            reason: format!("failed to parse args: {e}"),
        })?;

    let path = project_root.join("spec").join("work-units.json");

    // Option B — INLINE existsSync check, NO auto-create. Mirrors TS
    // `src/commands/generate-example-mapping-from-event-storm.ts:41-46`.
    if !path.exists() {
        return Err(FspecCoreError::InvalidArgs {
            command: "generate-example-mapping-from-event-storm",
            reason: "spec/work-units.json not found. Run fspec init first.".to_string(),
        });
    }

    // Read + parse (escalate malformed JSON, parity with TS readJSON throw).
    let raw = std::fs::read_to_string(&path).map_err(|source| FspecCoreError::Io {
        command: "generate-example-mapping-from-event-storm",
        source,
    })?;
    let mut data: WorkUnitsData =
        serde_json::from_str(&raw).map_err(|e| FspecCoreError::ParseJson {
            file: "work-units.json".to_string(),
            reason: crate::io::json_error::parse_json_reason(&raw, &e),
        })?;

    // Validate work unit exists (mirrors TS:60-63).
    let wu = match data.work_units.get_mut(&args.work_unit_id) {
        Some(wu) => wu,
        None => {
            return Err(FspecCoreError::InvalidArgs {
                command: "generate-example-mapping-from-event-storm",
                reason: format!("Work unit {} not found", args.work_unit_id),
            });
        }
    };

    // Validate Event Storm exists with an `items` array (mirrors TS:66-70:
    // `!workUnit.eventStorm || !workUnit.eventStorm.items`).
    let items: Vec<Value> = match wu.extra.get("eventStorm") {
        Some(Value::Object(es)) => match es.get("items") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => {
                return Err(FspecCoreError::InvalidArgs {
                    command: "generate-example-mapping-from-event-storm",
                    reason: format!("Work unit {} has no Event Storm data", args.work_unit_id),
                });
            }
        },
        _ => {
            return Err(FspecCoreError::InvalidArgs {
                command: "generate-example-mapping-from-event-storm",
                reason: format!("Work unit {} has no Event Storm data", args.work_unit_id),
            });
        }
    };

    // Initialize counters (backward compat: undefined → 0). Mirrors TS:80-89.
    let mut next_rule_id = wu
        .extra
        .get("nextRuleId")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    // TS initializes `nextExampleId` to 0 when undefined even though no
    // examples are derived (BUG-089). The key must therefore be present in
    // the persisted JSON for byte-parity (TS:83-85).
    let next_example_id = wu
        .extra
        .get("nextExampleId")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let mut next_question_id = wu
        .extra
        .get("nextQuestionId")
        .and_then(Value::as_u64)
        .unwrap_or(0);

    let mut new_rules: Vec<Value> = Vec::new();
    let mut new_questions: Vec<Value> = Vec::new();
    let mut rules_added = 0usize;
    let examples_added = 0usize; // BUG-089: never derive examples from events.
    let mut questions_added = 0usize;

    // Process Event Storm items (mirrors TS:92-153).
    for item in &items {
        // Skip soft-deleted items.
        if matches!(item.get("deleted"), Some(Value::Bool(true))) {
            continue;
        }

        let item_type = item.get("type").and_then(Value::as_str);

        // Derive rules from policies (with when + then).
        if item_type == Some("policy") {
            let when = item.get("when").and_then(Value::as_str);
            let then = item.get("then").and_then(Value::as_str);
            if let (Some(when), Some(then)) = (when, then) {
                let when_text = pascal_case_to_sentence(when);
                let then_text = pascal_case_to_sentence(then);
                let rule_text = format!("System must {then_text} after {when_text}");
                let now = iso8601_now();
                let mut rule_obj = Map::new();
                rule_obj.insert("id".to_string(), Value::from(next_rule_id));
                rule_obj.insert("text".to_string(), Value::String(rule_text));
                rule_obj.insert("deleted".to_string(), Value::Bool(false));
                rule_obj.insert("createdAt".to_string(), Value::String(now));
                new_rules.push(Value::Object(rule_obj));
                next_rule_id += 1;
                rules_added += 1;
            }
        }

        // Derive questions from hotspots (with concern).
        if item_type == Some("hotspot") {
            if let Some(concern) = item.get("concern").and_then(Value::as_str) {
                // BUG-088: preserve concern verbatim (trimmed), append `?`
                // only when absent.
                let mut concern_text = concern.trim().to_string();
                if !concern_text.ends_with('?') {
                    concern_text.push('?');
                }
                let question_text = format!("@human: {concern_text}");
                let now = iso8601_now();
                // TS pushes `answer: undefined` which JSON.stringify drops,
                // so the on-disk shape omits the `answer` key.
                let mut q_obj = Map::new();
                q_obj.insert("id".to_string(), Value::from(next_question_id));
                q_obj.insert("text".to_string(), Value::String(question_text));
                q_obj.insert("deleted".to_string(), Value::Bool(false));
                q_obj.insert("createdAt".to_string(), Value::String(now));
                new_questions.push(Value::Object(q_obj));
                next_question_id += 1;
                questions_added += 1;
            }
        }
    }

    let now = iso8601_now();

    // Ensure rules/examples/questions arrays exist (mirrors TS:73-78), then
    // append. `examples` is initialised but never populated (BUG-089).
    for field in ["rules", "examples", "questions"] {
        let entry = wu
            .extra
            .entry(field.to_string())
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            *entry = Value::Array(Vec::new());
        }
    }

    if let Some(Value::Array(rules)) = wu.extra.get_mut("rules") {
        rules.extend(new_rules);
    }
    if let Some(Value::Array(questions)) = wu.extra.get_mut("questions") {
        questions.extend(new_questions);
    }

    // Persist updated counters. `nextExampleId` is written verbatim (TS keeps
    // the initialized 0 even though no examples are derived — BUG-089).
    wu.extra
        .insert("nextRuleId".to_string(), Value::from(next_rule_id));
    wu.extra
        .insert("nextExampleId".to_string(), Value::from(next_example_id));
    wu.extra
        .insert("nextQuestionId".to_string(), Value::from(next_question_id));

    // Bump work-unit timestamp (mirrors TS:156).
    wu.updated_at = now.clone();

    // Bump meta.lastUpdated only when meta exists (mirrors TS:159-161).
    if let Some(meta) = &mut data.meta {
        meta.last_updated = now;
    }

    // Single atomic write.
    write_json_atomic(&path, &data)?;

    let result = GenerateExampleMappingResult {
        success: true,
        rules_added,
        examples_added,
        questions_added,
    };
    serde_json::to_string(&result).map_err(|e| FspecCoreError::InvalidArgs {
        command: "generate-example-mapping-from-event-storm",
        reason: format!("failed to serialize result: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;

    #[test]
    fn args_parse_camel_case() {
        let a: GenerateExampleMappingArgs =
            serde_json::from_str(r#"{"workUnitId":"AUTH-001"}"#).unwrap();
        assert_eq!(a.work_unit_id, "AUTH-001");
    }

    #[test]
    fn pascal_case_to_sentence_basic() {
        assert_eq!(pascal_case_to_sentence("UserRegistered"), "user registered");
        assert_eq!(
            pascal_case_to_sentence("SendWelcomeEmail"),
            "send welcome email"
        );
        assert_eq!(pascal_case_to_sentence("userLoggedIn"), "user logged in");
    }

    #[test]
    fn result_serializes_camel_case() {
        let r = GenerateExampleMappingResult {
            success: true,
            rules_added: 2,
            examples_added: 0,
            questions_added: 1,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"rulesAdded\":2"));
        assert!(s.contains("\"examplesAdded\":0"));
        assert!(s.contains("\"questionsAdded\":1"));
    }
}
