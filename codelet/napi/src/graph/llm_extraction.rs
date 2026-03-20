//! LLM-Based Concept Extraction Pipeline
//!
//! Builds extraction prompts from conversation turn batches. Validation
//! of LLM responses is in the `llm_validation` sibling module.
//!
//! Feature: spec/features/llm-concept-extraction.feature

use serde::Deserialize;

/// Maximum characters per turn in the extraction prompt.
const MAX_TURN_CONTENT_LENGTH: usize = 2000;

/// A conversation turn to be included in an extraction batch.
#[derive(Debug, Clone)]
pub struct ConversationTurn {
    pub role: String,
    pub content: String,
    pub turn_index: u32,
}

/// Raw extraction result from the LLM (before validation).
#[derive(Debug, Deserialize)]
pub struct ExtractionResult {
    #[serde(default)]
    pub concepts: Vec<ExtractedConcept>,
    #[serde(default)]
    pub decisions: Vec<ExtractedDecision>,
    #[serde(default)]
    pub relations: Vec<ExtractedRelation>,
}

/// A concept extracted by the LLM.
#[derive(Debug, Deserialize)]
pub struct ExtractedConcept {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub confidence: Option<String>,
}

/// A decision extracted by the LLM.
#[derive(Debug, Deserialize)]
pub struct ExtractedDecision {
    pub slug: Option<String>,
    pub title: Option<String>,
    pub rationale: Option<String>,
    pub domain: Option<String>,
    pub confidence: Option<String>,
}

/// A relation extracted by the LLM.
#[derive(Debug, Deserialize)]
pub struct ExtractedRelation {
    pub from: Option<String>,
    pub to: Option<String>,
    #[serde(rename = "type")]
    pub relation_type: Option<String>,
    pub strength: Option<f64>,
}

/// Filter a batch of turns to only user and assistant messages.
///
/// Tool results and other roles are excluded (handled by structural extractors).
pub fn filter_extractable_turns(turns: &[ConversationTurn]) -> Vec<&ConversationTurn> {
    turns
        .iter()
        .filter(|t| t.role == "user" || t.role == "assistant")
        .collect()
}

/// Build the extraction prompt from a batch of turns.
///
/// Each turn's content is truncated to MAX_TURN_CONTENT_LENGTH.
pub fn build_extraction_prompt(turns: &[&ConversationTurn]) -> String {
    let mut prompt = EXTRACTION_SYSTEM_PROMPT.to_string();
    prompt.push_str("\n\n## Conversation Turns\n\n");

    for turn in turns {
        let content = truncate_content(&turn.content, MAX_TURN_CONTENT_LENGTH);
        prompt.push_str(&format!(
            "### Turn {} ({})\n{}\n\n",
            turn.turn_index, turn.role, content
        ));
    }

    prompt
}

/// Truncate content to approximately max_len bytes (Unicode-safe).
///
/// Uses byte length for the threshold check and truncation point,
/// then scans backward to find a valid UTF-8 character boundary
/// to avoid splitting multi-byte characters.
fn truncate_content(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        content.to_string()
    } else {
        let mut end = max_len;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        let truncated = &content[..end];
        format!("{}...[truncated]", truncated)
    }
}

// Re-export validation function for backward compatibility
pub use super::llm_validation::parse_and_validate_response;

/// The extraction system prompt template.
const EXTRACTION_SYSTEM_PROMPT: &str = r#"You are a knowledge graph extractor. Given a batch of agent conversation turns, extract structured entities.

## Rules
- Only extract what is EXPLICITLY discussed — never infer or speculate
- Set confidence=high for explicitly named items, medium for contextually clear items, low for ambiguous references
- Use kebab-case for all slugs (e.g. "jwt-authentication", not "JWT Auth")
- Merge duplicates within a batch by slug
- Relations must connect two concepts that BOTH appear in this batch or were explicitly mentioned together

## Extract These Entity Types

### Concepts
Named ideas, technologies, patterns, domain terms.
```json
{ "slug": "string", "name": "string", "category": "string", "summary": "string", "confidence": "high|medium|low" }
```
Categories: architecture, convention, decision, dependency, domain_term, error_class, feature, library, pattern, person, platform, process, technology, tool

### Decisions
Explicit choices or conclusions reached (not hypotheticals).
```json
{ "slug": "string", "title": "string", "rationale": "string", "domain": "string", "confidence": "high|medium|low" }
```
Domains: architecture, convention, dependency, deployment, design, implementation, process, testing

### Relations
How two concepts relate. Both concepts must be in this batch.
```json
{ "from": "concept-slug", "to": "concept-slug", "type": "string", "strength": 0.0-1.0 }
```
Types: causes, composes, conflicts_with, depends_on, extends, implements, similar_to, supersedes, uses

## Output Format
Return a single JSON object:
```json
{
  "concepts": [...],
  "decisions": [...],
  "relations": [...]
}
```
Return empty arrays if nothing meaningful to extract."#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::extractors::GraphEntity;

    // ============================================================================
    // Scenario: Valid conversation batch produces concept, decision, and relation entities
    // ============================================================================
    #[test]
    fn test_valid_batch_produces_all_entity_types() {
        // @step Given a batch of 5 conversation turns discussing JWT authentication
        let llm_response = r#"{
            "concepts": [
                {"slug": "jwt-auth", "name": "JWT Authentication", "category": "technology", "summary": "Token-based auth", "confidence": "high"}
            ],
            "decisions": [
                {"slug": "use-jwt", "title": "Use JWT for auth", "rationale": "Stateless", "domain": "architecture", "confidence": "high"}
            ],
            "relations": [
                {"from": "jwt-auth", "to": "use-jwt", "type": "implements", "strength": 0.9}
            ]
        }"#;

        // @step When the LLM extraction pipeline processes the batch
        let entities =
            parse_and_validate_response(llm_response, "test-session", 0).unwrap();

        // @step Then Concept nodes are produced with valid slugs, categories, and confidence levels
        let concepts: Vec<_> = entities.iter().filter(|e| {
            matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Concept")
        }).collect();
        assert!(!concepts.is_empty(), "Should produce Concept nodes");

        // @step And Decision nodes are produced with valid domains and rationale
        let decisions: Vec<_> = entities.iter().filter(|e| {
            matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Decision")
        }).collect();
        assert!(!decisions.is_empty(), "Should produce Decision nodes");

        // Verify that a Turn node is emitted for the Decides edge
        let turn_nodes: Vec<_> = entities.iter().filter(|e| {
            matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Turn")
        }).collect();
        assert!(!turn_nodes.is_empty(), "Should produce Turn node for Decides edge provenance");

        // Verify Decides edge points from Turn, not Session
        let decides_edges: Vec<_> = entities.iter().filter(|e| {
            matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Decides")
        }).collect();
        assert_eq!(decides_edges.len(), 1, "Should produce exactly 1 Decides edge");
        if let GraphEntity::Edge { from_slug, .. } = decides_edges[0] {
            assert_eq!(from_slug, "test-session:0", "Decides edge should point from Turn slug, not Session slug");
        }

        // @step And RelatesTo edges connect related concepts with valid types and strength values
        let relates_to: Vec<_> = entities.iter().filter(|e| {
            matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "RelatesTo")
        }).collect();
        assert!(!relates_to.is_empty(), "Should produce RelatesTo edges");

        // @step And all entities are returned as Vec<GraphEntity> compatible with the merge/upsert pipeline
        // 1 concept + 1 Turn node + 1 decision + 1 Decides edge + 1 RelatesTo edge = 5
        assert_eq!(entities.len(), 5);
    }

    // ============================================================================
    // Scenario: Malformed concept entry is skipped without failing the batch
    // ============================================================================
    #[test]
    fn test_malformed_concept_is_skipped() {
        // @step Given an LLM response containing a concept with a missing slug field and two valid concepts
        let response = r#"{
            "concepts": [
                {"slug": "", "name": "Empty slug", "category": "technology"},
                {"slug": "valid-one", "name": "Valid One", "category": "tool", "confidence": "high"},
                {"slug": "valid-two", "name": "Valid Two", "category": "library", "confidence": "medium"}
            ],
            "decisions": [],
            "relations": []
        }"#;

        // @step When the response is parsed and validated
        let entities = parse_and_validate_response(response, "test-session", 0).unwrap();

        // @step Then the malformed concept is skipped
        // @step And the two valid concepts are returned as GraphEntity nodes
        assert_eq!(entities.len(), 2, "Should return 2 valid concepts, skipping the empty-slug one");
    }

    // ============================================================================
    // Scenario: Invalid enum values in entities are rejected
    // ============================================================================
    #[test]
    fn test_invalid_enum_values_rejected() {
        // @step Given an LLM response with a concept having category 'foobar' and a decision having domain 'invalid-domain'
        let response = r#"{
            "concepts": [
                {"slug": "bad-cat", "name": "Bad Category", "category": "foobar"},
                {"slug": "good-cat", "name": "Good Category", "category": "tool", "confidence": "high"}
            ],
            "decisions": [
                {"slug": "bad-domain", "title": "Bad Domain", "domain": "invalid-domain", "confidence": "high"},
                {"slug": "good-domain", "title": "Good Domain", "domain": "architecture", "confidence": "high"}
            ],
            "relations": []
        }"#;

        // @step When the response is parsed and validated
        let entities = parse_and_validate_response(response, "test-session", 0).unwrap();

        // @step Then both entities with invalid enum values are rejected
        // @step And entities with valid enum values from the same response are still returned
        // good-cat (1 concept) + good-domain (1 Turn node + 1 decision + 1 Decides edge) = 4
        assert_eq!(entities.len(), 4, "Should return valid entities only: 1 concept + 1 Turn + 1 decision + 1 Decides edge");
    }

    // ============================================================================
    // Scenario: Self-referencing relation is rejected
    // ============================================================================
    #[test]
    fn test_self_referencing_relation_rejected() {
        // @step Given an LLM response containing a relation where from and to slugs are identical
        let response = r#"{
            "concepts": [
                {"slug": "valid-concept", "name": "Valid", "category": "tool", "confidence": "high"}
            ],
            "decisions": [],
            "relations": [
                {"from": "same-slug", "to": "same-slug", "type": "uses", "strength": 0.5},
                {"from": "valid-concept", "to": "other-concept", "type": "uses", "strength": 0.6}
            ]
        }"#;

        // @step When the response is parsed and validated
        let entities = parse_and_validate_response(response, "test-session", 0).unwrap();

        // @step Then the self-referencing relation is rejected
        let self_ref_edges: Vec<_> = entities.iter().filter(|e| {
            matches!(e, GraphEntity::Edge { from_slug, to_slug, .. } if from_slug == to_slug)
        }).collect();
        assert!(self_ref_edges.is_empty(), "Self-referencing relations should be rejected");

        // @step And other valid entities in the response are still returned
        assert!(!entities.is_empty(), "Valid entities should still be returned");
    }

    // ============================================================================
    // Scenario: Tool-result-only batch is skipped without LLM invocation
    // ============================================================================
    #[test]
    fn test_tool_result_batch_skipped() {
        // @step Given a batch of turns that are all tool results with no user or assistant messages
        let turns = vec![
            ConversationTurn {
                role: "tool".to_string(),
                content: "file contents here".to_string(),
                turn_index: 0,
            },
        ];

        // @step When the batch is submitted to the extraction pipeline
        let filtered = filter_extractable_turns(&turns);

        // @step Then the pipeline returns an empty list without invoking the LLM
        assert!(filtered.is_empty());
    }

    // ============================================================================
    // Scenario: Long conversation turns are truncated before extraction
    // ============================================================================
    #[test]
    fn test_long_turns_truncated() {
        // @step Given a batch containing a user turn with 5000 characters of content
        let long_content = "x".repeat(5000);
        let turns = vec![
            ConversationTurn {
                role: "user".to_string(),
                content: long_content,
                turn_index: 0,
            },
        ];

        // @step When the extraction prompt is built from the batch
        let refs: Vec<&ConversationTurn> = turns.iter().collect();
        let prompt = build_extraction_prompt(&refs);

        // @step Then the turn content in the prompt is truncated to 2000 characters
        assert!(prompt.contains("...[truncated]"));
        // Prompt should not contain the full 5000 chars
        assert!(prompt.len() < 5000, "Prompt should be shorter than the original 5000-char content");
    }
}
