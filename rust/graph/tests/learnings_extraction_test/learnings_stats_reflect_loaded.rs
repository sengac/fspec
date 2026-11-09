// Feature: spec/features/learnings-extraction-pipeline-session-boundary-analysis.feature
use super::*;

// ============================================================================
// Scenario: Learnings stats reflect loaded entities
// ============================================================================
#[tokio::test]
async fn test_learnings_stats_reflect_loaded_entities() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given a Learnings graph database initialized in a temp directory
    let db = setup_learnings_db(temp_dir.path()).await;

    // @step And entities extracted from a realistic LLM response containing Learning and Exploration nodes
    let result = extract_learnings_from_text(REALISTIC_DAG_SUMMARY, Some(REALISTIC_LLM_RESPONSE));
    let extraction = result.expect("extraction should succeed");

    // @step When the entities are loaded into the Learnings graph
    let load_result = db.load_entities(&extraction.entities).await;
    assert!(load_result.is_ok(), "Loading should succeed");

    // @step And dispatch_learnings_stats is called
    let stats_result = learnings_dispatch::dispatch_learnings_stats(&db).await;
    let parsed: serde_json::Value =
        serde_json::from_str(&stats_result).expect("stats should be valid JSON");

    // @step Then the stats should show Learning count greater than 0
    let learning_count = parsed["nodes"]["Learning"].as_u64().unwrap_or(0);
    assert!(
        learning_count > 0,
        "Learning count should be > 0, got {}",
        learning_count
    );

    // @step And the stats should show Exploration count greater than 0
    let exploration_count = parsed["nodes"]["Exploration"].as_u64().unwrap_or(0);
    assert!(
        exploration_count > 0,
        "Exploration count should be > 0, got {}",
        exploration_count
    );
}

// ============================================================================
// Scenario: Graceful failure when LLM response is unavailable
// ============================================================================
#[tokio::test]
async fn test_graceful_failure_when_llm_unavailable() {
    let temp_dir = tempfile::tempdir().expect("temp dir");

    // @step Given a DAG summary text to process
    let summary_text = REALISTIC_DAG_SUMMARY;

    // @step And no LLM response is available
    let llm_response: Option<&str> = None;

    // @step When the extraction pipeline attempts to process the text
    let result = extract_learnings_from_text(summary_text, llm_response);

    // @step Then the result should be an Err with a descriptive message
    assert!(
        result.is_err(),
        "Should return an error when LLM is unavailable"
    );
    let err_msg = result.unwrap_err();
    assert!(
        !err_msg.is_empty(),
        "Error message should be descriptive, not empty"
    );

    // @step And a subsequently initialized Learnings graph database should remain empty
    let db = setup_learnings_db(temp_dir.path()).await;
    let stats = db.stats().expect("stats should work");
    let total_nodes: u64 = stats
        .get("nodes")
        .and_then(|v| v.as_object())
        .map(|obj| obj.values().filter_map(|v| v.as_u64()).sum())
        .unwrap_or(0);
    assert_eq!(total_nodes, 0, "Graph should remain empty after failure");
}

// ============================================================================
// Scenario: Volume constraint truncates at 20 entities
// ============================================================================
#[tokio::test]
async fn test_volume_constraint_truncates_at_20() {
    // @step Given an LLM response JSON containing 25 valid Learning entities
    let mut learnings_json = Vec::new();
    for i in 0..25 {
        learnings_json.push(format!(
            r#"{{
                "slug": "learning-{i:03}",
                "title": "Learning number {i}",
                "content": "Content for learning {i}",
                "category": "discovery",
                "confidence": "medium"
            }}"#
        ));
    }
    let response_json = format!(
        r#"```json
{{
  "learnings": [{}],
  "explorations": [],
  "constraints": []
}}
```"#,
        learnings_json.join(",\n")
    );

    // @step When the extraction pipeline processes the response
    let result = extract_learnings_from_text("test summary", Some(&response_json));

    // @step Then exactly 20 entities should be returned
    assert!(result.is_ok(), "Extraction should succeed");
    let extraction = result.unwrap();
    assert_eq!(
        extraction.entities.len(),
        20,
        "Should be truncated to 20, got {}",
        extraction.entities.len()
    );

    // @step And no error should be raised
    // Already confirmed by the is_ok() check above
}

// ============================================================================
// Scenario: Invalid entity categories are skipped
// ============================================================================
#[tokio::test]
async fn test_invalid_categories_are_skipped() {
    // @step Given an LLM response JSON containing 3 learnings where 1 has category "foo"
    let response = r#"```json
{
  "learnings": [
    {
      "slug": "valid-one",
      "title": "Valid learning one",
      "content": "This has a valid category",
      "category": "convention",
      "confidence": "high"
    },
    {
      "slug": "invalid-category",
      "title": "Invalid category learning",
      "content": "This has an invalid category",
      "category": "foo",
      "confidence": "high"
    },
    {
      "slug": "valid-two",
      "title": "Valid learning two",
      "content": "This also has a valid category",
      "category": "pattern",
      "confidence": "medium"
    }
  ],
  "explorations": [],
  "constraints": []
}
```"#;

    // @step When the extraction pipeline processes the response
    let result = extract_learnings_from_text("test summary", Some(response));
    assert!(result.is_ok(), "Extraction should succeed");
    let extraction = result.unwrap();

    // @step Then only 2 Learning nodes should be returned
    let learning_count = count_nodes(&extraction.entities, "Learning");
    assert_eq!(
        learning_count, 2,
        "Should have 2 valid learnings, got {learning_count}"
    );

    // @step And the entity with invalid category "foo" should be skipped
    assert!(
        find_node(&extraction.entities, "Learning", "invalid-category").is_none(),
        "The entity with category 'foo' should not be present"
    );
    assert!(
        find_node(&extraction.entities, "Learning", "valid-one").is_some(),
        "'valid-one' should be present"
    );
    assert!(
        find_node(&extraction.entities, "Learning", "valid-two").is_some(),
        "'valid-two' should be present"
    );
}

// ============================================================================
// Scenario: Malformed JSON response returns parse error
// ============================================================================
#[tokio::test]
async fn test_malformed_json_returns_parse_error() {
    // @step Given an LLM response containing invalid JSON
    let malformed_response = r#"```json
{
  "learnings": [
    { "slug": "broken", "title": "this is missing a closing bracket
  ]
}
```"#;

    // @step When the extraction pipeline attempts to process the response
    let result = extract_learnings_from_text("test summary", Some(malformed_response));

    // @step Then the result should be an Err containing a parse error message
    assert!(result.is_err(), "Should fail on malformed JSON");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("parse") || err_msg.contains("Parse") || err_msg.contains("JSON"),
        "Error should mention parsing, got: {err_msg}"
    );

    // @step And no entities should be produced
    // Already confirmed — result is Err, so no extraction to inspect
}
