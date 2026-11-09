//! Why a reasoning capability or level was refused.

use thiserror::Error;

use super::BudgetRange;
use crate::error::ProviderError;

#[derive(Debug, Error)]
pub enum ReasoningError {
    #[error("{0:?} is not an effort name (lowercase letters, digits, '-' and '_')")]
    InvalidEffortName(String),
    #[error("model {model} lists reasoning option type {kind:?}: {reason}")]
    UnrecognisedOption {
        model: String,
        kind: String,
        reason: String,
    },
    #[error("model {model} has no reasoning control; no thinking level can be chosen for it")]
    NoReasoningControl { model: String },
    #[error("model {model} does not take effort {effort}; it takes {supported}")]
    UnsupportedEffort {
        model: String,
        effort: String,
        supported: String,
    },
    #[error("model {model} takes no token budget")]
    BudgetNotTaken { model: String },
    #[error("model {model} takes a token budget of {range}; {budget} is outside it")]
    BudgetOutOfRange {
        model: String,
        budget: u64,
        range: BudgetRange,
    },
    #[error("model {model} takes no numeric effort; it takes {supported}")]
    NumericNotTaken { model: String, supported: String },
    #[error("model {model} takes {range}; {effort} is outside it")]
    NumericOutOfRange {
        model: String,
        effort: String,
        range: String,
    },
    #[error("model {model} takes {range}; {effort} is not a whole number")]
    NotWholeNumber {
        model: String,
        effort: String,
        range: String,
    },
    #[error("model {model} has no On level; it takes {supported}")]
    OnNotTaken { model: String, supported: String },
    #[error(transparent)]
    Model(#[from] ProviderError),
}
