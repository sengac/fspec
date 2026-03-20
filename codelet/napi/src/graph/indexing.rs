//! Scheduled Indexing via Skills File
//!
//! Skills file config parsing — parses markdown JSON blocks, validates with defaults.
//! Session scanning pipeline is in session_scanner.rs.
//!
//! Feature: spec/features/scheduled-indexing-via-skills-file.feature

use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::warn;

/// Default cron frequency for indexing.
const DEFAULT_FREQUENCY: &str = "*/15 * * * *";
/// Default batch size for LLM extraction.
const DEFAULT_BATCH_SIZE: u32 = 10;
/// Default extraction mode.
const DEFAULT_EXTRACTION_MODE: &str = "hybrid";
/// Default timezone.
const DEFAULT_TIMEZONE: &str = "UTC";

/// Indexing configuration parsed from skills file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    #[serde(default = "default_frequency")]
    pub frequency: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(rename = "batchSize", default = "default_batch_size")]
    pub batch_size: u32,
    #[serde(default)]
    pub extraction: ExtractionConfig,
}

/// Extraction-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionConfig {
    #[serde(default = "default_extraction_mode")]
    pub mode: String,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            mode: DEFAULT_EXTRACTION_MODE.to_string(),
        }
    }
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            frequency: DEFAULT_FREQUENCY.to_string(),
            timezone: DEFAULT_TIMEZONE.to_string(),
            batch_size: DEFAULT_BATCH_SIZE,
            extraction: ExtractionConfig::default(),
        }
    }
}

fn default_frequency() -> String {
    DEFAULT_FREQUENCY.to_string()
}
fn default_timezone() -> String {
    DEFAULT_TIMEZONE.to_string()
}
fn default_batch_size() -> u32 {
    DEFAULT_BATCH_SIZE
}
fn default_extraction_mode() -> String {
    DEFAULT_EXTRACTION_MODE.to_string()
}

/// Result of loading a skills file.
#[derive(Debug)]
pub enum SkillsLoadResult {
    /// Config loaded successfully.
    Loaded(IndexingConfig),
    /// Skills file does not exist — no indexing configured.
    NotFound,
    /// Skills file exists but has no JSON config blocks.
    NoConfig,
}

/// Extract JSON config blocks from markdown content.
///
/// Looks for fenced code blocks with `json` language identifier.
fn extract_json_blocks(markdown: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_json_block = false;
    let mut current_block = String::new();

    for line in markdown.lines() {
        if line.trim().starts_with("```json") {
            in_json_block = true;
            current_block.clear();
        } else if in_json_block && line.trim() == "```" {
            in_json_block = false;
            if !current_block.trim().is_empty() {
                blocks.push(current_block.trim().to_string());
            }
        } else if in_json_block {
            current_block.push_str(line);
            current_block.push('\n');
        }
    }

    blocks
}

/// Load indexing config from a skills markdown file.
pub fn load_skills_file(path: &Path) -> SkillsLoadResult {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return SkillsLoadResult::NotFound;
        }
        Err(e) => {
            warn!("Failed to read skills file {}: {e}", path.display());
            return SkillsLoadResult::NotFound;
        }
    };

    let json_blocks = extract_json_blocks(&content);

    let first_block = match json_blocks.first() {
        Some(block) => block,
        None => return SkillsLoadResult::NoConfig,
    };

    // Parse first JSON block as config, merging with defaults
    match serde_json::from_str::<IndexingConfig>(first_block) {
        Ok(config) => SkillsLoadResult::Loaded(config),
        Err(e) => {
            warn!("Failed to parse indexing config JSON: {e}, using defaults");
            SkillsLoadResult::Loaded(IndexingConfig::default())
        }
    }
}

/// Determine unindexed turn range for a session.
///
/// Returns `(start_turn, end_turn)` for the turns that need indexing.
/// Returns `None` if the session is fully indexed.
pub fn unindexed_turn_range(
    total_turns: u32,
    watermark_turn: u32,
) -> Option<(u32, u32)> {
    if watermark_turn >= total_turns {
        None
    } else {
        Some((watermark_turn + 1, total_turns))
    }
}

// Re-export session scanner for backward compat
pub use super::session_scanner::{scan_and_index_sessions, ScanResult};

// Unit tests moved to codelet/napi/tests/graph_session_indexing_test.rs
// to consolidate all KGRAPH-008 tests in one file per the 1:1 coverage rule.
