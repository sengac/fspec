use crate::execution_context::{scope_invocation, InvocationReceipt, WorkUnitAttribution};
use rig::{
    completion::ToolDefinition,
    tool::{ToolDyn, ToolError, ToolMiddleware},
    wasm_compat::WasmBoxedFuture,
};
use serde::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;

mod auxiliary;
mod registry;
pub use auxiliary::{register_auxiliary, register_auxiliary_for_owner, AuxiliaryScope};
pub use registry::{
    authorization_identity, register_session, unregister_session, AdmissionHandler,
    ContextValidator,
};

#[derive(Debug, thiserror::Error)]
#[error("invalid work_unit_id attribution: {0}. Supply top-level work_unit_id: \"an existing card in the invoking project, or NONE\"; business session/project targets cannot select the invoking context")]
pub struct AttributionError(pub String);

#[derive(Deserialize)]
struct Arguments {
    work_unit_id: String,
    #[serde(flatten)]
    business: Map<String, Value>,
}

pub fn split_arguments(args: &str) -> Result<(WorkUnitAttribution, String), AttributionError> {
    let arguments: Arguments = serde_json::from_str(args)
        .map_err(|error| AttributionError(format!("missing or malformed work_unit_id: {error}")))?;
    let id = arguments.work_unit_id;
    if id.is_empty() || id.trim() != id {
        return Err(AttributionError(
            "card ID must be a nonempty unpadded string".into(),
        ));
    }
    let attribution = if id == "NONE" {
        WorkUnitAttribution::None
    } else {
        WorkUnitAttribution::Card(id)
    };
    Ok((attribution, Value::Object(arguments.business).to_string()))
}

tokio::task_local! {
    static AUDIT_ARGUMENTS: String;
}

pub fn current_audit_arguments() -> Option<String> {
    AUDIT_ARGUMENTS.try_with(Clone::clone).ok()
}

pub struct AdmissionMiddleware {
    scope: String,
}

impl AdmissionMiddleware {
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
        }
    }
}

impl ToolMiddleware for AdmissionMiddleware {
    fn scope_id(&self) -> &str {
        &self.scope
    }

    fn decorate_definition(&self, definition: ToolDefinition) -> Result<ToolDefinition, ToolError> {
        crate::tool_attribution::decorate_definition(definition)
            .map_err(|error| ToolError::ToolCallError(Box::new(error)))
    }

    fn call<'a>(
        &'a self,
        tool: &'a dyn ToolDyn,
        args: String,
    ) -> WasmBoxedFuture<'a, Result<String, ToolError>> {
        Box::pin(async move {
            let (attribution, business) = split_arguments(&args).map_err(tool_error)?;
            let id = Uuid::parse_str(&self.scope).map_err(|error| {
                tool_error(AttributionError(format!(
                    "unknown invoking context: {error}"
                )))
            })?;
            let receipt = registry::admit(id, attribution).map_err(tool_error)?;
            AUDIT_ARGUMENTS
                .scope(args, scope_invocation(receipt, tool.call(business)))
                .await
        })
    }
}

fn tool_error(error: AttributionError) -> ToolError {
    ToolError::ToolCallError(Box::new(error))
}

pub(crate) fn validate_card(
    project: &std::path::Path,
    attribution: &WorkUnitAttribution,
) -> Result<(), AttributionError> {
    match attribution {
        WorkUnitAttribution::Card(id) => {
            let bytes = std::fs::read(project.join("spec/work-units.json")).map_err(|error| {
                AttributionError(format!("cannot read trusted project cards: {error}"))
            })?;
            let snapshot: Value = serde_json::from_slice(&bytes).map_err(|error| {
                AttributionError(format!("cannot parse trusted project cards: {error}"))
            })?;
            let exists = snapshot["workUnits"]
                .as_object()
                .and_then(|cards| cards.get(id));
            if exists.is_none_or(|card| card["id"] != id.as_str() || card["deleted"] == true) {
                return Err(AttributionError(format!(
                    "unknown card {id} in invoking project"
                )));
            }
            Ok(())
        }
        WorkUnitAttribution::None => Ok(()),
        WorkUnitAttribution::Absent => Err(AttributionError("attribution is required".into())),
    }
}
