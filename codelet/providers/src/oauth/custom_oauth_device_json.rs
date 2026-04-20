//! JSON-envelope wrappers around [`custom_oauth_device`] — PROV-088.
//!
//! The NAPI layer cannot depend on `rhai` directly, so this module
//! bridges the Rhai `Map` boundary: it loads the scripted provider,
//! invokes `ScriptedDeviceFlow`, and returns JSON strings.

use anyhow::{anyhow, Result};
use rhai::{Dynamic, Map};

use super::custom_oauth::{load_scripted_provider_for, map_to_json_string};
use super::custom_oauth_device::{persist_on_success, ScriptedDeviceFlow};
use super::json_convert::json_value_to_dynamic;

/// Load the scripted provider, call `auth_start` / legacy, return the
/// authorization metadata as a JSON string.
pub async fn custom_oauth_device_start_json(provider_name: &str) -> Result<String> {
    let provider = load_scripted_provider_for(provider_name)?;
    let flow = ScriptedDeviceFlow::new(&provider);
    let map = flow.start().await?;
    map_to_json_string(&map)
}

/// Load the scripted provider, decode the device-data JSON envelope,
/// call `auth_poll` / legacy, persist tokens on success, and return
/// the poll result as a JSON string.
pub async fn custom_oauth_device_poll_json(
    provider_name: &str,
    device_data_json: &str,
) -> Result<String> {
    let provider = load_scripted_provider_for(provider_name)?;
    let flow = ScriptedDeviceFlow::new(&provider);
    let device_value: serde_json::Value = serde_json::from_str(device_data_json)
        .map_err(|e| anyhow!("parse device_data_json: {e}"))?;
    let device_dyn: Dynamic = json_value_to_dynamic(&device_value);
    let device_map: Map = device_dyn
        .try_cast::<Map>()
        .ok_or_else(|| anyhow!("device_data_json must be a JSON object"))?;
    let result = flow.poll(device_map).await?;
    persist_on_success(provider_name, &result)?;
    map_to_json_string(&result)
}
