#![allow(clippy::unwrap_used, clippy::expect_used)]
//! REFAC-031: the OpenAI provider split into small modules.
//!
//! Feature: spec/features/openai-provider-module-split.feature

#[path = "custom_test_helpers.rs"]
mod helpers;

use codelet_providers::openai::OpenAIProvider;
use codelet_providers::{LlmProvider, ProviderAdapter};
use helpers::EnvGuard;
use serial_test::serial;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `openai.rs` plus every Rust file under the `openai/` module directory.
fn openai_module_files(src: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = vec![src.join("openai.rs")];
    for entry in std::fs::read_dir(src.join("openai"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(files)
}

#[test]
fn openai_provider_files_stay_under_300_lines() -> TestResult {
    // @step Given the OpenAI provider source and every module split from it in rust/providers
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = openai_module_files(&src)?;
    assert!(files.len() > 1, "openai.rs has no split modules");

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
#[serial]
fn the_openai_provider_surface_is_unchanged() -> TestResult {
    // @step Given code that names OpenAIProvider through codelet_providers::openai
    let _streaming = EnvGuard::set("OPENAI_STREAMING", Path::new("false"));

    // @step When a provider is built for a local base URL with streaming disabled
    let provider = OpenAIProvider::from_api_key_with_options(
        "test-key",
        "local-model",
        Some("http://localhost:8000/v1"),
        None,
    )?;

    // @step Then it reports that base URL, a local endpoint, streaming disabled and the openai provider name
    assert_eq!(provider.base_url(), Some("http://localhost:8000/v1"));
    assert!(provider.is_local_endpoint());
    assert!(!provider.supports_streaming());
    assert_eq!(provider.provider_name(), "openai");
    assert_eq!(provider.model(), "local-model");
    Ok(())
}
