//! The provider wire forms of MODEL-010's table: OpenAI Chat Completions,
//! OpenAI Responses, Anthropic, Gemini, Z.AI and Rhai.
//!
//! Feature: spec/features/thinking-levels-reach-each-provider-through-a-wire-table.feature

use crate::models::{ReasoningCapability, ReasoningLevel, TemplateToggle};
use serde_json::{json, Value};

use super::{ThinkingTarget, ThinkingWire};

impl ThinkingTarget {
    pub(super) fn off(
        &self,
        wire: ThinkingWire,
        capability: Option<&ReasoningCapability>,
    ) -> Result<Option<Value>, String> {
        let model = &self.model;
        match wire {
            ThinkingWire::OpenAiChat | ThinkingWire::OpenAiResponses => {
                if let Some(capability) = capability {
                    if !capability.efforts().iter().any(|effort| effort == "none") {
                        return Err(format!(
                            "thinking cannot be turned off on {model}; it takes {}",
                            supported(capability)
                        ));
                    }
                }
                Ok(Some(effort_wire(wire, "none")))
            }
            ThinkingWire::Gemini | ThinkingWire::Zai => {
                if capability.is_some_and(|capability| !capability.has_toggle()) {
                    return Err(format!(
                        "thinking cannot be turned off on {model}: it advertises no thinking toggle"
                    ));
                }
                Ok(Some(match wire {
                    ThinkingWire::Gemini => json!({"thinkingConfig": {"thinkingBudget": 0}}),
                    _ => json!({"thinking": {"type": "disabled"}}),
                }))
            }
            ThinkingWire::Anthropic if self.api_model.is_empty() => Err(format!(
                "thinking Off has no form for {model}: no Claude model is selected"
            )),
            ThinkingWire::Anthropic => codelet_tools::facade::claude_off_config(&self.api_model),
            // MODEL-017: Off reaches the script only where the declaration
            // has an off form, effort none or a toggle.
            ThinkingWire::Rhai => match capability {
                Some(declared)
                    if declared.efforts().iter().any(|effort| effort == "none")
                        || declared.has_toggle()
                        || declared.template_toggle().is_some() =>
                {
                    Ok(Some(rhai_config(json!("off"), capability)))
                }
                _ => Err(format!(
                    "{model} has no off form: it declares neither the effort none nor a \
                     template toggle; choose one of its declared levels"
                )),
            },
            ThinkingWire::LocalOpenAi => Err(format!("no engine is known for {model}")),
        }
    }

    pub(super) fn level(
        &self,
        wire: ThinkingWire,
        level: &ReasoningLevel,
        capability: Option<&ReasoningCapability>,
    ) -> Result<Value, String> {
        let model = &self.model;
        match (wire, level) {
            (ThinkingWire::Rhai, ReasoningLevel::Effort(name)) => {
                Ok(rhai_config(json!(name.as_str()), capability))
            }
            (ThinkingWire::Rhai, ReasoningLevel::Budget(budget)) => {
                Ok(rhai_config(json!(budget), capability))
            }
            (ThinkingWire::Rhai, ReasoningLevel::Numeric(number)) => {
                Ok(rhai_config(json!(number), capability))
            }
            (ThinkingWire::Rhai, ReasoningLevel::On) => Ok(rhai_config(json!("on"), capability)),
            (ThinkingWire::Anthropic, ReasoningLevel::Effort(name)) => Ok(json!({
                // PROV-080: summarized, or newer models return empty thinking.
                "thinking": {"type": "adaptive", "display": "summarized"},
                "output_config": {"effort": name.as_str()}
            })),
            (ThinkingWire::Anthropic, ReasoningLevel::Budget(budget)) => {
                Ok(json!({"thinking": {"type": "enabled", "budget_tokens": budget}}))
            }
            (ThinkingWire::Gemini, ReasoningLevel::Effort(name)) => Ok(json!({
                "thinkingConfig": {"includeThoughts": true, "thinkingLevel": name.as_str()}
            })),
            (ThinkingWire::Gemini, ReasoningLevel::Budget(budget)) => Ok(json!({
                "thinkingConfig": {"includeThoughts": true, "thinkingBudget": budget}
            })),
            (ThinkingWire::Zai, _) => Err(format!(
                "no effort form is known for zai; {model} can only turn thinking off"
            )),
            (
                ThinkingWire::OpenAiChat | ThinkingWire::OpenAiResponses,
                ReasoningLevel::Effort(name),
            ) => Ok(effort_wire(wire, name.as_str())),
            (
                ThinkingWire::OpenAiChat | ThinkingWire::OpenAiResponses,
                ReasoningLevel::Budget(budget),
            ) => Err(format!(
                "{model} takes a named reasoning effort, not a {budget}-token budget"
            )),
            (ThinkingWire::LocalOpenAi, _) => Err(format!("no engine is known for {model}")),
            (_, ReasoningLevel::Numeric(number)) => Err(format!(
                "no numeric effort form is known for {wire:?}; {number} cannot be sent to {model}"
            )),
            (_, ReasoningLevel::On) => Err(format!(
                "no On form is known for {wire:?}; {model} cannot be turned On"
            )),
        }
    }
}

/// A named effort on the OpenAI-family wires.
fn effort_wire(wire: ThinkingWire, effort: &str) -> Value {
    match wire {
        ThinkingWire::OpenAiResponses => {
            json!({"reasoning": {"effort": effort, "summary": "auto"}})
        }
        _ => json!({"reasoning_effort": effort}),
    }
}

/// What a Rhai script receives: the level and the model's capability
/// (MODEL-017: with its declared numeric range and template toggle).
fn rhai_config(level: Value, capability: Option<&ReasoningCapability>) -> Value {
    let capability = capability.map(|capability| {
        json!({
            "efforts": capability.efforts(),
            "budget": capability.budget().map(|range| json!({"min": range.min, "max": range.max})),
            "toggle": capability.has_toggle(),
            "numeric": capability.numeric().map(|range| json!({
                "kind": range.kind(),
                "min": range.min(),
                "max": range.max(),
            })),
            "templateToggle": capability.template_toggle().map(TemplateToggle::key),
        })
    });
    json!({"level": level, "capability": capability})
}

/// A capability's named efforts, or what it takes instead.
fn supported(capability: &ReasoningCapability) -> String {
    match capability.efforts() {
        [] => "no named efforts".to_string(),
        efforts => efforts.join(", "),
    }
}
