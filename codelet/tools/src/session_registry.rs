//! Per-session handler registry — generic storage for session-keyed values.
//!
//! Eliminates the repeated `Lazy<RwLock<HashMap<Uuid, T>>>` boilerplate
//! previously duplicated across `tool_progress.rs`, `tool_pause.rs`,
//! `bridge_handler.rs`, `bash.rs`, and 8+ other handler modules.
//!
//! # Usage
//!
//! ```ignore
//! use crate::session_registry::SessionRegistry;
//!
//! static MY_HANDLERS: Lazy<SessionRegistry<MyHandler>> =
//!     Lazy::new(SessionRegistry::new);
//!
//! MY_HANDLERS.set(session_id, Some(handler));   // register
//! MY_HANDLERS.set(session_id, None);            // unregister
//! let h = MY_HANDLERS.get(&session_id);         // clone out
//! let ok = MY_HANDLERS.has(&session_id);        // check existence
//! ```

use std::collections::HashMap;
use std::sync::RwLock;

use uuid::Uuid;

/// Thread-safe per-session value store keyed by `Uuid`.
///
/// Wraps `RwLock<HashMap<Uuid, T>>` with ergonomic accessors that
/// silently handle lock poisoning (returning `None` / `false` / no-op).
/// This matches the existing project convention of graceful degradation
/// when a concurrent thread panics while holding the lock.
pub struct SessionRegistry<T> {
    inner: RwLock<HashMap<Uuid, T>>,
}

impl<T> SessionRegistry<T> {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }

    /// Insert or remove a value for `session_id`.
    ///
    /// - `Some(value)` → inserts (overwrites any previous entry).
    /// - `None`        → removes the entry if present.
    pub fn set(&self, session_id: Uuid, value: Option<T>) {
        if let Ok(mut guard) = self.inner.write() {
            match value {
                Some(v) => {
                    guard.insert(session_id, v);
                }
                None => {
                    guard.remove(&session_id);
                }
            }
        }
    }

    /// Check whether a value is registered for `session_id`.
    pub fn has(&self, session_id: &Uuid) -> bool {
        self.inner
            .read()
            .map(|guard| guard.contains_key(session_id))
            .unwrap_or(false)
    }

    /// Remove the entry for `session_id` (no-op if absent or lock poisoned).
    pub fn remove(&self, session_id: &Uuid) {
        if let Ok(mut guard) = self.inner.write() {
            guard.remove(session_id);
        }
    }

    /// Remove all entries (useful for test cleanup).
    #[cfg(test)]
    pub fn clear_all(&self) {
        if let Ok(mut guard) = self.inner.write() {
            guard.clear();
        }
    }

    /// Borrow the value for `session_id` inside the read lock and apply `f`.
    ///
    /// Returns `None` if the session has no entry or the lock is poisoned.
    /// Prefer this over [`get`](Self::get) when you only need a temporary
    /// reference (avoids cloning).
    pub fn with<R>(&self, session_id: &Uuid, f: impl FnOnce(&T) -> R) -> Option<R> {
        self.inner
            .read()
            .ok()
            .and_then(|guard| guard.get(session_id).map(f))
    }

    /// Get or create an entry using `default`, then apply `f` to the value.
    ///
    /// Takes a write lock. Used by `clear_bash_abort` which lazily inserts a
    /// fresh `Arc<AtomicBool>` if the session has no entry yet.
    pub fn get_or_insert_with<R>(
        &self,
        session_id: Uuid,
        default: impl FnOnce() -> T,
        f: impl FnOnce(&T) -> R,
    ) -> Option<R> {
        self.inner
            .write()
            .ok()
            .map(|mut guard| {
                let entry = guard.entry(session_id).or_insert_with(default);
                f(entry)
            })
    }
}

impl<T: Clone> SessionRegistry<T> {
    /// Clone the value for `session_id` out of the registry.
    ///
    /// Returns `None` if the session has no entry or the lock is poisoned.
    pub fn get(&self, session_id: &Uuid) -> Option<T> {
        self.inner
            .read()
            .ok()
            .and_then(|guard| guard.get(session_id).cloned())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::Arc;

    static REG: Lazy<SessionRegistry<Arc<String>>> = Lazy::new(SessionRegistry::new);

    #[test]
    fn test_set_and_get() {
        let sid = Uuid::new_v4();
        REG.set(sid, Some(Arc::new("hello".to_string())));
        assert_eq!(REG.get(&sid).unwrap().as_str(), "hello");
        REG.remove(&sid);
    }

    #[test]
    fn test_has_and_remove() {
        let sid = Uuid::new_v4();
        assert!(!REG.has(&sid));
        REG.set(sid, Some(Arc::new("x".to_string())));
        assert!(REG.has(&sid));
        REG.remove(&sid);
        assert!(!REG.has(&sid));
    }

    #[test]
    fn test_set_none_removes() {
        let sid = Uuid::new_v4();
        REG.set(sid, Some(Arc::new("y".to_string())));
        REG.set(sid, None);
        assert!(!REG.has(&sid));
    }

    #[test]
    fn test_with_borrows_in_place() {
        let sid = Uuid::new_v4();
        REG.set(sid, Some(Arc::new("borrow me".to_string())));
        let len = REG.with(&sid, |v| v.len());
        assert_eq!(len, Some(9));
        REG.remove(&sid);
    }

    #[test]
    fn test_with_returns_none_for_missing() {
        let sid = Uuid::new_v4();
        let result = REG.with(&sid, |_| 42);
        assert!(result.is_none());
    }

    #[test]
    fn test_get_or_insert_with() {
        let reg: SessionRegistry<Arc<String>> = SessionRegistry::new();
        let sid = Uuid::new_v4();

        // First call inserts
        let len = reg.get_or_insert_with(sid, || Arc::new("default".to_string()), |v| v.len());
        assert_eq!(len, Some(7));

        // Second call reuses existing
        let len = reg.get_or_insert_with(sid, || Arc::new("other".to_string()), |v| v.len());
        assert_eq!(len, Some(7)); // still "default", not "other"
    }
}
