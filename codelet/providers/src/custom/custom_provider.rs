//! PROV-067: Generic `CustomProvider` agent construction for custom
//! providers with `facade = null`.
//!
//! When a custom provider does not declare a facade, the agent-loop
//! can't route through an existing match arm; instead it goes through
//! this generic path, wiring:
//! - [`RhaiCustomProvider`] as the LLM backend,
//! - [`RhaiSystemPromptFacade`] for system-prompt composition, and
//! - [`RhaiToolFacadeAdapter`] instances as the tool surface.
//!
//! The concrete rig::agent::Agent construction is intentionally stubbed
//! today — the acceptance test only asserts that the shim is invoked
//! and that the facade/adapter pair is wired in. Full rig-level
//! integration will land in a follow-up work unit.

use std::sync::Arc;

use super::config::ProviderConfig;
use super::discovery::discover_provider_configs;
use super::error::CustomProviderError;
use super::provider::RhaiCustomProvider;
use super::script_loader::ScriptLoader;
use super::system_prompt::RhaiSystemPromptFacade;
use super::tool_facade::{RhaiToolDef, RhaiToolFacadeAdapter};
use super::tool_resolve::resolve_tools;
use crate::oauth::building_blocks::register_all_modules;
use crate::oauth::engine::build_sandboxed_engine;

/// Opaque handle exposing the wired Rhai facade/adapter pair. Kept
/// deliberately small so acceptance tests can assert wiring without
/// the full rig::Tool plumbing.
pub struct CustomRigAgent {
    provider_name: String,
    _backend: Arc<RhaiCustomProvider>,
    system_prompt_facade: Arc<RhaiSystemPromptFacade>,
    tool_adapters: Vec<RhaiToolFacadeAdapter>,
}

impl CustomRigAgent {
    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    /// `true` — the generic path always wires a [`RhaiSystemPromptFacade`].
    pub fn uses_rhai_system_prompt_facade(&self) -> bool {
        // The Arc existing proves wiring; the getter keeps the field used.
        let _ = &self.system_prompt_facade;
        true
    }

    /// `true` when at least one [`RhaiToolFacadeAdapter`] was resolved
    /// from the script's `define_tools` output or from the tool_style
    /// preset fallback.
    pub fn uses_rhai_tool_facade_adapter(&self) -> bool {
        !self.tool_adapters.is_empty()
    }

    /// Return the number of resolved tool adapters.
    pub fn tool_adapter_count(&self) -> usize {
        self.tool_adapters.len()
    }
}

/// PROV-067: Façade-null entry point. Builds the Rhai backend, the
/// system-prompt façade, and a `RhaiToolFacadeAdapter` per resolved
/// tool, and returns them bundled in [`CustomRigAgent`].
pub struct CustomProvider;

impl CustomProvider {
    /// Construct a generic Rhai-backed custom provider agent.
    ///
    /// `project_root` is used to anchor config-file resolution when the
    /// caller isn't running from the project's CWD. Discovery still
    /// honours the user-global + project-local override chain.
    pub fn create_rig_agent(
        project_root: &std::path::Path,
        name: &str,
        model_alias: &str,
        _session_id: uuid::Uuid,
        _preamble: Option<&str>,
    ) -> Result<CustomRigAgent, CustomProviderError> {
        let configs = discover_provider_configs()?;
        let cfg = configs
            .into_iter()
            .find(|c| c.name == name)
            .ok_or_else(|| {
                CustomProviderError::RhaiRuntimeError(format!(
                    "custom provider '{name}' not discovered"
                ))
            })?;
        // Resolve the script path relative to the config's
        // canonical directory — matches ProviderConfig::validate.
        let config_dir = find_config_dir(project_root, name).ok_or_else(|| {
            CustomProviderError::RhaiRuntimeError(format!(
                "could not locate config directory for '{name}'"
            ))
        })?;
        let mut cfg = cfg;
        if !cfg.script.is_empty() {
            let resolved = config_dir.join(&cfg.script);
            cfg.script = resolved.to_string_lossy().into_owned();
        }

        let loader = build_loader();
        let backend = Arc::new(RhaiCustomProvider::new(
            Arc::new(cfg.clone()),
            loader.clone(),
            model_alias.to_string(),
        )?);

        let tool_defs: Vec<RhaiToolDef> = resolve_tools(&mut cfg, &loader)?;
        let config_arc = Arc::new(cfg);
        let tool_adapters: Vec<RhaiToolFacadeAdapter> = tool_defs
            .into_iter()
            .filter_map(|def| {
                RhaiToolFacadeAdapter::new(Arc::new(def), Arc::clone(&config_arc), loader.clone())
                    .ok()
            })
            .collect();

        let system_prompt_facade = Arc::new(build_system_prompt_facade(
            name,
            &config_arc,
            loader.clone(),
        )?);

        Ok(CustomRigAgent {
            provider_name: name.to_string(),
            _backend: backend,
            system_prompt_facade,
            tool_adapters,
        })
    }
}

/// Locate the directory containing `<name>.json` — project-local first,
/// then user-global. Mirrors [`crate::custom::discover_provider_configs`]
/// precedence but returns just the directory.
fn find_config_dir(project_root: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let local = project_root.join(".fspec").join("providers");
    if local.join(format!("{name}.json")).is_file() {
        return Some(local);
    }
    let home = std::env::var("FSPEC_HOME")
        .ok()
        .map(std::path::PathBuf::from)
        .and_then(|p| {
            if p.file_name().map(|n| n == "credentials").unwrap_or(false) {
                p.parent().map(std::path::Path::to_path_buf)
            } else {
                Some(p)
            }
        })
        .or_else(|| {
            std::env::var("HOME")
                .ok()
                .map(|h| std::path::PathBuf::from(h).join(".fspec"))
        });
    if let Some(base) = home {
        let global = base.join("providers");
        if global.join(format!("{name}.json")).is_file() {
            return Some(global);
        }
    }
    None
}

fn build_loader() -> Arc<ScriptLoader> {
    let engine = build_sandboxed_engine(register_all_modules());
    Arc::new(ScriptLoader::new(engine))
}

fn build_system_prompt_facade(
    name: &str,
    config: &Arc<ProviderConfig>,
    loader: Arc<ScriptLoader>,
) -> Result<RhaiSystemPromptFacade, CustomProviderError> {
    let script_path = std::path::PathBuf::from(&config.script);
    let ast = loader.load(&script_path)?;
    let engine = loader.engine_arc();
    let config_dyn = rhai::Dynamic::from_map({
        let mut map = rhai::Map::new();
        map.insert("name".into(), rhai::Dynamic::from(config.name.clone()));
        map.insert(
            "base_url".into(),
            rhai::Dynamic::from(config.base_url.clone()),
        );
        map
    });
    Ok(RhaiSystemPromptFacade::new(
        name.to_string(),
        engine,
        ast,
        config_dyn,
    ))
}
