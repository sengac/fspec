//! BUG-612: the ledger of environment variables profiles wrote.
//!
//! Feature: spec/features/deleting-a-profile-removes-the-environment-it-set.feature
//!
//! Each variable a profile's environment writes holds a stack of layers,
//! one per profile: the value the variable had just before that profile
//! wrote it, and the value it wrote (`None` = removed). Deleting a profile
//! removes its layer. When its write is still the variable's current value
//! the variable goes back to the layer's prior value; when something wrote
//! the variable since, the variable is left alone; the layer above it takes
//! over its prior, so a deleted profile's value is never restored later.

use std::collections::HashMap;
use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard, OnceLock};

struct Layer {
    profile: String,
    prior: Option<OsString>,
    written: Option<OsString>,
}

fn ledger() -> MutexGuard<'static, HashMap<String, Vec<Layer>>> {
    static LEDGER: OnceLock<Mutex<HashMap<String, Vec<Layer>>>> = OnceLock::new();
    let mutex = LEDGER.get_or_init(|| Mutex::new(HashMap::new()));
    // Every critical section is a single push/remove, so a poisoned ledger
    // is still consistent.
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// The key of `profile` of `provider`.
pub(crate) fn owner(provider: &str, profile: &str) -> String {
    format!("{provider}:{profile}")
}

/// Write `value` (`None` removes) to `var` for `owner`, recording the layer.
pub(crate) fn write(owner: &str, var: &str, value: Option<&str>) {
    let mut ledger = ledger();
    let layers = ledger.entry(var.to_string()).or_default();
    let prior = std::env::var_os(var);
    // A profile that writes again replaces its own layer (keeping the prior
    // it overwrote first when it is still on top).
    let prior = match layers.last() {
        Some(top) if top.profile == owner && prior == top.written => {
            layers.pop().map_or(prior, |top| top.prior)
        }
        _ => {
            remove_layer(layers, owner);
            prior
        }
    };
    match value {
        Some(value) => std::env::set_var(var, value),
        None => std::env::remove_var(var),
    }
    layers.push(Layer {
        profile: owner.to_string(),
        prior,
        written: value.map(OsString::from),
    });
}

/// Remove every layer of `owner`, restoring what it overwrote where its
/// write is still the current value.
pub(crate) fn forget_owner(owner: &str) {
    let mut ledger = ledger();
    for (var, layers) in ledger.iter_mut() {
        let Some(removed) = remove_layer(layers, owner) else {
            continue;
        };
        if std::env::var_os(var) == removed.written {
            match &removed.prior {
                Some(value) => std::env::set_var(var, value),
                None => std::env::remove_var(var),
            }
        }
    }
    ledger.retain(|_, layers| !layers.is_empty());
}

/// Remove `owner`'s layer; the layer above it, which recorded the owner's
/// value as its prior, takes over the owner's prior instead. The removed
/// layer is returned only when it was the top one (the variable's current
/// writer).
fn remove_layer(layers: &mut Vec<Layer>, owner: &str) -> Option<Layer> {
    let index = layers.iter().position(|layer| layer.profile == owner)?;
    let removed = layers.remove(index);
    match layers.get_mut(index) {
        Some(above) => {
            above.prior = removed.prior;
            None
        }
        None => Some(removed),
    }
}

/// PROV-133: a credential delete unset `var` whatever its origin; no layer
/// may bring a value back.
pub(crate) fn forget_var(var: &str) {
    ledger().remove(var);
}
