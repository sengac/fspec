//! Learnings Context Injection
//!
//! Builds formatted learnings context for injection into session system prompts.
//! Queries the Learnings graph for relevant decisions, failed explorations, and
//! conventions, then formats them as structured text suitable for a system reminder.
//!
//! Called by DeepSearch to inject learnings context into sub-agent prompts,
//! and available for future session-start injection and subordinate spawning.
//! Designed to be non-blocking — returns None if the graph is unavailable.

use serde_json::Value;
use tracing::{info, warn};

use super::database::GraphDatabase;
use super::dispatch_helpers::{matches_fields, LEARNINGS_SEARCHABLE_FIELDS};
use super::learnings_dispatch::LEARNINGS_QUERIES;

/// Maximum approximate token count for injected context.
/// Roughly 4 chars per token, so 2000 tokens ≈ 8000 chars.
const MAX_CONTEXT_CHARS: usize = 8000;

/// Build learnings context using the global registry.
///
/// Returns `None` if the Learnings graph is not initialized or has no
/// relevant data. Called from DeepSearch to inject context into sub-agent
/// system prompts. Also available for session-start and subordinate injection.
pub async fn build_learnings_context(query: &str) -> Option<String> {
    if !super::registry::is_graph_initialized(super::registry::LEARNINGS_GRAPH) {
        return None;
    }

    let db = match super::registry::get_graph(super::registry::LEARNINGS_GRAPH).await {
        Ok(db) => db,
        Err(e) => {
            warn!("Failed to get learnings graph for context injection: {e}");
            return None;
        }
    };

    build_learnings_context_from_db(&db, query).await
}

/// Build learnings context from a specific database instance.
///
/// Separated from `build_learnings_context` for testability — tests can pass
/// a temporary database directly without needing the global registry.
pub async fn build_learnings_context_from_db(
    db: &GraphDatabase,
    query: &str,
) -> Option<String> {
    let query_lower = query.to_lowercase();

    let decisions = collect_relevant_decisions(db, &query_lower).await;
    let warnings = collect_failed_explorations(db, &query_lower).await;
    let learnings = collect_relevant_learnings(db, &query_lower).await;

    if decisions.is_empty() && warnings.is_empty() && learnings.is_empty() {
        return None;
    }

    let formatted = format_context(&decisions, &warnings, &learnings);

    info!(
        decisions = decisions.len(),
        warnings = warnings.len(),
        learnings = learnings.len(),
        chars = formatted.len(),
        "built learnings context for injection"
    );

    Some(formatted)
}

/// Collect Decision nodes relevant to the query.
async fn collect_relevant_decisions(db: &GraphDatabase, query: &str) -> Vec<Value> {
    match db
        .query_with_source(LEARNINGS_QUERIES, "all_decisions", None)
        .await
    {
        Ok(Value::Array(items)) => items
            .into_iter()
            .filter(|item| matches_fields(item, query, LEARNINGS_SEARCHABLE_FIELDS))
            .take(10)
            .collect(),
        _ => Vec::new(),
    }
}

/// Collect Exploration nodes with failed/abandoned outcomes.
async fn collect_failed_explorations(db: &GraphDatabase, query: &str) -> Vec<Value> {
    match db
        .query_with_source(LEARNINGS_QUERIES, "all_explorations", None)
        .await
    {
        Ok(Value::Array(items)) => items
            .into_iter()
            .filter(|item| {
                let outcome_matches = item
                    .get("outcome")
                    .and_then(|v| v.as_str())
                    .map(|o| o == "failure" || o == "abandoned")
                    .unwrap_or(false);
                outcome_matches && matches_fields(item, query, LEARNINGS_SEARCHABLE_FIELDS)
            })
            .take(10)
            .collect(),
        _ => Vec::new(),
    }
}

/// Collect Learning nodes (conventions, patterns, constraints) relevant to query.
async fn collect_relevant_learnings(db: &GraphDatabase, query: &str) -> Vec<Value> {
    match db
        .query_with_source(LEARNINGS_QUERIES, "all_learnings", None)
        .await
    {
        Ok(Value::Array(items)) => items
            .into_iter()
            .filter(|item| matches_fields(item, query, LEARNINGS_SEARCHABLE_FIELDS))
            .take(15)
            .collect(),
        _ => Vec::new(),
    }
}

/// Format collected learnings into a structured context string.
///
/// Output is capped at `MAX_CONTEXT_CHARS` to avoid consuming too much
/// of the LLM context window.
fn format_context(
    decisions: &[Value],
    warnings: &[Value],
    learnings: &[Value],
) -> String {
    let mut output = String::with_capacity(MAX_CONTEXT_CHARS);

    output.push_str("# Learnings Context\n\n");

    // Warnings (failed explorations) come first — most important to surface
    if !warnings.is_empty() {
        output.push_str("## ⚠ Failed Approaches — Do NOT Repeat\n\n");
        for w in warnings {
            if output.len() >= MAX_CONTEXT_CHARS {
                break;
            }
            append_exploration(&mut output, w);
        }
        output.push('\n');
    }

    // Active decisions
    if !decisions.is_empty() {
        output.push_str("## Active Decisions\n\n");
        for d in decisions {
            if output.len() >= MAX_CONTEXT_CHARS {
                break;
            }
            append_decision(&mut output, d);
        }
        output.push('\n');
    }

    // General learnings (conventions, patterns, constraints)
    if !learnings.is_empty() {
        output.push_str("## Relevant Knowledge\n\n");
        for l in learnings {
            if output.len() >= MAX_CONTEXT_CHARS {
                break;
            }
            append_learning(&mut output, l);
        }
    }

    // Hard truncation safety net
    if output.len() > MAX_CONTEXT_CHARS {
        output.truncate(MAX_CONTEXT_CHARS);
        output.push_str("\n\n[Truncated — context volume limit reached]\n");
    }

    output
}

/// Append a formatted failed exploration to the output.
fn append_exploration(output: &mut String, item: &Value) {
    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let slug = item.get("slug").and_then(|v| v.as_str()).unwrap_or("");
    let strategy = item.get("strategy").and_then(|v| v.as_str()).unwrap_or("");
    let constraint = item
        .get("failureConstraint")
        .and_then(|v| v.as_str())
        .unwrap_or("no details available");

    output.push_str(&format!("- **{title}** (`{slug}`)\n"));
    if !strategy.is_empty() {
        output.push_str(&format!("  Strategy: {strategy}\n"));
    }
    output.push_str(&format!("  Why it failed: {constraint}\n"));
}

/// Append a formatted decision to the output.
fn append_decision(output: &mut String, item: &Value) {
    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let slug = item.get("slug").and_then(|v| v.as_str()).unwrap_or("");
    let status = item.get("status").and_then(|v| v.as_str()).unwrap_or("active");
    let rationale = item
        .get("rationale")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    output.push_str(&format!("- **{title}** (`{slug}`, {status})\n"));
    if !rationale.is_empty() {
        output.push_str(&format!("  Rationale: {rationale}\n"));
    }
}

/// Append a formatted learning to the output.
fn append_learning(output: &mut String, item: &Value) {
    let title = item.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
    let category = item.get("category").and_then(|v| v.as_str()).unwrap_or("general");
    let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");

    output.push_str(&format!("- **{title}** [{category}]\n"));
    if !content.is_empty() {
        // Truncate individual content to prevent one item from dominating
        let truncated = if content.len() > 200 {
            // Use char_indices for safe UTF-8 truncation
            let truncate_at = content
                .char_indices()
                .nth(200)
                .map(|(i, _)| i)
                .unwrap_or(content.len());
            format!("{}...", &content[..truncate_at])
        } else {
            content.to_string()
        };
        output.push_str(&format!("  {truncated}\n"));
    }
}
