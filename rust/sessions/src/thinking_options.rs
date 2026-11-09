//! MODEL-011: a session's chosen thinking level against its model.
//!
//! [`options_for`] turns the model's [`ThinkingTarget`] offer into the
//! [`ThinkingOptions`] the thinking dialog and `/thinking` show;
//! [`turn_level`] reads a stored [`ThinkingLevel`] as the level a turn
//! sends; [`accepted`] refuses a level the model does not offer before it
//! is stored. Nothing is remapped: a level is the model's own.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature

use codelet_providers::models::{NumericKind, ReasoningLevel};
use codelet_providers::thinking_wire::{OfferedLevel, ThinkingOffer, ThinkingTarget, TurnLevel};
use codelet_rpc_types::{NumericEffortKind, ThinkingChoice, ThinkingLevel, ThinkingOptions};

/// What `target`'s model offers, as clients show it.
pub fn options_for(target: &ThinkingTarget) -> ThinkingOptions {
    let model = target.model().to_string();
    match target.offer() {
        ThinkingOffer::NoReasoningControl(reason) => {
            ThinkingOptions::NoReasoningControl { model, reason }
        }
        ThinkingOffer::Levels { levels, takes_off } => ThinkingOptions::Levels {
            model,
            choices: levels.into_iter().map(choice).collect(),
            takes_off,
        },
    }
}

fn choice(level: OfferedLevel) -> ThinkingChoice {
    match level {
        OfferedLevel::Off => ThinkingChoice::Off,
        OfferedLevel::Effort(name) => ThinkingChoice::Effort(name),
        OfferedLevel::Numeric(range) => ThinkingChoice::NumericEffort {
            kind: match range.kind() {
                NumericKind::Whole => NumericEffortKind::Whole,
                NumericKind::Decimal => NumericEffortKind::Decimal,
            },
            min: range.min().clone(),
            max: range.max().clone(),
        },
        OfferedLevel::Budget(range) => ThinkingChoice::TokenBudget {
            min: range.min,
            max: range.max,
        },
        OfferedLevel::On => ThinkingChoice::On,
    }
}

/// The level a turn sends for a stored `level`; `None` for the model
/// default (no thinking parameter is sent).
pub fn turn_level(level: &ThinkingLevel) -> Result<Option<TurnLevel>, String> {
    let level = match level {
        ThinkingLevel::ModelDefault => return Ok(None),
        ThinkingLevel::Off => return Ok(Some(TurnLevel::Off)),
        ThinkingLevel::On => ReasoningLevel::On,
        ThinkingLevel::Effort(name) => {
            ReasoningLevel::effort(name).map_err(|error| error.to_string())?
        }
        ThinkingLevel::Numeric(number) => ReasoningLevel::Numeric(number.clone()),
        ThinkingLevel::Budget(tokens) => ReasoningLevel::Budget(*tokens),
    };
    Ok(Some(TurnLevel::Level(level)))
}

/// Refuse `level` unless `target`'s model offers it; the model default is
/// always accepted (it sends nothing).
pub fn accepted(target: &ThinkingTarget, level: &ThinkingLevel) -> Result<(), String> {
    match turn_level(level)? {
        None => Ok(()),
        Some(turn) => target.accepts(&turn),
    }
}

/// MODEL-012: the level a session on a model starts at from the model's
/// saved level, and the notice telling the user why a saved level was not
/// applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedLevel {
    /// The session's level: the saved level when the model offers it,
    /// otherwise not set (the model default; no thinking parameter is sent).
    pub level: ThinkingLevel,
    /// Why the saved level was not applied; `None` when it was, or when the
    /// model has no saved level.
    pub notice: Option<String>,
}

/// MODEL-012: apply `saved` (the model's saved level as loaded) to a session
/// on `target`'s model. A saved level the model offers is applied; one it
/// does not offer, or one that cannot be read, leaves the level not set with
/// a notice saying why (product owner, 2026-11-04, decision 7); no saved
/// level leaves it not set with no notice. Nothing is mapped or defaulted.
pub fn saved_level_for(
    target: &ThinkingTarget,
    saved: Result<Option<ThinkingLevel>, String>,
) -> SavedLevel {
    let model = target.model();
    let not_set = |notice: String| SavedLevel {
        level: ThinkingLevel::ModelDefault,
        notice: Some(format!("{notice}; the thinking level is not set")),
    };
    match saved {
        Ok(None) => SavedLevel {
            level: ThinkingLevel::ModelDefault,
            notice: None,
        },
        Err(error) => not_set(format!(
            "the saved thinking level for {model} in tui.thinkingLevels cannot be read: {error}"
        )),
        Ok(Some(level)) => match accepted(target, &level) {
            Ok(()) => SavedLevel {
                level,
                notice: None,
            },
            Err(refusal) => not_set(format!(
                "the saved thinking level {level} for {model} is not offered: {refusal}"
            )),
        },
    }
}
