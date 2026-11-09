//! MODEL-011: the levels a model offers for the thinking dialog and
//! `/thinking`, and the check that refuses any other level before it is
//! stored on a session.
//!
//! A level is offered only when the model advertises (or declares) it and
//! the dispatched provider has a form for it, so the offer is read off the
//! wire table itself: each candidate is probed with [`ThinkingTarget::config_for`].
//! A target with no capability has no reasoning control, whatever its
//! source (product owner, 2026-11-05): custom provider models that declare
//! no thinkingLevels in their provider config (MODEL-017), cloud models
//! missing from models.dev, profile models that declare nothing.
//!
//! Feature: spec/features/thinking-dialog-offers-only-the-models-levels.feature

use crate::models::{BudgetRange, NumericRange, ReasoningCapability, ReasoningLevel};

use super::{CapabilitySource, ThinkingTarget, TurnLevel};

/// One level, or one range of levels, a model offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfferedLevel {
    /// Thinking off, in the model's own off form.
    Off,
    /// A named effort.
    Effort(String),
    /// A numeric effort range (MODEL-016).
    Numeric(NumericRange),
    /// A token budget range.
    Budget(BudgetRange),
    /// A toggle-only model's thinking switched on (MODEL-016).
    On,
}

/// What a model offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThinkingOffer {
    /// The model has no reasoning control, with the reason.
    NoReasoningControl(String),
    /// The levels in canonical order. `takes_off` is true wherever the model
    /// has an off form, including one listed as its none effort.
    Levels {
        levels: Vec<OfferedLevel>,
        takes_off: bool,
    },
}

impl ThinkingTarget {
    /// `provider/model`, as errors name it.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The model's capability, or why it has no reasoning control.
    fn controlled(&self) -> Result<ReasoningCapability, String> {
        let model = &self.model;
        if let Err(error) = &self.wire {
            return Err(format!("{model} has no reasoning control: {error}"));
        }
        match &self.capability {
            Ok(Some(capability)) => Ok(capability.clone()),
            Err(error) => Err(format!("{model} has no reasoning control: {error}")),
            Ok(None) => Err(match &self.source {
                CapabilitySource::Declared(_) => format!(
                    "{model} has no reasoning control: it declares no thinking levels; declare \
                     them in its profile (thinkingLevels on the profile or on its customModels \
                     entry, with the profile's engine)"
                ),
                CapabilitySource::Advertised => super::unadvertised_model(model),
                CapabilitySource::ProviderConfig(provider) => {
                    super::undeclared_custom_model(model, provider)
                }
            }),
        }
    }

    fn sends(&self, level: TurnLevel) -> bool {
        self.config_for(Some(&level)).is_ok()
    }

    /// The levels the model offers, in canonical order: Off where it has an
    /// off form that is not its advertised none effort (none and Off send the
    /// same thing), its named efforts, its numeric effort range, its token
    /// budget, and On; each only when the provider has a form for it.
    pub fn offer(&self) -> ThinkingOffer {
        let capability = match self.controlled() {
            Ok(capability) => capability,
            Err(reason) => return ThinkingOffer::NoReasoningControl(reason),
        };
        let takes_off = self.sends(TurnLevel::Off);
        let mut levels = Vec::new();
        if takes_off && !capability.efforts().iter().any(|effort| effort == "none") {
            levels.push(OfferedLevel::Off);
        }
        for effort in capability.efforts() {
            let sent = ReasoningLevel::effort(effort)
                .is_ok_and(|level| self.sends(TurnLevel::Level(level)));
            if sent {
                levels.push(OfferedLevel::Effort(effort.clone()));
            }
        }
        if let Some(range) = capability.numeric() {
            let probe = ReasoningLevel::Numeric(range.min().clone());
            if self.sends(TurnLevel::Level(probe)) {
                levels.push(OfferedLevel::Numeric(range.clone()));
            }
        }
        if let Some(range) = capability.budget() {
            let probe = range.min.or(range.max).unwrap_or(0);
            if self.sends(TurnLevel::Level(ReasoningLevel::Budget(probe))) {
                levels.push(OfferedLevel::Budget(range));
            }
        }
        if self.sends(TurnLevel::Level(ReasoningLevel::On)) {
            levels.push(OfferedLevel::On);
        }
        if levels.is_empty() {
            return ThinkingOffer::NoReasoningControl(format!(
                "{} has no reasoning control: no level it advertises has a form on its provider",
                self.model
            ));
        }
        ThinkingOffer::Levels { levels, takes_off }
    }

    /// Refuse a level the model does not offer, with the reason, before it
    /// is stored; a model with no reasoning control takes no level.
    pub fn accepts(&self, level: &TurnLevel) -> Result<(), String> {
        self.controlled()?;
        self.config_for(Some(level)).map(|_| ())
    }
}
