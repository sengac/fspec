//! REFAC-021: rpc-types split into per-surface modules.
//!
//! Feature: spec/features/rpc-types-module-split.feature

use codelet_rpc_types::{
    GitState, SessionInfo, StreamChunk, WorkUnitInfo, WorkspaceEntry, WorkspaceWorkUnits,
};
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            rust_files(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
    Ok(())
}

fn round_trip<T>(value: &T) -> Result<T, serde_json::Error>
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    serde_json::from_str(&serde_json::to_string(value)?)
}

#[test]
fn every_rpc_types_source_file_stays_under_300_lines() -> TestResult {
    // @step Given the rust/rpc-types/src directory
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    // @step When the line count of every Rust source file is measured
    let mut files = Vec::new();
    rust_files(&src, &mut files)?;
    let mut oversized = Vec::new();
    for file in &files {
        let lines = std::fs::read_to_string(file)?.lines().count();
        if lines >= 300 {
            oversized.push(format!("{} ({lines})", file.display()));
        }
    }

    // @step Then no file has 300 or more lines
    assert!(
        oversized.is_empty(),
        "oversized rpc-types files: {oversized:?}"
    );
    Ok(())
}

#[test]
fn the_public_surface_is_unchanged() -> TestResult {
    // @step Given code that names WorkUnitInfo, SessionInfo, StreamChunk, GitState, WorkspaceEntry and WorkspaceWorkUnits through codelet_rpc_types
    let unit = WorkUnitInfo {
        id: "RPC-1".into(),
        title: "t".into(),
        work_type: "story".into(),
        status: "backlog".into(),
        description: None,
        estimate: Some(3),
        epic: None,
        attachments: vec!["a.md".into()],
        last_state_change_at: None,
    };
    let session = SessionInfo {
        id: "s-1".into(),
        name: "n".into(),
        status: "idle".into(),
        project: "/p".into(),
        message_count: 2,
        provider_id: None,
        model_id: None,
        is_isolated: false,
        worktree_path: None,
        role: None,
        updated_at_ms: Some(7),
        work_unit_focus: Default::default(),
    };
    let entry = WorkspaceEntry {
        path: "/p".into(),
        name: "p".into(),
        start: true,
    };
    let push = WorkspaceWorkUnits {
        workspace: "/p".into(),
        units: vec![unit.clone()],
    };
    let git = GitState::default();
    let chunk = StreamChunk::text("hello".to_string());

    // @step When it is compiled and the values round-trip through JSON
    // @step Then each value deserializes back equal to the original
    assert_eq!(round_trip(&unit)?, unit);
    assert_eq!(round_trip(&session)?, session);
    assert_eq!(round_trip(&entry)?, entry);
    assert_eq!(round_trip(&push)?, push);
    assert_eq!(round_trip(&git)?, git);
    let chunk_json = serde_json::to_value(&chunk)?;
    assert_eq!(serde_json::to_value(round_trip(&chunk)?)?, chunk_json);
    Ok(())
}
