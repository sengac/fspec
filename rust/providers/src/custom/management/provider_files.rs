//! Custom provider config validation, connection testing and templates (split from management.rs by REFAC-030).

use std::path::{Path, PathBuf};

use serde_json::json;

use super::{CustomErrorExt, ProviderTestResult};
use crate::custom::config::ProviderConfig;
use crate::error::ProviderError;

/// Validate a discovered custom provider's JSON config. Re-parses the
/// config from disk through [`ProviderConfig::from_file`] and performs
/// PROV-067-specific checks (facade OR script required; api_key_env_var
/// or auth must be present when facade is null).
pub fn validate_provider_config(name: &str) -> Result<(), ProviderError> {
    let config_path = find_provider_config_path(name)?;
    // PROV-067: An early structural scan so we can surface the missing
    // `facade` field before the stricter schema validator bails on
    // something else (e.g. a missing script).
    let raw = std::fs::read_to_string(&config_path).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("failed to read {}: {}", config_path.display(), e),
        )
    })?;
    let parsed: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("invalid JSON in {}: {}", config_path.display(), e),
        )
    })?;
    let has_facade = parsed
        .get("facade")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    let has_script = parsed
        .get("script")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    if !has_facade && !has_script {
        return Err(ProviderError::config(
            "manager",
            format!(
                "provider '{name}' is missing the required 'facade' field \
                 (or a 'script' path when facade is null)"
            ),
        ));
    }
    // Fall through to the stricter schema validator.
    let _ = ProviderConfig::from_file(&config_path).map_err(CustomErrorExt::to_provider_error)?;
    Ok(())
}

/// Probe the custom provider's `base_url` by issuing a `GET /v1/models`
/// and comparing the returned model IDs against the config's models.
pub async fn test_provider_connection(name: &str) -> Result<ProviderTestResult, ProviderError> {
    let config_path = find_provider_config_path(name)?;
    let cfg = ProviderConfig::from_file(&config_path).map_err(CustomErrorExt::to_provider_error)?;
    let url = format!("{}/models", cfg.base_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .get(&url)
        .send()
        .await
        .map_err(|e| ProviderError::api("manager", format!("{url} unreachable: {e}")))?;
    let status = response.status().as_u16();
    let body: serde_json::Value = response.json().await.unwrap_or(serde_json::Value::Null);
    let remote_ids: Vec<String> = body
        .get("data")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|entry| {
                    entry
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(ToString::to_string)
                })
                .collect()
        })
        .unwrap_or_default();
    let matched_models: Vec<String> = cfg
        .models
        .values()
        .map(|m| m.id.clone())
        .filter(|id| remote_ids.contains(id))
        .collect();
    Ok(ProviderTestResult {
        reachable: (200..300).contains(&status),
        status_code: Some(status),
        matched_models,
    })
}

/// Create a new `.fspec/providers/<name>.json` under `project_root`
/// from a named template. Currently supported template: `"openai-compatible"`.
pub fn init_provider_template(
    project_root: &Path,
    name: &str,
    template: &str,
) -> Result<PathBuf, ProviderError> {
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        || !name
            .chars()
            .next()
            .map(|c| c.is_ascii_lowercase())
            .unwrap_or(false)
    {
        return Err(ProviderError::config(
            "manager",
            format!("invalid provider slug '{name}'; must match ^[a-z][a-z0-9-]*$"),
        ));
    }
    let dir = project_root.join(".fspec").join("providers");
    std::fs::create_dir_all(&dir).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("failed to create {}: {}", dir.display(), e),
        )
    })?;
    let body = match template {
        "openai-compatible" => openai_compatible_template(name),
        other => {
            return Err(ProviderError::config(
                "manager",
                format!("unknown template '{other}' (supported: openai-compatible)"),
            ));
        }
    };
    let path = dir.join(format!("{name}.json"));
    let serialized = serde_json::to_string_pretty(&body).map_err(|e| {
        ProviderError::config("manager", format!("failed to serialize template: {e}"))
    })?;
    std::fs::write(&path, serialized).map_err(|e| {
        ProviderError::config(
            "manager",
            format!("failed to write {}: {}", path.display(), e),
        )
    })?;
    Ok(path)
}

/// Locate the JSON config file for `name` across user-global and
/// project-local search paths.
fn find_provider_config_path(name: &str) -> Result<PathBuf, ProviderError> {
    let project_dir = std::env::current_dir()
        .map(|cwd| {
            cwd.join(".fspec")
                .join("providers")
                .join(format!("{name}.json"))
        })
        .ok();
    if let Some(ref p) = project_dir {
        if p.is_file() {
            return Ok(p.clone());
        }
    }

    let home = std::env::var("FSPEC_HOME")
        .ok()
        .map(PathBuf::from)
        .and_then(|p| {
            if p.file_name().map(|n| n == "credentials").unwrap_or(false) {
                p.parent().map(Path::to_path_buf)
            } else {
                Some(p)
            }
        })
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join(".fspec"))
        });
    if let Some(base) = home {
        let global = base.join("providers").join(format!("{name}.json"));
        if global.is_file() {
            return Ok(global);
        }
    }
    Err(ProviderError::config(
        "manager",
        format!("custom provider config for '{name}' not found"),
    ))
}

/// Build the JSON skeleton for the `openai-compatible` template.
fn openai_compatible_template(name: &str) -> serde_json::Value {
    let env_var = format!("{}_API_KEY", name.to_uppercase().replace('-', "_"));
    json!({
        "name": name,
        "display_name": format!("Custom {}", name),
        "facade": "openai",
        "base_url": "https://api.example.com/v1",
        "api_key_env_var": env_var,
        "models": {
            "default-model": {
                "id": "default-model",
                "context_window": 200000,
                "max_output_tokens": 4096
            }
        },
        "tool_style": "openai",
        "api_style": "openai_chat"
    })
}
