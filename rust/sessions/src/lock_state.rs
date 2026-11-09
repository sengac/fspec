//! BUG-369: fallible access to session state behind a `std::sync::RwLock`
//! — a poisoned lock is an error naming the state, never skipped or read
//! as "nothing there".

use std::sync::RwLock;

/// Clone the state out of `lock`.
pub fn read_state<T: Clone>(lock: &RwLock<T>, what: &str) -> Result<T, String> {
    lock.read()
        .map(|guard| guard.clone())
        .map_err(|_| format!("the {what} lock is poisoned"))
}

/// Mutate the state behind `lock`, returning what `update` returns.
pub fn write_state<T, R>(
    lock: &RwLock<T>,
    what: &str,
    update: impl FnOnce(&mut T) -> R,
) -> Result<R, String> {
    lock.write()
        .map(|mut guard| update(&mut guard))
        .map_err(|_| format!("the {what} lock is poisoned"))
}
