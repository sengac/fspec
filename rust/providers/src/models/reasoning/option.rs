//! The models.dev `reasoning_options` entry and its wire shape.

use serde::{Deserialize, Serialize};

use super::BudgetRange;

/// One models.dev `reasoning_options` entry.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(from = "RawOption", into = "RawOption")]
pub enum ReasoningOption {
    Effort(Vec<String>),
    BudgetTokens(BudgetRange),
    Toggle,
    /// An entry fspec cannot read: an unknown `type`, or a known one
    /// missing its data. Kept so the rest of the catalog loads; resolving
    /// this model's capability reports it.
    Unrecognised {
        kind: String,
        reason: String,
    },
}

/// The wire shape of a `reasoning_options` entry.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct RawOption {
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<String>>,
    /// Read as any JSON number so one model's unusable bound (models.dev
    /// lists `min: -1` for some) cannot stop the catalog loading.
    #[serde(skip_serializing_if = "Option::is_none")]
    min: Option<serde_json::Number>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<serde_json::Number>,
}

/// A budget bound as a token count, or why it is not one.
fn bound(number: Option<serde_json::Number>) -> Result<Option<u64>, String> {
    match number {
        None => Ok(None),
        Some(number) => number.as_u64().map(Some).ok_or_else(|| {
            format!("budget bound {number} is not a whole, non-negative token count")
        }),
    }
}

impl From<RawOption> for ReasoningOption {
    fn from(raw: RawOption) -> Self {
        match (raw.kind.as_str(), raw.values) {
            ("effort", Some(values)) => Self::Effort(values),
            ("effort", None) => Self::Unrecognised {
                kind: raw.kind,
                reason: "an effort option without values".to_string(),
            },
            ("budget_tokens", _) => match (bound(raw.min), bound(raw.max)) {
                (Ok(min), Ok(max)) => Self::BudgetTokens(BudgetRange { min, max }),
                (Err(reason), _) | (_, Err(reason)) => Self::Unrecognised {
                    kind: raw.kind,
                    reason,
                },
            },
            ("toggle", _) => Self::Toggle,
            _ => Self::Unrecognised {
                kind: raw.kind,
                reason: "a reasoning option type fspec does not know".to_string(),
            },
        }
    }
}

impl From<ReasoningOption> for RawOption {
    fn from(option: ReasoningOption) -> Self {
        let raw = |kind: &str| RawOption {
            kind: kind.to_string(),
            values: None,
            min: None,
            max: None,
        };
        match option {
            ReasoningOption::Effort(values) => RawOption {
                values: Some(values),
                ..raw("effort")
            },
            ReasoningOption::BudgetTokens(range) => RawOption {
                min: range.min.map(serde_json::Number::from),
                max: range.max.map(serde_json::Number::from),
                ..raw("budget_tokens")
            },
            ReasoningOption::Toggle => raw("toggle"),
            ReasoningOption::Unrecognised { kind, .. } => raw(&kind),
        }
    }
}
