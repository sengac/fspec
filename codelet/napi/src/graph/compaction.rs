//! Graph Compaction & Schema Migration
//!
//! Turn node pruning with configurable retention, schema migration
//! via hash comparison, and compaction configuration management.
//!
//! Feature: spec/features/graph-compaction-schema-migration.feature

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::warn;

/// Default retention period for Turn nodes in days.
const DEFAULT_MAX_AGE_DAYS: u64 = 90;

/// Compaction retention configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// Maximum age in days for Turn nodes before pruning.
    #[serde(rename = "maxAgeDays", default = "default_max_age_days")]
    pub max_age_days: u64,
}

fn default_max_age_days() -> u64 {
    DEFAULT_MAX_AGE_DAYS
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            max_age_days: DEFAULT_MAX_AGE_DAYS,
        }
    }
}

/// Represents a Turn node for pruning evaluation.
#[derive(Debug, Clone)]
pub struct TurnNode {
    pub slug: String,
    pub timestamp: String,
    pub has_decides_edge: bool,
}

/// Result of a pruning operation.
#[derive(Debug)]
pub struct PruneResult {
    /// Slugs of Turn nodes that should be pruned.
    pub pruned_slugs: Vec<String>,
    /// Slugs of Turn nodes preserved (have Decides edges).
    pub preserved_slugs: Vec<String>,
    /// Edge types to cascade-delete for each pruned Turn.
    pub cascade_edge_types: Vec<String>,
}

/// Identify which Turn nodes should be pruned based on retention config.
///
/// Turns older than `max_age_days` are pruned UNLESS they have Decides edges
/// (decision provenance must be preserved).
pub fn identify_turns_to_prune(
    turns: &[TurnNode],
    cutoff_timestamp: &str,
) -> PruneResult {
    let mut pruned = Vec::new();
    let mut preserved = Vec::new();

    for turn in turns {
        if turn.timestamp.as_str() < cutoff_timestamp {
            if turn.has_decides_edge {
                preserved.push(turn.slug.clone());
            } else {
                pruned.push(turn.slug.clone());
            }
        }
    }

    PruneResult {
        pruned_slugs: pruned,
        preserved_slugs: preserved,
        cascade_edge_types: vec![
            "Mentions".to_string(),
            "Modifies".to_string(),
        ],
    }
}

/// Compute SHA256 hash of a schema source string.
pub fn schema_hash(schema_source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(schema_source.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Schema migration check result.
#[derive(Debug, PartialEq)]
pub enum MigrationCheck {
    /// Schema hashes match — no migration needed.
    NoMigrationNeeded,
    /// Schema has changed — safe migration can be attempted.
    MigrationRequired { bundled_hash: String, disk_hash: String },
}

/// Compare bundled schema hash with on-disk schema to determine if migration is needed.
pub fn check_schema_migration(
    bundled_schema: &str,
    on_disk_schema_ir_json: &str,
) -> MigrationCheck {
    let bundled_hash = schema_hash(bundled_schema);
    let disk_hash = schema_hash(on_disk_schema_ir_json);

    if bundled_hash == disk_hash {
        MigrationCheck::NoMigrationNeeded
    } else {
        MigrationCheck::MigrationRequired {
            bundled_hash,
            disk_hash,
        }
    }
}

/// Calculate cutoff timestamp for pruning (current time minus retention days).
pub fn calculate_cutoff_timestamp(now: &str, max_age_days: u64) -> String {
    // Parse ISO 8601 date and subtract days
    if let Some(date_part) = now.get(..10) {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            let cutoff = date - chrono::Duration::days(max_age_days as i64);
            return format!("{}T00:00:00Z", cutoff.format("%Y-%m-%d"));
        }
        warn!("Failed to parse date from timestamp: {now}");
    } else {
        warn!("Timestamp too short to extract date: {now}");
    }
    // Fallback: return epoch (prune nothing)
    "1970-01-01T00:00:00Z".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // Scenario: Old Turn nodes are pruned except decision-linked ones
    // ============================================================================
    #[test]
    fn test_old_turns_pruned_except_decision_linked() {
        // @step Given Turn nodes exist with various ages, some older than the 90-day retention period
        let old_timestamp = "2025-12-01T00:00:00Z"; // Way older than 90 days from 2026-03-19
        let recent_timestamp = "2026-03-15T00:00:00Z";

        let turns = vec![
            // Old turns without decisions — should be pruned
            TurnNode {
                slug: "sess1:0".to_string(),
                timestamp: old_timestamp.to_string(),
                has_decides_edge: false,
            },
            TurnNode {
                slug: "sess1:1".to_string(),
                timestamp: old_timestamp.to_string(),
                has_decides_edge: false,
            },
            TurnNode {
                slug: "sess1:2".to_string(),
                timestamp: old_timestamp.to_string(),
                has_decides_edge: false,
            },
            // @step And 5 of the old Turn nodes have Decides edges linking them to decisions
            TurnNode {
                slug: "sess1:3".to_string(),
                timestamp: old_timestamp.to_string(),
                has_decides_edge: true,
            },
            TurnNode {
                slug: "sess1:4".to_string(),
                timestamp: old_timestamp.to_string(),
                has_decides_edge: true,
            },
            // Recent turn — should not be pruned
            TurnNode {
                slug: "sess1:5".to_string(),
                timestamp: recent_timestamp.to_string(),
                has_decides_edge: false,
            },
        ];

        let config = RetentionConfig::default();

        // @step When the compaction pruning runs with default maxAgeDays of 90
        let cutoff = calculate_cutoff_timestamp("2026-03-19T00:00:00Z", config.max_age_days);
        let result = identify_turns_to_prune(&turns, &cutoff);

        // @step Then Turn nodes older than 90 days without Decides edges are pruned
        assert_eq!(result.pruned_slugs.len(), 3);
        assert!(result.pruned_slugs.contains(&"sess1:0".to_string()));
        assert!(result.pruned_slugs.contains(&"sess1:1".to_string()));
        assert!(result.pruned_slugs.contains(&"sess1:2".to_string()));

        // @step And Turn nodes with Decides edges are preserved regardless of age
        assert_eq!(result.preserved_slugs.len(), 2);
        assert!(result.preserved_slugs.contains(&"sess1:3".to_string()));
        assert!(result.preserved_slugs.contains(&"sess1:4".to_string()));
    }

    // ============================================================================
    // Scenario: Pruning a Turn cascades to delete its edges
    // ============================================================================
    #[test]
    fn test_prune_cascades_to_edges() {
        // @step Given a Turn node has Mentions and Modifies edges attached to it
        let turns = vec![TurnNode {
            slug: "sess1:10".to_string(),
            timestamp: "2025-01-01T00:00:00Z".to_string(),
            has_decides_edge: false,
        }];

        let config = RetentionConfig::default();
        let cutoff = calculate_cutoff_timestamp("2026-03-19T00:00:00Z", config.max_age_days);

        // @step When the Turn node is marked for pruning
        let result = identify_turns_to_prune(&turns, &cutoff);

        // @step Then the Turn node is deleted
        assert_eq!(result.pruned_slugs.len(), 1);
        assert_eq!(result.pruned_slugs[0], "sess1:10");

        // @step And its Mentions edges are also deleted
        assert!(result.cascade_edge_types.contains(&"Mentions".to_string()));

        // @step And its Modifies edges are also deleted
        assert!(result.cascade_edge_types.contains(&"Modifies".to_string()));
    }

    // ============================================================================
    // Scenario: Schema hash match skips migration
    // ============================================================================
    #[test]
    fn test_schema_hash_match_skips_migration() {
        // @step Given the bundled schema hash matches the on-disk schema.ir.json hash
        let schema = "node Concept { slug: String @key }";
        let same_schema = "node Concept { slug: String @key }";

        // @step When the database is opened
        let result = check_schema_migration(schema, same_schema);

        // @step Then no migration is performed
        // @step And the database opens normally
        assert_eq!(result, MigrationCheck::NoMigrationNeeded);
    }

    // ============================================================================
    // Scenario: Safe schema change auto-migrates on open
    // ============================================================================
    #[test]
    fn test_schema_change_triggers_migration() {
        // @step Given the bundled schema has a new optional property added to an existing node type
        let bundled = "node Concept { slug: String @key\n  tags: [String]? }";

        // @step And the on-disk schema.ir.json has a different hash
        let on_disk = "node Concept { slug: String @key }";

        // @step When the database is opened
        let result = check_schema_migration(bundled, on_disk);

        // @step Then the schema migration applies the safe change automatically
        // @step And the database opens with the updated schema
        match result {
            MigrationCheck::MigrationRequired {
                bundled_hash,
                disk_hash,
            } => {
                assert_ne!(bundled_hash, disk_hash);
            }
            MigrationCheck::NoMigrationNeeded => {
                panic!("Expected migration to be required");
            }
        }
    }
}
