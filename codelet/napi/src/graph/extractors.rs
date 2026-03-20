//! Graph Entity Extractors — Zero-Cost Structural Indexing
//!
//! Pure functions that extract graph entities from tool call data.
//! No LLM calls, no side effects — pattern matching only.
//!
//! Integration: Called from session_manager.rs after tool call results.

use chrono::Utc;
use serde_json::Map;
use std::path::Path;

/// A graph entity to be upserted into the nanograph database.
#[derive(Debug, Clone)]
pub enum GraphEntity {
    /// A node to insert/update.
    Node {
        node_type: String,
        slug: String,
        properties: Map<String, serde_json::Value>,
    },
    /// An edge to insert.
    Edge {
        edge_type: String,
        from_slug: String,
        to_slug: String,
        properties: Map<String, serde_json::Value>,
    },
}

/// Build a Turn slug consistent with the `{session}:{turn}` format used
/// throughout the graph pipeline (LLM extraction, dispatch tests, etc.).
pub fn turn_slug(session_slug: &str, turn_index: u32) -> String {
    format!("{}:{}", session_slug, turn_index)
}

/// Extract entities from a file operation (Write or Edit tool call).
///
/// Produces:
///   1. A Turn node  (ensures the edge's `from` resolves)
///   2. A CodeEntity node
///   3. A Modifies edge  (Turn → CodeEntity)
pub fn extract_from_file_operation(
    tool_name: &str,
    file_path: &str,
    session_slug: &str,
    turn_index: u32,
) -> Vec<GraphEntity> {
    let mut entities = Vec::new();

    let operation = match tool_name {
        "Write" => "created",
        "Edit" => "modified",
        _ => return entities,
    };

    let language = infer_language(file_path);
    let slug = slugify_path(file_path);
    let t_slug = turn_slug(session_slug, turn_index);
    let now = Utc::now().to_rfc3339();

    // Turn node — created inline so the Modifies edge can resolve.
    // Duplicate turns (same slug) are safely merged via @key upsert.
    let mut turn_props = Map::new();
    turn_props.insert("slug".to_string(), serde_json::Value::String(t_slug.clone()));
    turn_props.insert("sessionSlug".to_string(), serde_json::Value::String(session_slug.to_string()));
    turn_props.insert("turnIndex".to_string(), serde_json::Value::Number(turn_index.into()));
    turn_props.insert("role".to_string(), serde_json::Value::String("assistant".to_string()));
    turn_props.insert("timestamp".to_string(), serde_json::Value::String(now.clone()));
    entities.push(GraphEntity::Node {
        node_type: "Turn".to_string(),
        slug: t_slug.clone(),
        properties: turn_props,
    });

    // CodeEntity node
    let mut props = Map::new();
    props.insert("slug".to_string(), serde_json::Value::String(slug.clone()));
    props.insert("name".to_string(), serde_json::Value::String(
        Path::new(file_path).file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.to_string())
    ));
    props.insert("entityType".to_string(), serde_json::Value::String("file".to_string()));
    props.insert("filePath".to_string(), serde_json::Value::String(file_path.to_string()));
    if let Some(lang) = language {
        props.insert("language".to_string(), serde_json::Value::String(lang.to_string()));
    }
    props.insert("createdAt".to_string(), serde_json::Value::String(now.clone()));

    entities.push(GraphEntity::Node {
        node_type: "CodeEntity".to_string(),
        slug: slug.clone(),
        properties: props,
    });

    // Modifies edge (Turn → CodeEntity)
    let mut edge_props = Map::new();
    edge_props.insert("operation".to_string(), serde_json::Value::String(operation.to_string()));
    edge_props.insert("extractedAt".to_string(), serde_json::Value::String(now));

    entities.push(GraphEntity::Edge {
        edge_type: "Modifies".to_string(),
        from_slug: t_slug,
        to_slug: slug,
        properties: edge_props,
    });

    entities
}

/// Extract entities from an fspec command (create-story, create-bug, etc.).
pub fn extract_from_fspec_command(
    command: &str,
    work_unit_id: &str,
    title: &str,
    session_slug: &str,
) -> Vec<GraphEntity> {
    let mut entities = Vec::new();

    let work_type = match command {
        "create-story" => Some("story"),
        "create-bug" => Some("bug"),
        "create-task" => Some("task"),
        _ => None,
    };

    if let Some(wt) = work_type {
        let mut props = Map::new();
        props.insert("slug".to_string(), serde_json::Value::String(work_unit_id.to_string()));
        props.insert("title".to_string(), serde_json::Value::String(title.to_string()));
        props.insert("workType".to_string(), serde_json::Value::String(wt.to_string()));
        props.insert("status".to_string(), serde_json::Value::String("backlog".to_string()));
        let now = Utc::now().to_rfc3339();
        props.insert("createdAt".to_string(), serde_json::Value::String(now.clone()));
        props.insert("updatedAt".to_string(), serde_json::Value::String(now));

        entities.push(GraphEntity::Node {
            node_type: "WorkUnit".to_string(),
            slug: work_unit_id.to_string(),
            properties: props,
        });
    }

    if command == "update-work-unit-status" {
        let now = Utc::now().to_rfc3339();

        // Session node — created inline so the WorksOn edge can resolve.
        // Duplicate sessions (same slug) are safely merged via @key upsert.
        let mut session_props = Map::new();
        session_props.insert("slug".to_string(), serde_json::Value::String(session_slug.to_string()));
        session_props.insert("startedAt".to_string(), serde_json::Value::String(now.clone()));
        session_props.insert("lastIndexedAt".to_string(), serde_json::Value::String(now.clone()));
        session_props.insert("turnCount".to_string(), serde_json::Value::Number(0.into()));
        session_props.insert("indexedTurnCount".to_string(), serde_json::Value::Number(0.into()));
        entities.push(GraphEntity::Node {
            node_type: "Session".to_string(),
            slug: session_slug.to_string(),
            properties: session_props,
        });

        // WorksOn edge (Session → WorkUnit)
        let mut edge_props = Map::new();
        edge_props.insert("linkedAt".to_string(), serde_json::Value::String(now));
        entities.push(GraphEntity::Edge {
            edge_type: "WorksOn".to_string(),
            from_slug: session_slug.to_string(),
            to_slug: work_unit_id.to_string(),
            properties: edge_props,
        });
    }

    entities
}

/// Infer programming language from file extension.
fn infer_language(file_path: &str) -> Option<&'static str> {
    let ext = Path::new(file_path).extension()?.to_str()?;
    match ext {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "py" => Some("python"),
        "go" => Some("go"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
        "rb" => Some("ruby"),
        "swift" => Some("swift"),
        "kt" | "kts" => Some("kotlin"),
        "scala" => Some("scala"),
        "php" => Some("php"),
        "sh" | "bash" | "zsh" => Some("bash"),
        "css" | "scss" | "less" => Some("css"),
        "html" | "htm" => Some("html"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
        "md" | "markdown" => Some("markdown"),
        "sql" => Some("sql"),
        _ => None,
    }
}

/// Convert a file path to a URL-safe slug.
fn slugify_path(file_path: &str) -> String {
    file_path
        .replace('/', "-")
        .replace('\\', "-")
        .replace('.', "-")
}

/// Batch queue for graph entities.
///
/// Accumulates entities and flushes when the threshold is reached.
pub struct EntityQueue {
    buffer: Vec<GraphEntity>,
    threshold: usize,
}

impl EntityQueue {
    pub fn new(threshold: usize) -> Self {
        Self {
            buffer: Vec::new(),
            threshold,
        }
    }

    /// Push an entity. Returns `Some(batch)` if threshold is reached.
    pub fn push(&mut self, entity: GraphEntity) -> Option<Vec<GraphEntity>> {
        self.buffer.push(entity);
        if self.buffer.len() >= self.threshold {
            Some(self.flush())
        } else {
            None
        }
    }

    /// Flush all pending entities.
    pub fn flush(&mut self) -> Vec<GraphEntity> {
        std::mem::take(&mut self.buffer)
    }

    /// Number of pending entities.
    pub fn pending_count(&self) -> usize {
        self.buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feature: spec/features/structural-extractors-zero-cost-indexing.feature
    ///
    /// Tests for pure graph entity extractor functions.

    // ============================================================================
    // Scenario: File edit creates CodeEntity node
    // ============================================================================
    #[test]
    fn test_file_edit_creates_code_entity_node() {
        // @step Given the graph database is initialized

        // @step When an Edit tool call modifies 'src/auth/login.rs'
        let entities = extract_from_file_operation("Edit", "src/auth/login.rs", "session-1", 0);

        // Verify all 3 entities are produced (Turn + CodeEntity + Modifies edge)
        assert_eq!(entities.len(), 3, "Expected 3 entities: Turn + CodeEntity + Modifies edge");

        // Verify Turn node
        let turn_node = entities.iter().find(|e| {
            matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Turn")
        });
        assert!(turn_node.is_some(), "Should produce a Turn node");
        if let Some(GraphEntity::Node { properties, slug, .. }) = turn_node {
            assert_eq!(slug, "session-1:0", "Turn slug should use session:turn format");
            assert_eq!(
                properties.get("sessionSlug").and_then(|v| v.as_str()),
                Some("session-1")
            );
        }

        // @step Then a CodeEntity node is produced with the file path, language 'rust', and entityType 'file'
        let node = entities.iter().find(|e| {
            matches!(e, GraphEntity::Node { node_type, .. } if node_type == "CodeEntity")
        });
        assert!(node.is_some(), "Should produce a CodeEntity node");
        if let Some(GraphEntity::Node { properties, .. }) = node {
            assert_eq!(
                properties.get("filePath").and_then(|v| v.as_str()),
                Some("src/auth/login.rs")
            );
            assert_eq!(
                properties.get("language").and_then(|v| v.as_str()),
                Some("rust")
            );
            assert_eq!(
                properties.get("entityType").and_then(|v| v.as_str()),
                Some("file")
            );
        }

        // @step And a Modifies edge is produced linking the current turn to the CodeEntity
        let edge = entities.iter().find(|e| {
            matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Modifies")
        });
        assert!(edge.is_some(), "Should produce a Modifies edge");
        if let Some(GraphEntity::Edge { from_slug, properties, .. }) = edge {
            assert_eq!(from_slug, "session-1:0", "Modifies edge from_slug should match Turn slug");
            assert_eq!(
                properties.get("operation").and_then(|v| v.as_str()),
                Some("modified")
            );
            // Verify required extractedAt field is present
            assert!(
                properties.get("extractedAt").is_some(),
                "Modifies edge should have extractedAt DateTime"
            );
        }
    }

    // ============================================================================
    // Scenario: Fspec create-story produces WorkUnit node
    // ============================================================================
    #[test]
    fn test_fspec_create_story_produces_work_unit_node() {
        // @step Given the graph database is initialized

        // @step When an Fspec tool call with command 'create-story' creates work unit 'AUTH-001'
        let entities =
            extract_from_fspec_command("create-story", "AUTH-001", "User Login", "session-1");

        // @step Then a WorkUnit node is produced with slug 'AUTH-001', title, and workType 'story'
        let node = entities.iter().find(|e| {
            matches!(e, GraphEntity::Node { node_type, .. } if node_type == "WorkUnit")
        });
        assert!(node.is_some(), "Should produce a WorkUnit node");
        if let Some(GraphEntity::Node { properties, .. }) = node {
            assert_eq!(
                properties.get("slug").and_then(|v| v.as_str()),
                Some("AUTH-001")
            );
            assert_eq!(
                properties.get("title").and_then(|v| v.as_str()),
                Some("User Login")
            );
            assert_eq!(
                properties.get("workType").and_then(|v| v.as_str()),
                Some("story")
            );
        }
    }

    // ============================================================================
    // Scenario: Batch queue flushes at threshold
    // ============================================================================
    #[test]
    fn test_batch_queue_flushes_at_threshold() {
        // @step Given the entity queue has 49 pending entities
        let mut queue = EntityQueue::new(50);
        for i in 0..49 {
            queue.push(GraphEntity::Node {
                node_type: "CodeEntity".to_string(),
                slug: format!("file-{i}"),
                properties: serde_json::Map::new(),
            });
        }
        assert_eq!(queue.pending_count(), 49);

        // @step When one more entity is added to the queue
        let flushed = queue.push(GraphEntity::Node {
            node_type: "CodeEntity".to_string(),
            slug: "file-49".to_string(),
            properties: serde_json::Map::new(),
        });

        // @step Then all 50 entities are flushed to the graph database
        assert!(flushed.is_some(), "Queue should flush at threshold");
        assert_eq!(flushed.unwrap().len(), 50);

        // @step And the queue is empty after flush
        assert_eq!(queue.pending_count(), 0);
    }

    // ============================================================================
    // Extractors are pure functions — always produce entities regardless of DB state.
    // The "silently skip" scenario (KGRAPH-004) is tested in
    // graph_entity_pipeline_test.rs::test_extract_and_queue_silently_skips_when_graph_unavailable
    // which tests the actual is_graph_initialized() guard in entity_pipeline.rs.
    // ============================================================================
    #[test]
    fn test_extractors_are_pure_and_always_produce_entities() {
        // @step Given a Write tool call with a file path
        let entities = extract_from_file_operation("Write", "src/new_file.ts", "session-1", 0);

        // @step Then the extractor always returns entities regardless of DB state
        // Extractors are pure functions — the "silently skip" guard lives in entity_pipeline.rs
        assert_eq!(entities.len(), 3, "Extractor produces 3 entities (Turn + CodeEntity + Modifies edge) regardless of DB state");
    }

    // ============================================================================
    // Scenario: update-work-unit-status produces Session node and WorksOn edge
    // ============================================================================
    #[test]
    fn test_update_work_unit_status_produces_session_node_and_works_on_edge() {
        // @step Given the graph database is initialized

        // @step When an Fspec tool call with command 'update-work-unit-status' is processed
        let entities = extract_from_fspec_command(
            "update-work-unit-status",
            "AUTH-001",
            "specifying",
            "session-abc-123",
        );

        // @step Then a Session node is produced so the WorksOn edge can resolve
        assert_eq!(entities.len(), 2, "Expected 2 entities: Session node + WorksOn edge");

        let session_node = entities.iter().find(|e| {
            matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Session")
        });
        assert!(session_node.is_some(), "Should produce a Session node");
        if let Some(GraphEntity::Node { slug, properties, .. }) = session_node {
            assert_eq!(slug, "session-abc-123");
            assert_eq!(
                properties.get("slug").and_then(|v| v.as_str()),
                Some("session-abc-123")
            );
            assert!(properties.get("startedAt").is_some(), "Session should have startedAt");
        }

        // @step And a WorksOn edge is produced linking the session to the work unit
        let edge = entities.iter().find(|e| {
            matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "WorksOn")
        });
        assert!(edge.is_some(), "Should produce a WorksOn edge");
        if let Some(GraphEntity::Edge { from_slug, to_slug, properties, .. }) = edge {
            assert_eq!(from_slug, "session-abc-123");
            assert_eq!(to_slug, "AUTH-001");
            assert!(properties.get("linkedAt").is_some(), "WorksOn edge should have linkedAt");
        }
    }
}
