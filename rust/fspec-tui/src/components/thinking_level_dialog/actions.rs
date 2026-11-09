//! MODEL-011: ThinkingLevelDialog row activation, the number field of a
//! numeric row, `D` and left clicks. MODEL-012: `D` saves any level the
//! model offers, for the current model.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature
//! Feature: spec/features/thinking-level-remembered-per-model.feature

use crossterm::event::KeyCode;

use codelet_rpc_types::{ThinkingChoice, ThinkingLevel};

use super::{NumberField, ThinkingLevelDialog};
use crate::components::{Action, EventResult};

impl ThinkingLevelDialog {
    /// Choose `level` and close the dialog.
    pub(super) fn choose(&mut self, level: ThinkingLevel) -> EventResult {
        self.emit_action(Action::ThinkingLevelSelected(
            self.session_id.clone(),
            level,
        ));
        self.close()
    }

    /// Activate the highlighted row: choose its level, or open the number
    /// field of a numeric row, pre-filled with the session's current value
    /// of that kind.
    pub(super) fn activate(&mut self) -> EventResult {
        if let Some(level) = self.selected_level() {
            return self.choose(level);
        }
        let text = match (&self.choices[self.selected_index], &self.current) {
            (ThinkingChoice::TokenBudget { .. }, ThinkingLevel::Budget(tokens)) => {
                tokens.to_string()
            }
            (ThinkingChoice::NumericEffort { .. }, ThinkingLevel::Numeric(number)) => {
                number.to_string()
            }
            _ => String::new(),
        };
        self.field = Some(NumberField {
            row: self.selected_index,
            text,
            refusal: None,
        });
        EventResult::consumed()
    }

    /// RPC-027 / MODEL-012: `D` saves the highlighted level as the current
    /// model's saved level; the daemon checks it against the model again.
    pub(super) fn set_default(&mut self) -> EventResult {
        let session = self.session_id.clone();
        match self.level_to_save() {
            Ok(level) => {
                self.default_index = Some(self.selected_index); // live marker move (TS parity)
                self.emit_action(Action::SetThinkingLevelDefault(session, level));
            }
            Err(reason) => self.emit_action(Action::ThinkingLevelRefused(session, reason)),
        }
        EventResult::consumed()
    }

    /// MODEL-012: the level `D` saves: the highlighted row's level, or on a
    /// numeric row the session's current value of that kind (the value the
    /// number field is pre-filled with); a numeric row with no such value is
    /// refused, never given an invented number.
    fn level_to_save(&self) -> Result<ThinkingLevel, String> {
        if let Some(level) = self.selected_level() {
            return Ok(level);
        }
        let choice = &self.choices[self.selected_index];
        if choice.holds(&self.current) {
            return Ok(self.current.clone());
        }
        let kind = match choice {
            ThinkingChoice::TokenBudget { .. } => "token budget",
            _ => "numeric effort",
        };
        Err(format!(
            "D saves the session's {kind} for {}: choose a {kind} first with Enter, then press D on this row",
            self.model
        ))
    }

    /// A key while the number field is open.
    pub(super) fn field_key(&mut self, code: KeyCode) -> EventResult {
        let Some(field) = self.field.as_mut() else {
            return EventResult::ignored();
        };
        match code {
            KeyCode::Esc => self.field = None,
            KeyCode::Backspace => {
                field.text.pop();
                field.refusal = None;
            }
            KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                field.text.push(c);
                field.refusal = None;
            }
            KeyCode::Enter => {
                let choice = &self.choices[field.row];
                match choice.level_from_text(&self.model, &field.text) {
                    Ok(level) => return self.choose(level),
                    Err(refusal) => field.refusal = Some(refusal),
                }
            }
            _ => {}
        }
        EventResult::consumed()
    }

    /// The row a left click landed on, if any.
    pub(super) fn click(&mut self, column: u16, row: u16) -> EventResult {
        let layout = self.last_layout.clone();
        if !layout.contains(column, row) {
            return EventResult::ignored();
        }
        let offset = usize::from(self.no_control.is_some());
        match layout
            .hit(column, row)
            .and_then(|hit| hit.checked_sub(offset))
        {
            Some(index) if index < self.row_count() => {
                self.selected_index = index;
                self.activate()
            }
            _ => EventResult::ignored(),
        }
    }

    pub(super) fn description(&self, choice: &ThinkingChoice) -> String {
        match choice {
            ThinkingChoice::Off => "No thinking".to_string(),
            ThinkingChoice::Effort(_) => "Named effort".to_string(),
            ThinkingChoice::NumericEffort { .. } => "Enter types a number".to_string(),
            ThinkingChoice::TokenBudget { .. } => "Enter types a token budget".to_string(),
            ThinkingChoice::On => "Thinking on".to_string(),
        }
    }
}
