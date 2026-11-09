//! A model's reasoning capability, read from its models.dev
//! `reasoning_options`, and the check that a chosen level is one the model
//! takes. Nothing is remapped or defaulted: an unadvertised level is an
//! error naming the model and what it supports.

use std::fmt;

mod catalog;
mod error;
mod numeric;
mod option;

pub use error::ReasoningError;
pub use numeric::{BudgetRange, NumericKind, NumericRange, TemplateToggle};
pub use option::ReasoningOption;

use numeric::NumericRefusal;

/// Named efforts in their one canonical order; advertised names outside
/// this list follow it in the order the model lists them.
const CANONICAL_EFFORTS: [&str; 8] = [
    "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
];

/// A named effort: lowercase letters, digits, `-` and `_`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffortName(String);

impl EffortName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EffortName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A chosen reasoning level: a named effort, a token budget, a numeric
/// effort (MODEL-016: a JSON number sent as `reasoning_effort`) or On, the
/// toggle-only model's thinking switched on (Off is the turn's own level).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReasoningLevel {
    Effort(EffortName),
    Budget(u64),
    Numeric(serde_json::Number),
    On,
}

impl ReasoningLevel {
    pub fn effort(name: &str) -> Result<Self, ReasoningError> {
        let valid = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_');
        if valid {
            Ok(Self::Effort(EffortName(name.to_string())))
        } else {
            Err(ReasoningError::InvalidEffortName(name.to_string()))
        }
    }
}

/// What a model lets the user choose for its reasoning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReasoningCapability {
    efforts: Vec<String>,
    budget: Option<BudgetRange>,
    /// A models.dev toggle option.
    toggle: bool,
    /// MODEL-016: a declared numeric effort range.
    numeric: Option<NumericRange>,
    /// MODEL-016: a declared chat-template toggle.
    template_toggle: Option<TemplateToggle>,
}

/// Sort efforts into canonical order. Names outside the canonical list sort
/// after it, keeping the order the model lists them in (the stable sort
/// keeps ties): thinking-levels-from-model-capabilities.feature,
/// architecture notes.
fn sort_canonically(efforts: &mut [String]) {
    let rank = |effort: &String| {
        CANONICAL_EFFORTS
            .iter()
            .position(|known| known == effort)
            .unwrap_or(CANONICAL_EFFORTS.len())
    };
    efforts.sort_by_key(rank);
}

impl ReasoningCapability {
    /// Resolve `model`'s capability from its options. No options, or an
    /// empty list, is no capability: no reasoning control is offered.
    pub fn resolve(
        model: &str,
        options: Option<&[ReasoningOption]>,
    ) -> Result<Option<Self>, ReasoningError> {
        let Some(options) = options.filter(|options| !options.is_empty()) else {
            return Ok(None);
        };
        let mut capability = Self {
            efforts: Vec::new(),
            budget: None,
            toggle: false,
            numeric: None,
            template_toggle: None,
        };
        for option in options {
            match option {
                ReasoningOption::Effort(values) => {
                    for value in values {
                        if !capability.efforts.contains(value) {
                            capability.efforts.push(value.clone());
                        }
                    }
                }
                ReasoningOption::BudgetTokens(range) => {
                    if capability.budget.replace(*range).is_some() {
                        return Err(ReasoningError::UnrecognisedOption {
                            model: model.to_string(),
                            kind: "budget_tokens".to_string(),
                            reason: "more than one budget_tokens option".to_string(),
                        });
                    }
                }
                ReasoningOption::Toggle => capability.toggle = true,
                ReasoningOption::Unrecognised { kind, reason } => {
                    return Err(ReasoningError::UnrecognisedOption {
                        model: model.to_string(),
                        kind: kind.clone(),
                        reason: reason.clone(),
                    });
                }
            }
        }
        sort_canonically(&mut capability.efforts);
        Ok(Some(capability))
    }

    /// MODEL-016: a declared capability (effort names already checked).
    pub fn declared(
        efforts: &[String],
        numeric: Option<NumericRange>,
        budget: Option<BudgetRange>,
        template_toggle: Option<TemplateToggle>,
    ) -> Self {
        let mut unique: Vec<String> = Vec::new();
        for effort in efforts {
            if !unique.contains(effort) {
                unique.push(effort.clone());
            }
        }
        sort_canonically(&mut unique);
        Self {
            efforts: unique,
            budget,
            toggle: false,
            numeric,
            template_toggle,
        }
    }

    /// The model's named efforts, in canonical order.
    pub fn efforts(&self) -> &[String] {
        &self.efforts
    }

    pub fn budget(&self) -> Option<BudgetRange> {
        self.budget
    }

    pub fn has_toggle(&self) -> bool {
        self.toggle || self.template_toggle.is_some()
    }

    pub fn numeric(&self) -> Option<&NumericRange> {
        self.numeric.as_ref()
    }

    pub fn template_toggle(&self) -> Option<TemplateToggle> {
        self.template_toggle
    }

    /// MODEL-016: a model whose only effort control is its toggle offers On
    /// and Off (product owner, 2026-11-04).
    pub fn takes_on(&self) -> bool {
        self.has_toggle() && self.efforts.is_empty() && self.numeric.is_none()
    }

    /// The levels the model offers, as the user picks them: its named
    /// efforts, its numeric effort range, its token budget, and On and Off
    /// for a toggle-only model.
    pub fn offered_levels(&self) -> Vec<String> {
        let mut levels = self.efforts.clone();
        if let Some(range) = &self.numeric {
            levels.push(range.to_string());
        }
        if let Some(range) = self.budget {
            levels.push(format!("a token budget of {range}"));
        }
        if self.takes_on() {
            levels.extend(["On".to_string(), "Off".to_string()]);
        }
        levels
    }

    /// MODEL-010: what a model without named efforts takes instead (its
    /// token budget, its on/off toggle), so a refused effort says what can be
    /// chosen; `None` when the model has named efforts.
    fn supported_without_efforts(&self) -> Option<String> {
        if !self.efforts.is_empty() {
            return None;
        }
        let mut takes = Vec::new();
        if let Some(range) = &self.numeric {
            takes.push(range.to_string());
        }
        if let Some(range) = self.budget {
            takes.push(format!("a token budget of {range}"));
        }
        if self.takes_on() && self.template_toggle.is_some() {
            takes.push("On or Off".to_string());
        } else if self.toggle {
            takes.push("only turning thinking off".to_string());
        }
        Some(match takes.as_slice() {
            [] => "no named efforts".to_string(),
            _ => takes.join(" or "),
        })
    }

    /// What the model takes, for a refusal.
    fn supported(&self) -> String {
        match self.offered_levels().as_slice() {
            [] => self
                .supported_without_efforts()
                .unwrap_or_else(|| "no reasoning level".to_string()),
            levels => levels.join(", "),
        }
    }

    /// Accept `level` unchanged if `model` takes it.
    pub fn check(
        &self,
        model: &str,
        level: &ReasoningLevel,
    ) -> Result<ReasoningLevel, ReasoningError> {
        let refused = match level {
            ReasoningLevel::Effort(name) if self.efforts.iter().any(|e| e == name.as_str()) => None,
            ReasoningLevel::Effort(name) => Some(ReasoningError::UnsupportedEffort {
                model: model.to_string(),
                effort: name.to_string(),
                supported: self
                    .supported_without_efforts()
                    .unwrap_or_else(|| self.efforts.join(", ")),
            }),
            ReasoningLevel::Budget(budget) => match self.budget {
                Some(range) if range.contains(*budget) => None,
                Some(range) => Some(ReasoningError::BudgetOutOfRange {
                    model: model.to_string(),
                    budget: *budget,
                    range,
                }),
                None => Some(ReasoningError::BudgetNotTaken {
                    model: model.to_string(),
                }),
            },
            ReasoningLevel::Numeric(effort) => match &self.numeric {
                None => Some(ReasoningError::NumericNotTaken {
                    model: model.to_string(),
                    supported: self.supported(),
                }),
                Some(range) => range.refusal(effort).map(|refusal| match refusal {
                    NumericRefusal::NotWhole => ReasoningError::NotWholeNumber {
                        model: model.to_string(),
                        effort: effort.to_string(),
                        range: range.to_string(),
                    },
                    NumericRefusal::OutOfRange => ReasoningError::NumericOutOfRange {
                        model: model.to_string(),
                        effort: effort.to_string(),
                        range: range.to_string(),
                    },
                }),
            },
            ReasoningLevel::On if self.takes_on() => None,
            ReasoningLevel::On => Some(ReasoningError::OnNotTaken {
                model: model.to_string(),
                supported: self.supported(),
            }),
        };
        match refused {
            Some(error) => Err(error),
            None => Ok(level.clone()),
        }
    }
}
