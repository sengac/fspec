//! MODEL-016: the reasoning a local inference server's model declares in its
//! local-server profile, the engine facts that decide which declared parts
//! can be sent, and the live check against the engine's own report.
//!
//! Engine facts are read from each engine's source:
//! spec/attachments/MODEL-016/local-engine-reasoning-research.md.
//!
//! Feature: spec/features/local-inference-server-thinking-levels.feature

mod declaration;
mod live_check;

pub use declaration::{
    NumericEffortsDeclaration, ThinkingLevelsDeclaration, TokenBudgetDeclaration,
};
pub use live_check::{LiveCheck, LiveCheckReport};

use crate::models::{NumericKind, NumericRange, ReasoningCapability, TemplateToggle};

/// The inference engine a local-server profile names (`engine`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalEngine {
    Vllm,
    Sglang,
    Strata,
    LlamaCpp,
    Ollama,
    LmStudio,
    Tgi,
    TensorRtLlm,
    Lmdeploy,
}

const ENGINES: [LocalEngine; 9] = [
    LocalEngine::Vllm,
    LocalEngine::Sglang,
    LocalEngine::Strata,
    LocalEngine::LlamaCpp,
    LocalEngine::Ollama,
    LocalEngine::LmStudio,
    LocalEngine::Tgi,
    LocalEngine::TensorRtLlm,
    LocalEngine::Lmdeploy,
];

/// The request field an engine reads a token budget from (research §0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetField {
    /// `thinking_token_budget` (vLLM, TensorRT-LLM).
    ThinkingTokenBudget,
    /// `reasoning_budget_tokens` (llama.cpp, Strata).
    ReasoningBudgetTokens,
    /// `custom_params.thinking_budget` (SGLang).
    CustomParamsThinkingBudget,
}

impl LocalEngine {
    /// The engine a profile's `engine` value names; `None` when it names
    /// none of them.
    pub fn from_config(id: &str) -> Option<Self> {
        ENGINES.into_iter().find(|engine| engine.config_id() == id)
    }

    /// The profile's `engine` value for this engine.
    pub fn config_id(self) -> &'static str {
        match self {
            Self::Vllm => "vllm",
            Self::Sglang => "sglang",
            Self::Strata => "strata",
            Self::LlamaCpp => "llama-cpp",
            Self::Ollama => "ollama",
            Self::LmStudio => "lm-studio",
            Self::Tgi => "tgi",
            Self::TensorRtLlm => "tensorrt-llm",
            Self::Lmdeploy => "lmdeploy",
        }
    }

    /// The engine's name, as errors and reports give it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Vllm => "vLLM",
            Self::Sglang => "SGLang",
            Self::Strata => "Strata",
            Self::LlamaCpp => "llama.cpp",
            Self::Ollama => "Ollama",
            Self::LmStudio => "LM Studio",
            Self::Tgi => "TGI",
            Self::TensorRtLlm => "TensorRT-LLM",
            Self::Lmdeploy => "LMDeploy",
        }
    }

    /// Every engine's `engine` value, for errors.
    pub fn config_ids() -> String {
        ENGINES.map(Self::config_id).join(", ")
    }

    /// Named and numeric efforts go in `reasoning_effort` on every engine
    /// but TGI, which takes no reasoning parameter at all (research §7).
    pub fn takes_efforts(self) -> bool {
        self != Self::Tgi
    }

    /// The numeric efforts the engine takes: decimals 0.0 to 0.99 on SGLang
    /// and whole numbers 1 to 100 on Strata; none elsewhere.
    pub fn numeric_efforts(self) -> Result<Option<NumericRange>, String> {
        let range = |kind, min: serde_json::Number, max: serde_json::Number| {
            NumericRange::new(kind, min, max).map(Some)
        };
        match self {
            Self::Sglang => match (
                serde_json::Number::from_f64(0.0),
                serde_json::Number::from_f64(0.99),
            ) {
                (Some(min), Some(max)) => range(NumericKind::Decimal, min, max),
                _ => Err("SGLang's decimal effort bounds are not JSON numbers".to_string()),
            },
            Self::Strata => range(NumericKind::Whole, 1.into(), 100.into()),
            _ => Ok(None),
        }
    }

    /// The field the engine reads a token budget from; Ollama, LM Studio,
    /// LMDeploy and TGI have none.
    pub fn budget_field(self) -> Option<BudgetField> {
        match self {
            Self::Vllm | Self::TensorRtLlm => Some(BudgetField::ThinkingTokenBudget),
            Self::LlamaCpp | Self::Strata => Some(BudgetField::ReasoningBudgetTokens),
            Self::Sglang => Some(BudgetField::CustomParamsThinkingBudget),
            Self::Ollama | Self::LmStudio | Self::Lmdeploy | Self::Tgi => None,
        }
    }

    /// Whether the engine passes `toggle` to the chat template: not Ollama,
    /// LM Studio or TGI, and only `enable_thinking` on Strata.
    pub fn takes_toggle(self, toggle: TemplateToggle) -> bool {
        match self {
            Self::Ollama | Self::LmStudio | Self::Tgi => false,
            Self::Strata => toggle == TemplateToggle::EnableThinking,
            _ => true,
        }
    }
}

/// A local profile model's declared reasoning, resolved against its engine.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalReasoning {
    pub profile: String,
    pub engine: LocalEngine,
    pub capability: ReasoningCapability,
}

/// The reasoning `profile`'s `model` declares: its own declaration, else the
/// profile-wide one (a per-model declaration replaces the profile-wide one;
/// they are not merged). `Ok(None)` when neither declares anything: the
/// model has no reasoning control. The engine is never inferred from the
/// base URL or the model id.
pub fn resolve_local_reasoning(
    profile: &str,
    model: &str,
    engine: Option<&str>,
    profile_wide: Option<&ThinkingLevelsDeclaration>,
    per_model: Option<&ThinkingLevelsDeclaration>,
) -> Result<Option<LocalReasoning>, String> {
    let engine = engine
        .map(|id| {
            LocalEngine::from_config(id).ok_or_else(|| {
                format!(
                    "profile {profile} names the unknown engine {id:?}; the engine is one of {}",
                    LocalEngine::config_ids()
                )
            })
        })
        .transpose()?;
    let Some(declaration) = per_model.or(profile_wide) else {
        return Ok(None);
    };
    let Some(engine) = engine else {
        return Err(format!(
            "profile {profile} declares thinking levels for model {model} but names no engine; \
             set its \"engine\" to one of {} (no engine is inferred from the base URL or the \
             model id)",
            LocalEngine::config_ids()
        ));
    };
    declaration.resolve(profile, model, engine).map(Some)
}
