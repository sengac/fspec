//! REFAC-028: ProviderManager and model-limit resolution split into small modules.
//!
//! Feature: spec/features/provider-manager-module-split.feature

use codelet_providers::{resolve_context_window, ProviderManager, ProviderType};
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `stem.rs` plus every Rust file under the `stem/` module directory.
fn module_files(src: &Path, stem: &str, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    out.push(src.join(format!("{stem}.rs")));
    let dir = src.join(stem);
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    Ok(())
}

#[test]
fn provider_manager_and_model_limit_source_files_stay_under_300_lines() -> TestResult {
    // @step Given the ProviderManager and model-limit source files in rust/providers/src
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    module_files(&src, "manager", &mut files)?;
    module_files(&src, "model_limits", &mut files)?;

    // @step When the line count of each file is measured
    let mut oversized = Vec::new();
    for file in &files {
        let lines = std::fs::read_to_string(file)?.lines().count();
        if lines >= 300 {
            oversized.push(format!("{} ({lines})", file.display()));
        }
    }

    // @step Then no file has 300 or more lines
    assert!(oversized.is_empty(), "oversized files: {oversized:?}");
    Ok(())
}

#[test]
fn the_provider_manager_and_model_limit_surface_is_unchanged() {
    // @step Given code that names ProviderManager, ProviderType and resolve_context_window through codelet_providers
    let manager = ProviderManager::for_testing(ProviderType::Claude, Some(150_000), Some(6_000));

    // @step When a Claude manager with registry limits resolves its context window and output limit
    let context = manager.context_window();
    let output = manager.max_output_tokens();
    let resolved = resolve_context_window(Some(150_000), None, &NoCeiling);

    // @step Then it reports the registry values
    assert_eq!((context, output), (Some(150_000), Some(6_000)));
    assert_eq!(resolved, Some(150_000));
}

struct NoCeiling;

impl codelet_providers::ModelLimitsResolver for NoCeiling {
    fn default_context_window(&self) -> Option<usize> {
        Some(1)
    }

    fn default_max_output_tokens(&self) -> Option<usize> {
        Some(1)
    }
}
