//! MODEL-011: a `/thinking <arg>` argument resolved against the levels the
//! session's model offers.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature

use serde_json::Number;

use super::ThinkingOptions;
use crate::ThinkingLevel;

impl ThinkingOptions {
    /// The level a `/thinking <arg>` argument (trimmed, lowercased) stands
    /// for on this model, or why it is refused. `budget <n>` and
    /// `effort <n>` name the numeric kind; a bare number is the one numeric
    /// kind the model offers and is ambiguous when it offers both.
    pub fn resolve_command(&self, arg: &str) -> Result<ThinkingLevel, String> {
        if let Self::NoReasoningControl { reason, .. } = self {
            return Err(reason.clone());
        }
        let model = self.model();
        if let Some(number) = arg.strip_prefix("budget ") {
            return match self.numeric_choice(true) {
                Some(choice) => choice.level_from_text(model, number),
                None => Err(self.not_taken("a token budget")),
            };
        }
        if let Some(number) = arg.strip_prefix("effort ") {
            return match self.numeric_choice(false) {
                Some(choice) => choice.level_from_text(model, number),
                None => Err(self.not_taken("a numeric effort")),
            };
        }
        if serde_json::from_str::<Number>(arg).is_ok() {
            return match (self.numeric_choice(true), self.numeric_choice(false)) {
                (Some(budget), Some(effort)) => Err(format!(
                    "{arg} is ambiguous on {model}: it takes an effort of {} and a token budget \
                     of {}; write /thinking effort {arg} or /thinking budget {arg}",
                    effort.range(),
                    budget.range()
                )),
                (Some(choice), None) | (None, Some(choice)) => choice.level_from_text(model, arg),
                (None, None) => Err(self.not_taken(&format!("the number {arg}"))),
            };
        }
        let level = match arg {
            "off" => ThinkingLevel::Off,
            "on" => ThinkingLevel::On,
            name => ThinkingLevel::Effort(name.to_string()),
        };
        let taken = match (&level, self) {
            (ThinkingLevel::Off, Self::Levels { takes_off, .. }) => *takes_off,
            _ => self.choices().iter().any(|choice| choice.holds(&level)),
        };
        if taken {
            Ok(level)
        } else {
            Err(self.not_taken(arg))
        }
    }
}
