//! Reverse ACDD session types + persistence — Rust port of
//! `src/types/reverse-session.ts` and `src/utils/reverse-session.ts` (RPC-294).
//!
//! The `reverse` command keeps an ephemeral session file in the OS temp
//! directory, keyed by a SHA-256 hash of the project root path (first 12 hex
//! chars), exactly mirroring the TS `getSessionPath` helper. All persistence
//! is blocking `std::fs` — no async — so the command resolves on the first
//! poll under `dispatch::poll_sync_future`.
//!
//! This module is the single source of truth for the session-file path so
//! both the command implementation AND its integration tests compute the
//! same location.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::io::project_root::find_project_root;
use crate::io::time::iso8601_now;

/// Gap analysis counts + the ordered list of files to process. Mirrors the TS
/// `GapAnalysis` interface (`src/types/reverse-session.ts:14-20`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapAnalysis {
    #[serde(rename = "testsWithoutFeatures", default)]
    pub tests_without_features: u64,
    #[serde(rename = "featuresWithoutTests", default)]
    pub features_without_tests: u64,
    #[serde(rename = "unmappedScenarios", default)]
    pub unmapped_scenarios: u64,
    #[serde(rename = "unmappedImplementation", default)]
    pub unmapped_implementation: u64,
    #[serde(default)]
    pub files: Vec<String>,
}

/// The persisted reverse session. Mirrors the TS `ReverseSession` interface
/// (`src/types/reverse-session.ts:22-31`). Optional fields are omitted when
/// `None`, matching `JSON.stringify` dropping `undefined` keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseSession {
    pub phase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(
        rename = "strategyName",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub strategy_name: Option<String>,
    #[serde(
        rename = "currentStep",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub current_step: Option<u64>,
    #[serde(
        rename = "totalSteps",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub total_steps: Option<u64>,
    pub gaps: GapAnalysis,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<Vec<String>>,
    pub timestamp: String,
}

/// Coverage analysis sub-result. Mirrors the inline object on the TS
/// `AnalysisResult.coverageAnalysis` field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageAnalysis {
    #[serde(rename = "unmappedCount", default)]
    pub unmapped_count: u64,
    #[serde(default)]
    pub scenarios: Vec<String>,
}

/// The project-analysis result. Mirrors the TS `AnalysisResult` interface
/// (`src/types/reverse-session.ts:33-42`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    #[serde(rename = "testFiles", default)]
    pub test_files: Vec<String>,
    #[serde(rename = "featureFiles", default)]
    pub feature_files: Vec<String>,
    #[serde(rename = "implementationFiles", default)]
    pub implementation_files: Vec<String>,
    #[serde(
        rename = "coverageAnalysis",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub coverage_analysis: Option<CoverageAnalysis>,
    pub summary: String,
}

/// Compute the absolute session-file path for a given project root.
///
/// Mirrors `getSessionPath` (`src/utils/reverse-session.ts:17-29`): resolve
/// the project root via boundary-marker walk, SHA-256 hash its string form,
/// take the first 12 hex characters, and place the file in the OS temp dir as
/// `fspec-reverse-<hash>.json`.
pub fn session_path(project_root: &Path) -> PathBuf {
    let root = find_project_root(project_root);
    let mut hasher = Sha256::new();
    hasher.update(root.to_string_lossy().as_bytes());
    let digest = hasher.finalize();
    // Hex-encode manually to avoid a `hex` crate dependency.
    let mut hex = String::with_capacity(64);
    for byte in digest.iter() {
        hex.push_str(&format!("{byte:02x}"));
    }
    std::env::temp_dir().join(format!("fspec-reverse-{}.json", &hex[..12]))
}

/// Returns true if a session file exists for the project root. Mirrors
/// `sessionExists` (`src/utils/reverse-session.ts:31-39`).
pub fn session_exists(project_root: &Path) -> bool {
    session_path(project_root).exists()
}

/// Load and parse the session file. Returns `None` on any error (missing file
/// OR malformed JSON), mirroring the bare `catch` in `loadSession`
/// (`src/utils/reverse-session.ts:41-49`).
pub fn load_session(project_root: &Path) -> Option<ReverseSession> {
    let path = session_path(project_root);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Serialize and write the session file with 2-space indentation. Mirrors
/// `saveSession` (`src/utils/reverse-session.ts:51-59`).
pub fn save_session(project_root: &Path, session: &ReverseSession) -> std::io::Result<()> {
    let path = session_path(project_root);
    let json = serde_json::to_string_pretty(session).unwrap_or_else(|_| "{}".to_string());
    std::fs::write(path, json)
}

/// Delete the session file, swallowing a missing-file error. Mirrors
/// `deleteSession` (`src/utils/reverse-session.ts:61-68`).
pub fn delete_session(project_root: &Path) {
    let path = session_path(project_root);
    let _ = std::fs::remove_file(path);
}

/// Build a fresh session. Mirrors `createSession`
/// (`src/utils/reverse-session.ts:70-83`).
pub fn create_session(
    phase: &str,
    gaps: GapAnalysis,
    strategy: Option<String>,
    strategy_name: Option<String>,
) -> ReverseSession {
    ReverseSession {
        phase: phase.to_string(),
        strategy,
        strategy_name,
        current_step: None,
        total_steps: None,
        gaps,
        completed: None,
        timestamp: iso8601_now(),
    }
}

/// Transition a session into the executing phase with a chosen strategy.
/// Mirrors `setStrategy` (`src/utils/reverse-session.ts:96-111`).
pub fn set_strategy(
    mut session: ReverseSession,
    strategy: &str,
    strategy_name: &str,
    total_steps: u64,
) -> ReverseSession {
    session.phase = "executing".to_string();
    session.strategy = Some(strategy.to_string());
    session.strategy_name = Some(strategy_name.to_string());
    session.current_step = Some(1);
    session.total_steps = Some(total_steps);
    session.timestamp = iso8601_now();
    session
}

/// Advance to the next step. Mirrors `incrementStep`
/// (`src/utils/reverse-session.ts:113-120`): `currentStep = (currentStep ?? 1) + 1`.
pub fn increment_step(mut session: ReverseSession) -> ReverseSession {
    let current = session.current_step.unwrap_or(1);
    session.current_step = Some(current + 1);
    session.timestamp = iso8601_now();
    session
}

/// Returns true when all steps are finished. Mirrors `validateCompletion`
/// (`src/utils/reverse-session.ts:122-127`): requires both counters present
/// and `currentStep >= totalSteps`.
pub fn validate_completion(session: &ReverseSession) -> bool {
    match (session.current_step, session.total_steps) {
        (Some(current), Some(total)) if current > 0 && total > 0 => current >= total,
        _ => false,
    }
}
