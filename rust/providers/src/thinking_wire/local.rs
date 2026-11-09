//! MODEL-016: the local-openai form, per engine. A local profile model's
//! level is sent in the form its engine takes, from what the model declares:
//! named and numeric efforts as `reasoning_effort`, a token budget in the
//! engine's own field, and the declared template toggle (and only it) in
//! `chat_template_kwargs`.
//!
//! Feature: spec/features/local-inference-server-thinking-levels.feature

use crate::local_reasoning::{BudgetField, LocalEngine};
use crate::models::{ReasoningCapability, ReasoningLevel, TemplateToggle};
use serde_json::{json, Map, Value};

/// `{<key>: on}` for the declared toggle.
fn toggle_kwargs(toggle: TemplateToggle, on: bool) -> Value {
    let mut kwargs = Map::new();
    kwargs.insert(toggle.key().to_string(), Value::Bool(on));
    Value::Object(kwargs)
}

/// `request` plus `chat_template_kwargs {<key>: on}` when a toggle is
/// declared; a model with no declared toggle gets no template kwargs.
fn with_toggle(mut request: Value, toggle: Option<TemplateToggle>, on: bool) -> Value {
    if let Some(toggle) = toggle {
        request["chat_template_kwargs"] = toggle_kwargs(toggle, on);
    }
    request
}

/// A named or numeric effort, sent unchanged as `reasoning_effort`.
fn effort(
    engine: LocalEngine,
    model: &str,
    effort: Value,
    toggle: Option<TemplateToggle>,
    on: bool,
) -> Result<Value, String> {
    if !engine.takes_efforts() {
        return Err(format!(
            "{} takes no reasoning effort; no effort can be sent to {model}",
            engine.name()
        ));
    }
    Ok(with_toggle(json!({"reasoning_effort": effort}), toggle, on))
}

/// Off in the model's own declared form: effort none (with the toggle off
/// when one is declared), else the toggle off alone; refused when neither
/// is declared.
pub(super) fn off(
    engine: LocalEngine,
    capability: &ReasoningCapability,
    model: &str,
) -> Result<Value, String> {
    let toggle = capability.template_toggle();
    if capability.efforts().iter().any(|effort| effort == "none") {
        return effort(engine, model, json!("none"), toggle, false);
    }
    match toggle {
        Some(toggle) => Ok(json!({"chat_template_kwargs": toggle_kwargs(toggle, false)})),
        None => Err(format!(
            "thinking cannot be turned off on {model}: it declares neither the effort none \
             nor a template toggle"
        )),
    }
}

/// A level the capability takes, in the engine's form. A declared toggle is
/// on at every level but the effort none, a token budget included.
pub(super) fn level(
    engine: LocalEngine,
    capability: &ReasoningCapability,
    level: &ReasoningLevel,
    model: &str,
) -> Result<Value, String> {
    let toggle = capability.template_toggle();
    match level {
        ReasoningLevel::Effort(name) => effort(
            engine,
            model,
            json!(name.as_str()),
            toggle,
            name.as_str() != "none",
        ),
        ReasoningLevel::Numeric(number) => effort(engine, model, json!(number), toggle, true),
        ReasoningLevel::Budget(budget) => {
            let request = match engine.budget_field() {
                Some(BudgetField::ThinkingTokenBudget) => {
                    json!({"thinking_token_budget": budget})
                }
                Some(BudgetField::ReasoningBudgetTokens) => {
                    json!({"reasoning_budget_tokens": budget})
                }
                Some(BudgetField::CustomParamsThinkingBudget) => {
                    json!({"custom_params": {"thinking_budget": budget}})
                }
                None => {
                    return Err(format!(
                        "{} takes no token budget; no budget can be sent to {model}",
                        engine.name()
                    ))
                }
            };
            Ok(with_toggle(request, toggle, true))
        }
        ReasoningLevel::On => match toggle {
            Some(toggle) => Ok(json!({"chat_template_kwargs": toggle_kwargs(toggle, true)})),
            None => Err(format!(
                "{model} declares no template toggle, so it has no On level"
            )),
        },
    }
}
