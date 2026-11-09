//! REFAC-030: custom-provider config, management and limit tests split into small modules.
//!
//! Feature: spec/features/custom-provider-config-module-split.feature

use codelet_providers::custom::config::{AuthConfig, ModelDef, ProviderConfig};
use codelet_providers::custom::management::ProviderModelInfo;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// `stem.rs` plus every Rust file under the `stem/` module directory.
fn module_files(base: &Path, stem: &str, dir: &str, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    out.push(base.join(format!("{stem}.rs")));
    let dir = base.join(dir);
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
fn custom_provider_config_management_and_limit_test_files_stay_under_300_lines() -> TestResult {
    // @step Given the custom-provider config and management sources and the four limit test targets in rust/providers
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let custom = root.join("src").join("custom");
    let tests = root.join("tests");
    let mut files = Vec::new();
    module_files(&custom, "config", "config", &mut files)?;
    module_files(&custom, "management", "management", &mut files)?;
    for (stem, dir) in [
        ("rhai_rig_agent_keystone_tests", "rhai_rig_agent_keystone"),
        (
            "rhai_scripted_model_limits_tests",
            "rhai_scripted_model_limits",
        ),
        ("custom_http_lifecycle_tests", "custom_http_lifecycle"),
        (
            "bug_139_custom_provider_per_model_limits_test",
            "bug_139_per_model_limits",
        ),
    ] {
        module_files(&tests, stem, dir, &mut files)?;
    }

    // @step When the line count of each file and of every file split from it is measured
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
fn the_custom_provider_config_and_management_surface_is_unchanged() -> TestResult {
    // @step Given code that names ModelDef, AuthConfig, ProviderConfig and ProviderModelInfo through codelet_providers::custom
    let json = r#"{"name":"split-check","display_name":"Split","base_url":"http://x",
        "facade":"openai","models":{"m":{"id":"model-1"}}}"#;

    // @step When a provider config that omits model limits and auth is parsed
    let cfg: ProviderConfig = serde_json::from_str(json)?;
    let def: &ModelDef = cfg.models.get("m").ok_or("model m missing")?;
    let info = ProviderModelInfo {
        id: "m".to_string(),
        context_window: def.context_window.map(u32::try_from).transpose()?,
        max_output_tokens: def.max_output_tokens.map(u32::try_from).transpose()?,
        supports_tools: def.supports_tools,
        supports_streaming: def.supports_streaming,
        supports_thinking: def.supports_thinking,
        supports_vision: def.supports_vision,
    };

    // @step Then its model reports no context window and no max output tokens and its auth is the custom variant
    assert_eq!((info.context_window, info.max_output_tokens), (None, None));
    assert_eq!(
        cfg.auth,
        AuthConfig::Custom {
            credential_file: None
        }
    );
    Ok(())
}
