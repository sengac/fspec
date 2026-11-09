//! BUG-575: hold individual MockBackend calls and release them in any order.
//!
//! A gated method computes its answer when it is called (the backend read
//! happens then), records the call, and returns it only when the test
//! releases that call. Two in-flight calls can so resolve in reverse
//! order, deterministically: a test waits for the calls to be held
//! (`held`), then releases them one by one (`release`), draining the App
//! between releases. Ungated methods return at once, as before.

use std::collections::HashMap;

use parking_lot::Mutex;
use tokio::sync::oneshot;

#[derive(Default)]
struct Gate {
    /// Calls held, in arrival order; `None` once released.
    held: Vec<Option<oneshot::Sender<()>>>,
}

/// Per-method gates. `Default` gates nothing.
#[derive(Default)]
pub struct CallGates {
    gates: Mutex<HashMap<&'static str, Gate>>,
}

impl CallGates {
    /// Hold every later call of `method` until released.
    pub fn gate(&self, method: &'static str) {
        self.gates.lock().entry(method).or_default();
    }

    /// How many calls of `method` have arrived since it was gated
    /// (released ones included).
    pub fn held(&self, method: &str) -> usize {
        self.gates
            .lock()
            .get(method)
            .map_or(0, |gate| gate.held.len())
    }

    /// Release the `nth` call of `method` (0-based arrival order). Returns
    /// whether such a call was held.
    pub fn release(&self, method: &str, nth: usize) -> bool {
        self.gates
            .lock()
            .get_mut(method)
            .and_then(|gate| gate.held.get_mut(nth))
            .and_then(Option::take)
            .is_some_and(|sender| sender.send(()).is_ok())
    }

    /// Stop gating `method` and release every call it holds.
    pub fn open(&self, method: &str) {
        if let Some(gate) = self.gates.lock().remove(method) {
            for sender in gate.held.into_iter().flatten() {
                sender.send(()).unwrap_or(());
            }
        }
    }

    /// Called by a gated method after computing its answer: waits for the
    /// release when `method` is gated, otherwise returns at once.
    pub async fn pass(&self, method: &'static str) {
        let released = {
            let mut gates = self.gates.lock();
            let Some(gate) = gates.get_mut(method) else {
                return;
            };
            let (sender, released) = oneshot::channel();
            gate.held.push(Some(sender));
            released
        };
        // A dropped sender (the gates went away) releases the call too.
        released.await.unwrap_or(());
    }
}
