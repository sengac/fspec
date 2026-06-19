//! RPC-337 — Navigator event-translation helpers.
//!
//! Feature files:
//!   - spec/features/rpc012-board-agent-navigation.feature
//!   - spec/features/full-screen-model-selector.feature
//!
//! Extracted from `navigator.rs` so it stays under the 300-LoC ceiling
//! (RPC-013 source-shape rule). Hosts the per-mode-view key→Action
//! translation methods: ProviderSettings, ModelSelector, Blocklist.
//! Methods are `pub(crate)` so the sibling `navigator` module can call
//! them via `self.handle_*_event(...)`.

use crossterm::event::Event;

use crate::components::{Action, EventResult};
use crate::views::{BlocklistEvent, ModelSelectorEvent, ProviderSettingsEvent};

use super::navigator::Navigator;

impl Navigator {
    /// RPC-054: forward key events to the ProviderSettingsView and
    /// translate its `ProviderSettingsEvent` outcomes onto the action
    /// bus.
    pub(crate) fn handle_provider_settings_event(&mut self, event: &Event) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };
        match self.provider_settings.handle_key(*key) {
            ProviderSettingsEvent::Consumed => EventResult::consumed(),
            ProviderSettingsEvent::Ignored => EventResult::ignored(),
            ProviderSettingsEvent::Close => {
                if let Some(tx) = self.action_tx.as_ref() {
                    let _ = tx.send(Action::CloseProviderSettingsView);
                }
                EventResult::consumed()
            }
            ProviderSettingsEvent::Emit(action) => {
                if let Some(tx) = self.action_tx.as_ref() {
                    let _ = tx.send(action);
                }
                EventResult::consumed()
            }
            // RPC-337: Tab→SwitchToModels flips into the full-screen
            // ModelSelector mode-view via Action::OpenModelSelectorView.
            ProviderSettingsEvent::SwitchToModels => {
                if let Some(tx) = self.action_tx.as_ref() {
                    let _ = tx.send(Action::OpenModelSelectorView);
                }
                EventResult::consumed()
            }
        }
    }

    /// RPC-337: forward key events to the ModelSelectorView and translate
    /// its `ModelSelectorEvent` outcomes onto the action bus. Mirrors
    /// `handle_provider_settings_event`.
    pub(crate) fn handle_model_selector_event(&mut self, event: &Event) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };
        match self.model_selector.handle_key(*key) {
            ModelSelectorEvent::Consumed => EventResult::consumed(),
            ModelSelectorEvent::Ignored => EventResult::ignored(),
            ModelSelectorEvent::Close => {
                if let Some(tx) = self.action_tx.as_ref() {
                    let _ = tx.send(Action::CloseModelSelectorView);
                }
                EventResult::consumed()
            }
            ModelSelectorEvent::Emit(action) => {
                if let Some(tx) = self.action_tx.as_ref() {
                    let _ = tx.send(action);
                }
                EventResult::consumed()
            }
        }
    }

    /// RPC-056: forward key events to the BlocklistView and translate
    /// its `BlocklistEvent` outcomes onto the action bus. Mirrors
    /// `handle_provider_settings_event`.
    pub(crate) fn handle_blocklist_event(&mut self, event: &Event) -> EventResult {
        let Event::Key(key) = event else {
            return EventResult::ignored();
        };
        match self.blocklist.handle_key(*key) {
            BlocklistEvent::Consumed => EventResult::consumed(),
            BlocklistEvent::Ignored => EventResult::ignored(),
            BlocklistEvent::Close => {
                if let Some(tx) = self.action_tx.as_ref() {
                    let _ = tx.send(Action::CloseBlocklistView);
                }
                EventResult::consumed()
            }
            BlocklistEvent::Emit(action) => {
                if let Some(tx) = self.action_tx.as_ref() {
                    let _ = tx.send(action);
                }
                EventResult::consumed()
            }
        }
    }
}
