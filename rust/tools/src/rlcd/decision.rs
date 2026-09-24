//! RLCD-002 — the Decision tool: first-class rig Tool for typed RLCD
//! decisions (choice/score/noul).
//!
//! Feature: spec/features/decision-first-class-rig-tool-for-typed-rlcd-decisions-choice-score-noul.feature
//!
//! Mirrors the DeepSearchTool pattern (HOOK-017): pre-tool hook check,
//! argument validation BEFORE any network I/O (`decision_validate`), then
//! the RLCD-001 supervisor (bounded `ensure_ready`; unreachable => fail-open
//! structured error with a hint, never a hard block), then
//! `RlcdClient::classify`, then a compact render (choice => argmax +
//! confidence + probabilities, score => score + legend, noul => P(true),
//! plus usage).

use std::time::Duration;

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::rlcd::client::{HttpRlcdClient, RlcdClient, RlcdResponse};
use crate::rlcd::config::load_rlcd_config;
use crate::rlcd::decision_validate::validate_questions;
use crate::rlcd::error::RlcdError;
use crate::rlcd::service::RlcdSupervisor;
use crate::ToolError;

/// Bounded foreground wait for the engine to be ready (default).
const DEFAULT_ENSURE_BUDGET: Duration = Duration::from_secs(10);
/// Per-classify-request transport budget (default).
const DEFAULT_REQUEST_BUDGET: Duration = Duration::from_secs(60);

/// Arguments for the Decision tool.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DecisionArgs {
    /// Shared context as a plain string (most-important content FIRST — the
    /// backend tail-truncates at its max_len). Required, non-empty.
    pub state: String,
    /// qid -> question spec (`{type?, instructions?, criteria?}`). Required;
    /// must be a JSON object with 1..=100 entries.
    pub questions: Value,
    /// Model name override. Defaults to the server-reported model from
    /// `/health` (never hardcoded).
    #[serde(default)]
    pub model: Option<String>,
    /// Base-URL override for this call (defaults to the configured URL).
    #[serde(default)]
    pub url: Option<String>,
}

/// The Decision tool — asks the RLCD engine typed decision questions.
#[derive(Debug, Clone)]
pub struct DecisionTool {
    session_id: Uuid,
    ensure_budget: Duration,
    request_budget: Duration,
}

impl DecisionTool {
    /// Create a Decision tool with the default budgets (10s ensure, 60s
    /// request).
    #[must_use]
    pub fn new(session_id: Uuid) -> Self {
        Self::with_budgets(session_id, DEFAULT_ENSURE_BUDGET, DEFAULT_REQUEST_BUDGET)
    }

    /// Create a Decision tool with explicit budgets (test seam: short
    /// budgets keep the fail-open/timeout scenarios fast).
    #[must_use]
    pub fn with_budgets(
        session_id: Uuid,
        ensure_budget: Duration,
        request_budget: Duration,
    ) -> Self {
        Self {
            session_id,
            ensure_budget,
            request_budget,
        }
    }
}

/// Render a classifier response compactly (one line per question + usage).
fn render(resp: &RlcdResponse) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(answers) = resp.answers.as_object() {
        for (qid, ans) in answers {
            match ans.get("type").and_then(Value::as_str) {
                Some("choice") => {
                    let choice = ans.get("choice").and_then(Value::as_str).unwrap_or("?");
                    let conf = ans.get("confidence").and_then(Value::as_f64).unwrap_or(0.0);
                    let probs = ans.get("probabilities").cloned().unwrap_or(Value::Null);
                    lines.push(format!(
                        "{qid} -> {choice} (confidence {}, P{probs})",
                        format_prob(conf)
                    ));
                }
                Some("score") => {
                    let score = ans.get("score").and_then(Value::as_f64).unwrap_or(0.0);
                    let conf = ans.get("confidence").and_then(Value::as_f64).unwrap_or(0.0);
                    let probs = ans.get("probabilities").cloned().unwrap_or(Value::Null);
                    lines.push(format!(
                        "{qid} -> score {} (confidence {}, P{probs})",
                        format_prob(score),
                        format_prob(conf)
                    ));
                    if let Some(legend) = ans.get("legend").and_then(Value::as_object) {
                        for (level, label) in legend {
                            let label = match label {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            lines.push(format!("  level {level}: {label}"));
                        }
                    }
                }
                Some("noul") => {
                    let p = ans.get("noul").and_then(Value::as_f64).unwrap_or(0.0);
                    lines.push(format!("{qid} -> P(true)={}", format_prob(p)));
                }
                _ => lines.push(format!("{qid} -> {ans}")),
            }
        }
    }
    let in_tok = resp.usage.get("input_tokens").and_then(Value::as_u64).unwrap_or(0);
    let out_tok = resp.usage.get("output_tokens").and_then(Value::as_u64).unwrap_or(0);
    lines.push(format!("usage: {in_tok} in / {out_tok} out tokens"));
    lines.join("\n")
}

/// Format a probability/score with at least one decimal (1.0, not 1).
fn format_prob(p: f64) -> String {
    if p.fract() == 0.0 {
        format!("{p}.0")
    } else {
        format!("{p}")
    }
}

/// Structured fail-open error for an unreachable engine (never a hard block).
fn unreachable_message(url: &str, detail: &str) -> String {
    format!(
        "RLCD unreachable at {url}: {detail} — start the backend (e.g. `rlcd serve`) \
         or set rlcd.url in the user config (~/.fspec/fspec-config.json). \
         Fail-open: this call degrades to an error, the caller is not blocked."
    )
}

impl Tool for DecisionTool {
    const NAME: &'static str = "Decision";

    type Error = ToolError;
    type Args = DecisionArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "Decision".to_string(),
            description: concat!(
                "Ask the RLCD decision engine typed questions and get calibrated, ",
                "non-hallucinatory decisions: choice (pick one of 2-50 criteria), ",
                "score (rate on a 2-50 level rubric) or noul (P(true) of a yes/no). ",
                "Supplies shared context (state) plus questions; the model name is ",
                "echoed from the server's /health (never hardcoded). If the engine is ",
                "unreachable the tool fails open with a structured error."
            )
            .to_string(),
            parameters: json!({
                "$schema": "http://json-schema.org/draft-07/schema#",
                "type": "object",
                "properties": {
                    "state": {
                        "type": "string",
                        "minLength": 1,
                        "description": "Shared context as a plain string. Put the most-important content FIRST (the backend tail-truncates long state)."
                    },
                    "questions": {
                        "type": "object",
                        "minProperties": 1,
                        "maxProperties": 100,
                        "additionalProperties": {
                            "type": "object",
                            "properties": {
                                "type": { "type": "string", "enum": ["choice", "score", "noul"], "description": "Question type (default: choice)." },
                                "instructions": { "type": ["string", "null"], "description": "Question prompt text (keep criteria short: <= ~8 words each)." },
                                "criteria": {
                                    "description": "choice: object qid->text with 2-50 entries. score: array of 2-50 rubric labels, lowest first. noul: optional object with only 'true'/'false' keys."
                                }
                            },
                            "required": ["criteria"],
                            "additionalProperties": false
                        },
                        "description": "Question ID -> question spec. IDs become the answer keys."
                    },
                    "model": {
                        "type": ["string", "null"],
                        "description": "Model name override. Defaults to the server-reported model from /health."
                    },
                    "url": {
                        "type": ["string", "null"],
                        "description": "RLCD base URL override for this call (defaults to the configured URL)."
                    }
                },
                "required": ["state", "questions"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 1. pre-tool hook (HOOK-017 pattern)
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            &self.name(),
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "Decision",
                message: reason,
            });
        }

        // 2. argument validation — BEFORE any network I/O
        if args.state.trim().is_empty() {
            return Err(ToolError::Validation {
                tool: "Decision",
                message: "state is required and must not be empty".to_string(),
            });
        }
        let questions = match validate_questions(&args.questions) {
            Ok(q) => q,
            Err(message) => {
                return Err(ToolError::Validation {
                    tool: "Decision",
                    message,
                })
            }
        };

        // 3. ensure the engine is ready (bounded; fail-open on failure)
        let mut config = load_rlcd_config();
        if let Some(url) = &args.url {
            config.url = url.clone();
        }
        let configured_url = config.url.clone();
        let supervisor = RlcdSupervisor::new(config);
        let model = match supervisor.ensure_ready(self.ensure_budget).await {
            Ok(m) => m,
            Err(RlcdError::Unreachable { url, detail }) => {
                return Err(ToolError::Execution {
                    tool: "Decision",
                    message: unreachable_message(&url, &detail),
                })
            }
            Err(e) => {
                return Err(ToolError::Execution {
                    tool: "Decision",
                    message: format!("RLCD unreachable: {e} (fail-open; set rlcd.url to point at a live engine)"),
                })
            }
        };
        let model = args.model.clone().unwrap_or(model);

        // 4. classify
        let base_url = supervisor
            .effective_base_url()
            .unwrap_or(configured_url);
        let client = HttpRlcdClient;
        let response: RlcdResponse = client
            .classify(
                &base_url,
                &model,
                &args.state,
                &questions,
                self.request_budget,
            )
            .await
            .map_err(|e| match e {
                RlcdError::Validation { detail } => ToolError::Validation {
                    tool: "Decision",
                    message: detail,
                },
                RlcdError::Busy { retry_after } => ToolError::Execution {
                    tool: "Decision",
                    message: format!("RLCD queue busy, retry in {retry_after}s"),
                },
                RlcdError::Server { detail } => ToolError::Execution {
                    tool: "Decision",
                    message: format!("RLCD server failure: {detail}"),
                },
                RlcdError::Unreachable { url, detail } => ToolError::Execution {
                    tool: "Decision",
                    message: unreachable_message(&url, &detail),
                },
                other => ToolError::Execution {
                    tool: "Decision",
                    message: other.to_string(),
                },
            })?;

        // 5. render compactly
        Ok(render(&response))
    }
}
