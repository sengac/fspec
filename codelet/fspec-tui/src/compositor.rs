//! Layered priority dispatcher (RPC-008 rules [8] [9] [10]).
//!
//! Feature: spec/features/fspec-tui-compositor.feature
//!
//! ~30 LoC core dispatcher per RPC-002 doc 09 §A.7 + §D.1. Stable
//! priority sort with FIFO tiebreak (newer registrations win), short-
//! circuit on `Consumed`, skip on `is_active() == false`, deferred
//! callbacks for self-removal, bottom-up render order, top-down
//! Action fan-out.

use crossterm::event::Event;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::components::{Action, Component, EventResult, Priority};

/// Wrapper holding a registration counter alongside each layer so the
/// stable-priority sort can break ties by registration order (FIFO with
/// newer-wins semantics — see rule [8]).
struct Layer {
    component: Box<dyn Component>,
    /// Higher counter values were pushed later and therefore win
    /// ties at the same priority during event dispatch.
    registration: u64,
}

/// Priority-sorted stack of [`Component`]s with FIFO tiebreak,
/// short-circuit dispatch, deferred-callback removal, bottom-up render,
/// and top-down action fan-out.
#[derive(Default)]
pub struct Compositor {
    layers: Vec<Layer>,
    next_registration: u64,
}

impl Compositor {
    /// Construct an empty Compositor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a Component onto the stack. Bumps the registration counter
    /// and re-sorts by priority (stable sort) so equal-priority layers
    /// retain registration order.
    pub fn push(&mut self, component: Box<dyn Component>) {
        let registration = self.next_registration;
        self.next_registration += 1;
        self.layers.push(Layer {
            component,
            registration,
        });
        // Stable sort by priority ascending; iter().rev() in dispatch
        // walks from highest priority down, FIFO inside each priority
        // band (newest pushed last → matched first).
        self.layers.sort_by_key(|l| l.component.priority());
    }

    /// Remove the most recently pushed layer regardless of priority.
    /// Returns the removed Component or `None` if empty.
    pub fn pop(&mut self) -> Option<Box<dyn Component>> {
        // "Most recently pushed" = highest registration counter.
        let idx = self
            .layers
            .iter()
            .enumerate()
            .max_by_key(|(_, l)| l.registration)?
            .0;
        Some(self.layers.remove(idx).component)
    }

    /// Remove the layer with the given id. Returns the removed
    /// Component or `None` if no match.
    pub fn remove(&mut self, id: &str) -> Option<Box<dyn Component>> {
        let idx = self.layers.iter().position(|l| l.component.id() == id)?;
        Some(self.layers.remove(idx).component)
    }

    /// Number of layers currently on the stack.
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// True iff the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// Snapshot of every layer's id, ordered by priority (ascending) +
    /// registration (ascending). Used by tests + introspection.
    pub fn layer_ids(&self) -> Vec<String> {
        self.layers
            .iter()
            .map(|l| l.component.id().to_string())
            .collect()
    }

    /// True iff any layer reports the given id.
    pub fn contains(&self, id: &str) -> bool {
        self.layers.iter().any(|l| l.component.id() == id)
    }

    /// Highest-priority layer's id, if any. FIFO tiebreak: the
    /// most-recently-pushed layer wins at equal priority.
    pub fn topmost_id(&self) -> Option<String> {
        self.topmost_index()
            .map(|i| self.layers[i].component.id().to_string())
    }

    /// Highest-priority layer's [`Priority`], if any.
    pub fn topmost_priority(&self) -> Option<Priority> {
        self.topmost_index()
            .map(|i| self.layers[i].component.priority())
    }

    fn topmost_index(&self) -> Option<usize> {
        let mut best: Option<(usize, Priority, u64)> = None;
        for (i, layer) in self.layers.iter().enumerate() {
            let p = layer.component.priority();
            let r = layer.registration;
            best = match best {
                Some((_, bp, br)) if (bp, br) >= (p, r) => best,
                _ => Some((i, p, r)),
            };
        }
        best.map(|(i, _, _)| i)
    }

    /// Dispatch an event in priority order (highest first), with FIFO
    /// tiebreak inside each priority band, skipping inactive layers,
    /// short-circuiting on `Consumed`. Returns the final EventResult so
    /// the App can run any deferred callback against the compositor.
    pub fn handle_event(&mut self, event: &Event) -> EventResult {
        // Walk the stack from highest priority + newest registration
        // down. The layers vec is sorted by priority ascending; reverse
        // iteration walks high-priority → low-priority. Inside each
        // priority band the stable sort kept registration order; we
        // reverse-iterate so the most recent registration is matched
        // first inside the band.
        let mut indices: Vec<usize> = (0..self.layers.len()).collect();
        indices.sort_by(|&a, &b| {
            let la = &self.layers[a];
            let lb = &self.layers[b];
            // Higher priority first; within equal priority, higher
            // registration counter (newer pushed) first.
            lb.component
                .priority()
                .cmp(&la.component.priority())
                .then_with(|| lb.registration.cmp(&la.registration))
        });
        for i in indices {
            let layer = &mut self.layers[i];
            if !layer.component.is_active() {
                continue;
            }
            let result = layer.component.handle_event(event);
            if result.is_consumed() {
                return result;
            }
        }
        EventResult::ignored()
    }

    /// Fan an Action across every layer in registration order
    /// (top-down). Returns the first non-None follow-up Action a layer
    /// produces, or None if every layer returned None.
    pub fn update(&mut self, action: Action) -> Option<Action> {
        let mut follow: Option<Action> = None;
        for layer in self.layers.iter_mut() {
            let next = layer.component.update(action.clone());
            if follow.is_none() && next.is_some() {
                follow = next;
            }
        }
        follow
    }

    /// Render every active layer bottom-up so highest priority paints
    /// LAST and ends up visually on top (rule [9]).
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        for layer in self.layers.iter_mut() {
            if !layer.component.is_active() {
                continue;
            }
            layer.component.render(area, buf);
        }
    }

    /// RPC-403: forward the REAL `Event::Paste(String)` through the
    /// normal `handle_event` layer chain so the topmost active modal
    /// receives ONE intact paste event (newlines + grapheme clusters
    /// preserved). Replaces the RPC-008 char-splitting stub that
    /// exploded pastes into synthetic `KeyCode::Char` events.
    ///
    /// Returns the layer chain's [`EventResult`] — `Ignored` when no
    /// layer consumed the paste, letting `App::handle_paste` fall back
    /// through the Navigator → AgentView input.
    pub fn handle_paste(&mut self, text: &str) -> EventResult {
        self.handle_event(&Event::Paste(text.to_string()))
    }
}
