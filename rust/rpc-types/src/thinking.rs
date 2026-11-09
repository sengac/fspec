//! MODEL-011: the levels a session's current model offers, as clients show
//! and choose them (the thinking dialog and `/thinking`).
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature
//!
//! The daemon computes [`ThinkingOptions`] from the model's thinking target
//! (MODEL-009 capability through the MODEL-010 wire table); clients only
//! display them and resolve what the user typed against them. Nothing is
//! remapped or defaulted: a level the model does not offer is refused with
//! the levels it does.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Number;

use crate::ThinkingLevel;

/// Whether a numeric effort range takes whole numbers or decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum NumericEffortKind {
    Whole,
    Decimal,
}

/// One level, or one range of levels, a model offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ThinkingChoice {
    /// Thinking off, in the model's own off form.
    Off,
    /// A named effort, as the model advertises it.
    Effort(String),
    /// A numeric effort within `min..=max` (MODEL-016); each bound crosses
    /// the binary wire as a tagged number (BUG-594).
    NumericEffort {
        kind: NumericEffortKind,
        #[serde(with = "wire_number")]
        #[schemars(with = "Number")]
        min: Number,
        #[serde(with = "wire_number")]
        #[schemars(with = "Number")]
        max: Number,
    },
    /// A token budget within `min..=max`; an absent bound is open.
    TokenBudget { min: Option<u64>, max: Option<u64> },
    /// A toggle-only model's thinking switched on (MODEL-016).
    On,
}

/// What the session's current model offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub enum ThinkingOptions {
    /// The model has no reasoning control; `reason` says why.
    NoReasoningControl { model: String, reason: String },
    /// The levels the model offers, in canonical order. `takes_off` is true
    /// wherever the model has an off form, including the one listed as its
    /// none effort.
    Levels {
        model: String,
        choices: Vec<ThinkingChoice>,
        takes_off: bool,
    },
}

impl fmt::Display for ThinkingLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Off => f.write_str("off"),
            Self::ModelDefault => f.write_str("model default"),
            Self::On => f.write_str("on"),
            Self::Effort(name) => f.write_str(name),
            Self::Numeric(number) => write!(f, "{number}"),
            Self::Budget(tokens) => write!(f, "{tokens} tokens"),
        }
    }
}

fn range_text(min: Option<u64>, max: Option<u64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("{min} to {max}"),
        (Some(min), None) => format!("{min} or more"),
        (None, Some(max)) => format!("up to {max}"),
        (None, None) => "any number of tokens".to_string(),
    }
}

impl ThinkingChoice {
    /// The dialog row label.
    pub fn label(&self) -> String {
        match self {
            Self::Off => "Off".to_string(),
            Self::Effort(name) => name.clone(),
            Self::NumericEffort { min, max, .. } => format!("Effort {min}–{max}"),
            Self::TokenBudget {
                min: Some(min),
                max: Some(max),
            } => format!("Token budget {min}–{max}"),
            Self::TokenBudget { min, max } => format!("Token budget {}", range_text(*min, *max)),
            Self::On => "On".to_string(),
        }
    }

    /// Whether the row takes a typed number.
    pub fn takes_number(&self) -> bool {
        matches!(self, Self::NumericEffort { .. } | Self::TokenBudget { .. })
    }

    /// The level a row stands for; `None` for a numeric row, which takes a
    /// typed number.
    pub fn level(&self) -> Option<ThinkingLevel> {
        match self {
            Self::Off => Some(ThinkingLevel::Off),
            Self::Effort(name) => Some(ThinkingLevel::Effort(name.clone())),
            Self::On => Some(ThinkingLevel::On),
            Self::NumericEffort { .. } | Self::TokenBudget { .. } => None,
        }
    }

    /// Whether `level` is this row (a number within this row's range).
    pub fn holds(&self, level: &ThinkingLevel) -> bool {
        match (self, level) {
            (Self::Off, ThinkingLevel::Off) | (Self::On, ThinkingLevel::On) => true,
            (Self::Effort(name), ThinkingLevel::Effort(chosen)) => name == chosen,
            (Self::NumericEffort { .. }, ThinkingLevel::Numeric(number)) => {
                self.numeric_refusal(number).is_none()
            }
            (Self::TokenBudget { min, max }, ThinkingLevel::Budget(tokens)) => {
                budget_in_range(*tokens, *min, *max)
            }
            _ => false,
        }
    }

    /// The row's range, as refusals name it.
    fn range(&self) -> String {
        match self {
            Self::NumericEffort { min, max, .. } => format!("{min} to {max}"),
            Self::TokenBudget { min, max } => range_text(*min, *max),
            other => other.label(),
        }
    }

    /// Why `number` is not a numeric effort this row takes.
    fn numeric_refusal(&self, number: &Number) -> Option<String> {
        let Self::NumericEffort { kind, min, max } = self else {
            return Some("no numeric effort".to_string());
        };
        if *kind == NumericEffortKind::Whole && !(number.is_u64() || number.is_i64()) {
            return Some(format!("{number} is not a whole number"));
        }
        let (Some(value), Some(low), Some(high)) = (number.as_f64(), min.as_f64(), max.as_f64())
        else {
            return Some(format!("{number} cannot be compared with {min} to {max}"));
        };
        (value < low || value > high).then(|| format!("{number} is outside {min} to {max}"))
    }

    /// The level `text` stands for on this numeric row, or why it does not.
    pub fn level_from_text(&self, model: &str, text: &str) -> Result<ThinkingLevel, String> {
        let text = text.trim();
        match self {
            Self::TokenBudget { min, max } => {
                let tokens: u64 = text.parse().map_err(|_| {
                    format!(
                        "{text} is not a token budget; {model} takes a token budget of {}",
                        range_text(*min, *max)
                    )
                })?;
                if budget_in_range(tokens, *min, *max) {
                    Ok(ThinkingLevel::Budget(tokens))
                } else {
                    Err(format!(
                        "{model} takes a token budget of {}, not {tokens}",
                        range_text(*min, *max)
                    ))
                }
            }
            Self::NumericEffort { .. } => {
                let number: Number = serde_json::from_str(text).map_err(|_| {
                    format!(
                        "{text} is not a number; {model} takes an effort of {}",
                        self.range()
                    )
                })?;
                match self.numeric_refusal(&number) {
                    None => Ok(ThinkingLevel::Numeric(number)),
                    Some(reason) => Err(format!(
                        "{model} takes an effort of {}: {reason}",
                        self.range()
                    )),
                }
            }
            other => Err(format!("{} takes no number", other.label())),
        }
    }
}

fn budget_in_range(tokens: u64, min: Option<u64>, max: Option<u64>) -> bool {
    min.is_none_or(|min| tokens >= min) && max.is_none_or(|max| tokens <= max)
}

mod command;
pub(crate) mod wire_number;

impl ThinkingOptions {
    pub fn model(&self) -> &str {
        match self {
            Self::NoReasoningControl { model, .. } | Self::Levels { model, .. } => model,
        }
    }

    /// The offered choices; none without reasoning control.
    pub fn choices(&self) -> &[ThinkingChoice] {
        match self {
            Self::NoReasoningControl { .. } => &[],
            Self::Levels { choices, .. } => choices,
        }
    }

    /// The offered levels as refusals list them.
    fn listed(&self) -> String {
        self.choices()
            .iter()
            .map(ThinkingChoice::label)
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn not_taken(&self, what: &str) -> String {
        format!(
            "{} does not take {what}; it takes {}",
            self.model(),
            self.listed()
        )
    }

    fn numeric_choice(&self, budget: bool) -> Option<&ThinkingChoice> {
        self.choices().iter().find(|choice| match choice {
            ThinkingChoice::TokenBudget { .. } => budget,
            ThinkingChoice::NumericEffort { .. } => !budget,
            _ => false,
        })
    }
}
