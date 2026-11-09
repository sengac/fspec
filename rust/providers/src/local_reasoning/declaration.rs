//! The `thinkingLevels` declaration of a local-server profile or of one of
//! its customModels entries, and its resolution against the engine.

use serde::{Deserialize, Serialize};

use super::{LocalEngine, LocalReasoning};
use crate::models::{
    BudgetRange, NumericKind, NumericRange, ReasoningCapability, ReasoningLevel, TemplateToggle,
};

/// `thinkingLevels`: the levels a local model takes, in any combination.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThinkingLevelsDeclaration {
    /// Named efforts, sent unchanged as `reasoning_effort`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub efforts: Option<Vec<String>>,
    /// A numeric effort range, sent as a JSON number in `reasoning_effort`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric_efforts: Option<NumericEffortsDeclaration>,
    /// A token budget range, sent in the engine's budget field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<TokenBudgetDeclaration>,
    /// The chat-template toggle key (`enable_thinking` or `thinking`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_toggle: Option<TemplateToggle>,
}

/// `numericEfforts`: `{kind: whole | decimal, min, max}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NumericEffortsDeclaration {
    pub kind: NumericKind,
    pub min: serde_json::Number,
    pub max: serde_json::Number,
}

/// `tokenBudget`: `{min, max}` in tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TokenBudgetDeclaration {
    pub min: u64,
    pub max: u64,
}

impl ThinkingLevelsDeclaration {
    /// `profile`'s `model` with this declaration on `engine`. A part the
    /// engine has no field for is an error naming the profile, the model,
    /// the engine and the part.
    pub fn resolve(
        &self,
        profile: &str,
        model: &str,
        engine: LocalEngine,
    ) -> Result<LocalReasoning, String> {
        let error = |reason: String| format!("profile {profile}, model {model}: {reason}");
        Ok(LocalReasoning {
            profile: profile.to_string(),
            engine,
            capability: self.capability(&error, Some(engine))?,
        })
    }

    /// MODEL-017: custom provider `provider`'s `model` with this
    /// declaration. A Rhai script owns its wire, so no part is checked
    /// against an engine; the declaration itself must still be usable.
    pub fn resolve_for_provider(
        &self,
        provider: &str,
        model: &str,
    ) -> Result<ReasoningCapability, String> {
        let error = |reason: String| format!("custom provider {provider}, model {model}: {reason}");
        self.capability(&error, None)
    }

    /// The declared capability; with an engine, every part must be one the
    /// engine has a field for.
    fn capability(
        &self,
        error: &dyn Fn(String) -> String,
        engine: Option<LocalEngine>,
    ) -> Result<ReasoningCapability, String> {
        let no_field = |engine: LocalEngine, part: String| {
            error(format!("{} has no field for {part}", engine.name()))
        };
        let efforts = match &self.efforts {
            None => Vec::new(),
            Some(efforts) if efforts.is_empty() => {
                return Err(error("thinkingLevels.efforts lists no effort".to_string()))
            }
            Some(efforts) => {
                if let Some(engine) = engine.filter(|engine| !engine.takes_efforts()) {
                    return Err(no_field(
                        engine,
                        format!("efforts \"{}\"", efforts.join(", ")),
                    ));
                }
                for effort in efforts {
                    ReasoningLevel::effort(effort).map_err(|reason| error(reason.to_string()))?;
                }
                efforts.clone()
            }
        };
        let numeric = self
            .numeric_efforts
            .as_ref()
            .map(|declared| {
                let range =
                    NumericRange::new(declared.kind, declared.min.clone(), declared.max.clone())
                        .map_err(error)?;
                let Some(engine) = engine else {
                    return Ok(range);
                };
                match engine.numeric_efforts().map_err(error)? {
                    None => Err(no_field(engine, range.to_string())),
                    Some(taken) if taken.covers(&range).map_err(error)? => Ok(range),
                    Some(taken) => Err(error(format!(
                        "{} takes {taken} only, not {range}",
                        engine.name()
                    ))),
                }
            })
            .transpose()?;
        let budget = self
            .token_budget
            .as_ref()
            .map(|declared| {
                if let Some(engine) = engine.filter(|engine| engine.budget_field().is_none()) {
                    return Err(no_field(engine, "a token budget".to_string()));
                }
                if declared.min > declared.max {
                    return Err(error(format!(
                        "the token budget {} to {} has its minimum above its maximum",
                        declared.min, declared.max
                    )));
                }
                Ok(BudgetRange {
                    min: Some(declared.min),
                    max: Some(declared.max),
                })
            })
            .transpose()?;
        let toggle = match (self.template_toggle, engine) {
            (Some(toggle), Some(engine)) if !engine.takes_toggle(toggle) => {
                let only = match engine {
                    LocalEngine::Strata => " (Strata reads only enable_thinking)",
                    _ => "",
                };
                return Err(no_field(
                    engine,
                    format!("the template toggle {}{only}", toggle.key()),
                ));
            }
            (toggle, _) => toggle,
        };
        if efforts.is_empty() && numeric.is_none() && budget.is_none() && toggle.is_none() {
            return Err(error("thinkingLevels declares no level".to_string()));
        }
        Ok(ReasoningCapability::declared(
            &efforts, numeric, budget, toggle,
        ))
    }
}
