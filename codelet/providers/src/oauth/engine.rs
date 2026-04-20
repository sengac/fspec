//! Sandboxed Rhai Engine Factory (PROV-060)
//!
//! Creates a Rhai `Engine` via `Engine::new_raw()` with:
//! - Operation limits (50,000 ops)
//! - Call depth limits (32 levels)
//! - String/array/map size limits
//! - No standard library (sandboxed)
//! - Extensible module registration

use rhai::packages::{
    BasicArrayPackage, BasicMapPackage, CorePackage, LogicPackage, MoreStringPackage, Package,
};
use rhai::{Engine, Module};

/// Maximum operations before the engine terminates a script.
pub const MAX_OPERATIONS: u64 = 50_000;

/// Maximum call depth before the engine terminates a script.
const MAX_CALL_LEVELS: usize = 32;

/// Maximum string size (1 MB).
const MAX_STRING_SIZE: usize = 1_048_576;

/// Maximum array size (10,000 elements).
const MAX_ARRAY_SIZE: usize = 10_000;

/// Maximum map size (10,000 entries).
const MAX_MAP_SIZE: usize = 10_000;

/// Maximum top-level expression depth. Matches Rhai's release-profile
/// default so nested map/array literals in `define_tools` scripts
/// compile identically under `cargo test` (debug) and release builds.
const MAX_EXPR_DEPTH: usize = 64;

/// Maximum expression depth inside function bodies.
const MAX_FUNCTION_EXPR_DEPTH: usize = 32;

/// A named Rhai module to be registered with the engine.
pub struct RhaiModule {
    /// Namespace under which the module is registered (e.g. "http", "oauth").
    pub name: String,
    /// The Rhai module containing registered functions.
    pub module: Module,
}

/// Build a sandboxed Rhai engine with the given modules registered.
///
/// The engine is created via `Engine::new_raw()` (no standard library)
/// and configured with safety limits. Modules are registered as static
/// namespaces (e.g. `http::post`, `oauth::generate_pkce`).
///
/// Accepts an extensible module list so PROV-061 can add `time::`, `env::` etc.
pub fn build_sandboxed_engine(modules: Vec<RhaiModule>) -> Engine {
    let mut engine = Engine::new_raw();

    // Safety limits
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    engine.set_max_string_size(MAX_STRING_SIZE);
    engine.set_max_array_size(MAX_ARRAY_SIZE);
    engine.set_max_map_size(MAX_MAP_SIZE);
    engine.set_max_expr_depths(MAX_EXPR_DEPTH, MAX_FUNCTION_EXPR_DEPTH);

    // PROV-066: enable `map.contains(key)` and other basic map operators
    // that custom `map_tool_params` scripts rely on. The raw engine has
    // no operators registered; the BasicMapPackage adds only map/obj
    // helpers without exposing filesystem or OS functions.
    //
    // PROV-095 regression fix: custom provider scripts (e.g.
    // `claude_rhai.rhai`) iterate `for msg in request.messages { … }`
    // in `build_request`, which requires Rhai's iterator adapters.
    // `Engine::new_raw()` on its own has no iterators registered, so
    // the for-loop surfaces as:
    //
    //   script 'build_request' failed:
    //   For loop expects iterable type (line N, position 16)
    //
    // Registering `CorePackage` pulls in `BasicIteratorPackage`
    // (for-loop support), `ArithmeticPackage` (int/float operators),
    // `BasicStringPackage` (string equality + concatenation), and
    // `BasicFnPackage` (`type_of(…)`). `BasicArrayPackage` adds
    // `.push()`, `.len()`, `.contains()`, etc. on arrays, which the
    // claude-rhai script uses to build `system_parts` / `conversation`
    // / `tool_list`. `LogicPackage` adds boolean helpers (`!`, bit
    // ops, and non-short-circuit `&`/`|`) that scripts commonly rely
    // on.
    //
    // None of these packages touch the filesystem, network, OS
    // processes, or environment — the sandbox guarantees (operation
    // cap, call depth cap, size caps, and the deliberate absence of
    // filesystem / OS / env modules) are unchanged.
    let core_package = CorePackage::new();
    core_package.register_into_engine(&mut engine);

    let array_package = BasicArrayPackage::new();
    array_package.register_into_engine(&mut engine);

    let logic_package = LogicPackage::new();
    logic_package.register_into_engine(&mut engine);

    let map_package = BasicMapPackage::new();
    map_package.register_into_engine(&mut engine);

    // PROV-095 regression fix (second occurrence): `CorePackage` bundles
    // `BasicStringPackage` which registers string equality, concatenation
    // and `type_of(…)` — but the `.len()` / `len(&str)` family for string
    // types lives in `MoreStringPackage` (`string_more.rs`). Without it,
    // any custom provider script calling `body.len()` on a `String` /
    // `ImmutableString` (as `claude_rhai.rhai::map_error` does on line
    // `if body.len() > 0 { … }`) fails with:
    //
    //   Configuration error: script missing required function
    //   'len (&str | ImmutableString | String)' (map_error)
    //
    // which is the exact error captured in the user-supplied screenshot.
    // `MoreStringPackage` is sandbox-safe: it adds string utility
    // functions (`len`, `pad`, `replace`, `split`, etc.) and touches no
    // filesystem / network / OS / environment APIs.
    let more_string_package = MoreStringPackage::new();
    more_string_package.register_into_engine(&mut engine);

    // Register each module as a static namespace
    for rhai_module in modules {
        engine.register_static_module(&rhai_module.name, rhai_module.module.into());
    }

    engine
}

/// Build a sandboxed engine with the default PROV-060 modules
/// (http, crypto, json, oauth).
pub fn build_default_engine() -> Engine {
    let modules = super::building_blocks::register_all_modules();
    build_sandboxed_engine(modules)
}

/// Build a sandboxed engine with the default PROV-060 modules plus a
/// provider-scoped `cred::` module (PROV-086).
///
/// Scripts evaluated in the returned engine can call
/// `cred::read(name)`, `cred::write(name, map)`, `cred::delete(name)`,
/// and `cred::path(name)`, but only when `name == provider_name` —
/// preventing script A from reading provider B's credential file.
pub fn build_provider_engine(provider_name: &str) -> Engine {
    let modules = super::building_blocks::register_all_modules_for_provider(provider_name);
    build_sandboxed_engine(modules)
}
