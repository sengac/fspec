//! Context Compaction Module
//!
//! This module implements hierarchical lossless context compaction using:
//! - Layer 0: Structurally lossless trimming (base64, metadata, tool output)
//! - Per-turn structural annotation detection (fspec milestones, error resolutions, file modifications)
//! - In-view DAG construction via agent-driven SessionSearch + inject_summary
//! - Cache-aware token tracking
//!
//! Legacy batch LLM compaction has been removed.

pub mod annotation_detector;
mod model;
pub mod trimmer;
mod trimmer_base64;
mod trimmer_metadata;

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod trimmer_tests {
    include!("__tests__/trimmer.test.rs");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod structural_annotation_tests {
    include!("__tests__/structural_annotation.test.rs");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod annotation_detector_tests {
    include!("__tests__/annotation_detector.test.rs");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod dag_node_parsing_tests {
    include!("__tests__/dag_node_parsing.test.rs");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod dag_node_proptest {
    include!("__tests__/dag_node_proptest.test.rs");
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]
mod token_tracker_proptest {
    include!("__tests__/token_tracker_proptest.test.rs");
}

// Re-export public types from model
pub use model::{
    ConversationTurn, DagDepth, DagNodeMeta, FileOp, StructuralAnnotation, TokenTracker, ToolCall,
    ToolResult,
};

// Re-export dag-node parser and DAG content wrapper
pub use model::parse_dag_nodes;
pub use model::wrap_dag_content;

// Re-export trimmer
pub use trimmer::Trimmer;
