#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]
//! Shared helpers for PROV-065 custom provider system prompt facade tests.
//!
//! Included via `#[path = "custom_system_prompts_test_helpers.rs"] mod helpers;`.

use std::sync::Arc;

use rhai::{Dynamic, Engine, Map, AST};

use codelet_providers::oauth::building_blocks::register_all_modules;
use codelet_providers::oauth::engine::build_sandboxed_engine;

/// Build a sandboxed engine with all default PROV-060 modules registered.
pub fn test_engine() -> Arc<Engine> {
    Arc::new(build_sandboxed_engine(register_all_modules()))
}

/// Compile a Rhai script source string into an `Arc<AST>` using the
/// sandboxed engine. Panics on parse failure — tests are red-phase OK to
/// surface compilation errors loudly.
pub fn compile_script(engine: &Engine, source: &str) -> Arc<AST> {
    let ast = engine.compile(source).expect("compile rhai script");
    Arc::new(ast)
}

/// Build a minimal `config` Dynamic map matching the shape that
/// `RhaiCustomProvider::config_dynamic` produces. Only includes `name`
/// so the scripts can identify themselves if needed.
pub fn config_dynamic(name: &str) -> Dynamic {
    let mut map = Map::new();
    map.insert("name".into(), Dynamic::from(name.to_string()));
    Dynamic::from_map(map)
}
