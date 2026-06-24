//! Rhai script loader with AST caching (PROV-062).
//!
//! [`ScriptLoader`] compiles `.rhai` files using the sandboxed engine from
//! PROV-060, caches the compiled `Arc<AST>` keyed by canonical path +
//! mtime, and validates that each compiled script defines the 7 required
//! provider-lifecycle functions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use rhai::{Engine, AST};

use super::error::CustomProviderError;
use crate::oauth::building_blocks::register_all_modules;
use crate::oauth::engine::build_sandboxed_engine;

/// The 7 required provider-lifecycle functions a custom provider script
/// must define after compilation. Order is deterministic so error
/// messages are stable.
const REQUIRED_FUNCTIONS: &[&str] = &[
    "build_request",
    "build_headers",
    "build_url",
    "parse_response",
    "parse_stream_chunk",
    "build_stream_request",
    "map_error",
];

/// Cache entry keyed by canonical path; tracks the `mtime` at which the
/// AST was compiled so we can invalidate on change.
struct CacheEntry {
    /// Modification time at compilation.
    mtime: SystemTime,
    /// Shared compiled AST.
    ast: Arc<AST>,
}

/// Compiles and caches Rhai scripts for custom providers.
///
/// Thread-safe: the cache sits behind a `Mutex`. The engine is `Arc`-shared
/// so multiple threads can `eval_ast` concurrently without contention on
/// the cache mutex itself.
pub struct ScriptLoader {
    engine: Arc<Engine>,
    cache: Mutex<HashMap<PathBuf, CacheEntry>>,
}

impl Default for ScriptLoader {
    fn default() -> Self {
        Self::with_default_engine()
    }
}

impl ScriptLoader {
    /// Build a script loader with the default sandboxed engine (PROV-060
    /// modules: http, crypto, json, oauth).
    pub fn with_default_engine() -> Self {
        let engine = build_sandboxed_engine(register_all_modules());
        Self::new(engine)
    }

    /// Build a script loader with a caller-supplied engine.
    ///
    /// Accepting a pre-built engine lets callers register additional
    /// sandboxed modules (e.g. PROV-061 `time::`, `env::`, `cred::`)
    /// before the loader takes ownership.
    pub fn new(engine: Engine) -> Self {
        Self {
            engine: Arc::new(engine),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Get the shared sandboxed engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Get a cloneable `Arc` handle to the shared sandboxed engine.
    ///
    /// Useful for code paths (e.g. `tokio::task::spawn_blocking`) that
    /// need to move the engine into a closure by value.
    pub fn engine_arc(&self) -> Arc<Engine> {
        Arc::clone(&self.engine)
    }

    /// Load, compile, and cache a `.rhai` file. Subsequent calls with the
    /// same path return the cached `Arc<AST>` unless the file's mtime has
    /// changed.
    pub fn load(&self, script_path: &Path) -> Result<Arc<AST>, CustomProviderError> {
        let canonical =
            std::fs::canonicalize(script_path).map_err(|source| CustomProviderError::Io {
                path: script_path.to_path_buf(),
                source,
            })?;
        let metadata = std::fs::metadata(&canonical).map_err(|source| CustomProviderError::Io {
            path: canonical.clone(),
            source,
        })?;
        let mtime = metadata
            .modified()
            .map_err(|source| CustomProviderError::Io {
                path: canonical.clone(),
                source,
            })?;

        // Fast path: cache hit with matching mtime.
        {
            // A poisoned mutex means another thread panicked while
            // holding the lock. The cache is best-effort — fall through
            // to recompute rather than propagating the panic.
            if let Ok(cache) = self.cache.lock() {
                if let Some(entry) = cache.get(&canonical) {
                    if entry.mtime == mtime {
                        return Ok(Arc::clone(&entry.ast));
                    }
                }
            }
        }

        // Slow path: read + compile + insert.
        let script =
            std::fs::read_to_string(&canonical).map_err(|source| CustomProviderError::Io {
                path: canonical.clone(),
                source,
            })?;

        let ast = self.engine.compile(&script).map_err(|e| {
            let position = e.1;
            let line = position.line().unwrap_or(1);
            let column = position.position().unwrap_or(0);
            tracing::debug!(path = %canonical.display(), line, column, "rhai parse error");
            CustomProviderError::RhaiParseError {
                path: canonical.clone(),
                line,
                column,
                message: e.0.to_string(),
            }
        })?;

        // Stash the source path on the AST so validators can surface the
        // path later (e.g. MissingFunction).
        let mut ast = ast;
        ast.set_source(canonical.to_string_lossy().as_ref());

        let arc_ast = Arc::new(ast);

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(
                canonical,
                CacheEntry {
                    mtime,
                    ast: Arc::clone(&arc_ast),
                },
            );
        }
        Ok(arc_ast)
    }

    /// Verify the compiled script defines every function in
    /// [`REQUIRED_FUNCTIONS`]. Returns the first missing function as a
    /// [`CustomProviderError::MissingFunction`].
    pub fn validate_required_functions(&self, ast: &AST) -> Result<(), CustomProviderError> {
        let defined: std::collections::HashSet<&str> =
            ast.iter_functions().map(|meta| meta.name).collect();

        let source_path = ast
            .source()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("<unknown>"));

        for required in REQUIRED_FUNCTIONS {
            if !defined.contains(*required) {
                return Err(CustomProviderError::MissingFunction {
                    path: source_path,
                    function: (*required).to_string(),
                });
            }
        }
        Ok(())
    }
}

impl std::fmt::Debug for ScriptLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let cache_len = self.cache.lock().map(|c| c.len()).unwrap_or_default();
        f.debug_struct("ScriptLoader")
            .field("cache_entries", &cache_len)
            .finish_non_exhaustive()
    }
}
