mod types;

pub use types::*;

use serde_json::{json, Map, Value};

impl super::CodexProvider {
    pub fn set_capabilities(
        &self,
        capabilities: Option<CodexCapabilities>,
    ) -> Result<(), RequestOptionsError> {
        self.completion_model.set_capabilities(capabilities)
    }
}

pub fn resolve(
    configuration: Option<&Value>,
    capabilities: Option<&CodexCapabilities>,
) -> Result<Value, RequestOptionsError> {
    let fallback = CodexCapabilities::conservative();
    let capabilities = capabilities.filter(|caps| !caps.stale).unwrap_or(&fallback);
    let explicit = configuration
        .map(CodexRequestOptions::parse)
        .transpose()?
        .unwrap_or_default();
    let defaults = &capabilities.defaults;
    let mut output = explicit.additional.clone();
    let mut reasoning = Map::new();
    let explicit_reasoning = explicit.reasoning.as_ref();
    let default_reasoning = defaults.reasoning.as_ref();
    select(
        &mut reasoning,
        "effort",
        "reasoning.effort",
        explicit_reasoning
            .and_then(|value| value.effort.as_ref())
            .map(Effort::wire),
        default_reasoning
            .and_then(|value| value.effort.as_ref())
            .map(Effort::wire),
        capabilities.efforts.iter().map(Effort::wire).collect(),
    )?;
    select(
        &mut reasoning,
        "context",
        "reasoning.context",
        explicit_reasoning
            .and_then(|value| value.context.as_ref())
            .map(|value| json!(value)),
        default_reasoning
            .and_then(|value| value.context.as_ref())
            .map(|value| json!(value)),
        values(&capabilities.contexts),
    )?;
    select(
        &mut reasoning,
        "summary",
        "reasoning.summary",
        explicit_reasoning
            .and_then(|value| value.summary.as_ref())
            .map(|value| json!(value)),
        default_reasoning
            .and_then(|value| value.summary.as_ref())
            .map(|value| json!(value)),
        values(&capabilities.summaries),
    )?;
    if !reasoning.is_empty() {
        output.insert("reasoning".into(), Value::Object(reasoning));
    }
    let mut text = Map::new();
    select(
        &mut text,
        "verbosity",
        "text.verbosity",
        explicit
            .text
            .as_ref()
            .and_then(|value| value.verbosity.as_ref())
            .map(|value| json!(value)),
        defaults
            .text
            .as_ref()
            .and_then(|value| value.verbosity.as_ref())
            .map(|value| json!(value)),
        values(&capabilities.verbosities),
    )?;
    if let Some(format) = explicit.text.as_ref().and_then(|text| text.format.as_ref()) {
        text.insert("format".into(), format.clone());
    }
    if !text.is_empty() {
        output.insert("text".into(), Value::Object(text));
    }
    select(
        &mut output,
        "service_tier",
        "service_tier",
        explicit.service_tier.as_ref().map(|value| json!(value)),
        defaults.service_tier.as_ref().map(|value| json!(value)),
        values(&capabilities.service_tiers),
    )?;
    let mut stream = Map::new();
    select(
        &mut stream,
        "reasoning_summary_delivery",
        "stream_options.reasoning_summary_delivery",
        explicit
            .stream_options
            .as_ref()
            .map(|value| json!(value.reasoning_summary_delivery)),
        defaults
            .stream_options
            .as_ref()
            .map(|value| json!(value.reasoning_summary_delivery)),
        if capabilities.sequential_cutoff {
            vec![json!("sequential_cutoff")]
        } else {
            vec![]
        },
    )?;
    if !stream.is_empty() {
        output.insert("stream_options".into(), Value::Object(stream));
    }
    let mut include = explicit.include.unwrap_or_default();
    if !include
        .iter()
        .any(|item| item == "reasoning.encrypted_content")
    {
        include.push("reasoning.encrypted_content".into());
    }
    output.insert("include".into(), json!(include));
    output.insert("store".into(), Value::Bool(false));
    Ok(Value::Object(output))
}

fn values(values: &[String]) -> Vec<Value> {
    values.iter().map(|value| json!(value)).collect()
}

fn select(
    output: &mut Map<String, Value>,
    key: &str,
    path: &str,
    explicit: Option<Value>,
    default: Option<Value>,
    supported: Vec<Value>,
) -> Result<(), RequestOptionsError> {
    if let Some(value) = explicit {
        if !supported.contains(&value) {
            return Err(RequestOptionsError::Unsupported {
                option: path.into(),
                value,
                supported,
            });
        }
        output.insert(key.into(), value);
    } else if let Some(value) = default.filter(|value| supported.contains(value)) {
        output.insert(key.into(), value);
    }
    Ok(())
}
