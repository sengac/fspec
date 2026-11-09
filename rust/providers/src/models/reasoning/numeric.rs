//! MODEL-016: numeric reasoning efforts (whole numbers or decimals, sent as
//! a JSON number in `reasoning_effort`, never as a token budget) and the
//! chat-template toggle a local model's thinking is switched with.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Number;

/// An advertised token-budget range; either end may be open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetRange {
    pub min: Option<u64>,
    pub max: Option<u64>,
}

impl BudgetRange {
    pub(super) fn contains(self, budget: u64) -> bool {
        self.min.is_none_or(|min| budget >= min) && self.max.is_none_or(|max| budget <= max)
    }
}

impl fmt::Display for BudgetRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.min, self.max) {
            (Some(min), Some(max)) => write!(f, "{min} to {max} tokens"),
            (Some(min), None) => write!(f, "at least {min} tokens"),
            (None, Some(max)) => write!(f, "at most {max} tokens"),
            (None, None) => write!(f, "any number of tokens"),
        }
    }
}

/// Whether a numeric effort range takes whole numbers or decimals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NumericKind {
    Whole,
    Decimal,
}

/// A declared numeric effort range, both ends inclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumericRange {
    kind: NumericKind,
    min: Number,
    max: Number,
}

/// Why a numeric effort is not one a range takes.
pub(super) enum NumericRefusal {
    NotWhole,
    OutOfRange,
}

fn is_whole(number: &Number) -> bool {
    number.is_i64() || number.is_u64()
}

fn value(number: &Number) -> Result<f64, String> {
    number
        .as_f64()
        .ok_or_else(|| format!("{number} is not a number fspec can compare"))
}

impl NumericRange {
    /// A range from `min` to `max`; whole-number ranges need whole bounds.
    pub fn new(kind: NumericKind, min: Number, max: Number) -> Result<Self, String> {
        if kind == NumericKind::Whole && !(is_whole(&min) && is_whole(&max)) {
            return Err(format!(
                "a whole-number effort range needs whole-number bounds, not {min} to {max}"
            ));
        }
        if value(&min)? > value(&max)? {
            return Err(format!(
                "the numeric effort range {min} to {max} has its minimum above its maximum"
            ));
        }
        Ok(Self { kind, min, max })
    }

    pub fn kind(&self) -> NumericKind {
        self.kind
    }

    /// MODEL-011: the lowest effort the range takes.
    pub fn min(&self) -> &Number {
        &self.min
    }

    /// MODEL-011: the highest effort the range takes.
    pub fn max(&self) -> &Number {
        &self.max
    }

    /// Whether every effort `other` takes is one this range takes.
    pub fn covers(&self, other: &Self) -> Result<bool, String> {
        Ok(self.kind == other.kind
            && value(&other.min)? >= value(&self.min)?
            && value(&other.max)? <= value(&self.max)?)
    }

    /// `None` when `effort` is in the range and of its kind.
    pub(super) fn refusal(&self, effort: &Number) -> Option<NumericRefusal> {
        if self.kind == NumericKind::Whole && !is_whole(effort) {
            return Some(NumericRefusal::NotWhole);
        }
        match (value(effort), value(&self.min), value(&self.max)) {
            (Ok(effort), Ok(min), Ok(max)) if (min..=max).contains(&effort) => None,
            _ => Some(NumericRefusal::OutOfRange),
        }
    }
}

impl fmt::Display for NumericRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            NumericKind::Whole => "whole-number",
            NumericKind::Decimal => "decimal",
        };
        write!(f, "{kind} efforts from {} to {}", self.min, self.max)
    }
}

/// The `chat_template_kwargs` key a model's chat template switches its
/// thinking with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum TemplateToggle {
    /// Qwen3, GLM-4.5, Nemotron 3, InternS1, Gemma 4, Granite 4.2.
    #[serde(rename = "enable_thinking")]
    EnableThinking,
    /// DeepSeek-V3.x, Kimi K2, Granite 3.2.
    #[serde(rename = "thinking")]
    Thinking,
}

impl TemplateToggle {
    pub fn key(self) -> &'static str {
        match self {
            Self::EnableThinking => "enable_thinking",
            Self::Thinking => "thinking",
        }
    }
}
