#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect,
    clippy::await_holding_lock
)]
// Feature: spec/features/learnings-graph-data-model.feature
//
// Learnings Graph Data Model & Schema
// Tests for the Learnings graph schema, JSONL loading, edge traversal,
// and registry integration.
//
// Each test uses an isolated temp directory to avoid polluting real data.

use codelet_graph::database::GraphDatabase;
use codelet_graph::registry;
use serde_json::Value;
use std::sync::Mutex;

/// The Learnings schema, same as what's bundled in the binary.
const LEARNINGS_SCHEMA: &str = include_str!("../schemas/learnings.pg");

/// Inline query source for traversing Learnings graph.
const LEARNINGS_QUERIES: &str = r#"
query all_learnings() {
    match { $l: Learning }
    return { $l.slug, $l.title, $l.category, $l.confidence, $l.content, $l.mentionCount }
}

query all_explorations() {
    match { $e: Exploration }
    return { $e.slug, $e.title, $e.strategy, $e.outcome }
}

query exploration_discoveries($exp_slug: String) {
    match {
        $e: Exploration { slug: $exp_slug }
        $e discovered $l
    }
    return { $l.slug, $l.title, $l.category }
}

query learning_related($learn_slug: String) {
    match {
        $src: Learning { slug: $learn_slug }
        $src relatesTo $dst
    }
    return { $dst.slug, $dst.title, $dst.category }
}

query learning_superseded_by($learn_slug: String) {
    match {
        $old: Learning { slug: $learn_slug }
        $new supersedes $old
    }
    return { $new.slug, $new.title }
}
"#;

// Global mutex for tests that use the global registry (scenario 4).
lazy_static::lazy_static! {
    static ref REGISTRY_TEST_MUTEX: Mutex<()> = Mutex::new(());
}

#[path = "learnings_graph_data_model_test/initialize_learnings_graph_database.rs"]
mod initialize_learnings_graph_database;
#[path = "learnings_graph_data_model_test/learnings_graph_registered_as.rs"]
mod learnings_graph_registered_as;
