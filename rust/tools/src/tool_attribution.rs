use rig::agent::Agent;
use rig::completion::{CompletionModel, ToolDefinition};
use rig::tool::{
    server::{ToolServerError, ToolServerHandle},
    ToolError, ToolMiddleware,
};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
#[error("tool {tool}: attribution schema incompatible: {reason}")]
pub struct AttributionSchemaError {
    pub tool: String,
    pub reason: String,
}

pub fn decorate_definition(
    mut definition: ToolDefinition,
) -> Result<ToolDefinition, AttributionSchemaError> {
    let fail = |reason: &str| AttributionSchemaError {
        tool: definition.name.clone(),
        reason: reason.into(),
    };
    if references_root(&definition.parameters) {
        return Err(fail(
            "references cannot be safely extended with work_unit_id; reference an isolated local business $defs or definitions schema instead of root, anchored or external schemas",
        ));
    }
    let schema = definition
        .parameters
        .as_object_mut()
        .ok_or_else(|| fail("expected an object argument schema"))?;
    if schema.get("nullable") == Some(&json!(true)) {
        return Err(fail(
            "nullable root arguments bypass required work_unit_id attribution; provide a non-null object schema",
        ));
    }
    if schema.get("type").is_some_and(|kind| kind != "object") {
        return Err(fail(
            "only object argument schemas support reserved work_unit_id attribution",
        ));
    }
    for keyword in [
        "$ref",
        "$dynamicRef",
        "$recursiveRef",
        "allOf",
        "anyOf",
        "oneOf",
        "not",
        "if",
        "then",
        "else",
        "propertyNames",
        "patternProperties",
        "unevaluatedProperties",
        "maxProperties",
        "minProperties",
        "dependentSchemas",
        "dependentRequired",
        "dependencies",
        "enum",
        "const",
    ] {
        if schema.contains_key(keyword) {
            return Err(fail(&format!(
                "unsupported root constraint {keyword}; supply an extensible object schema with business references inside properties"
            )));
        }
    }
    if schema.get("required").is_some_and(|required| {
        !required
            .as_array()
            .is_some_and(|required| required.iter().all(Value::is_string))
    }) {
        return Err(fail("required must be an array of business property names"));
    }
    if schema
        .get("required")
        .and_then(Value::as_array)
        .is_some_and(|required| required.contains(&json!("work_unit_id")))
    {
        return Err(fail(
            "reserved work_unit_id requirement collides with business arguments; use target_work_unit_id for the business target",
        ));
    }
    let properties = schema
        .entry("properties")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| fail("properties must be an object"))?;
    if properties.contains_key("work_unit_id") {
        return Err(fail(
            "reserved work_unit_id property collides with business arguments; use target_work_unit_id for the business target",
        ));
    }
    properties.insert("work_unit_id".into(), json!({
        "type":"string", "minLength":1,
        "description":"Required harness attribution for the calling session, never a spawned or targeted session: an existing work-unit ID in the invoking project, or exact NONE to explicitly clear the caller's activity (not retain it). Applies even when managing another session. Omission, null and empty strings are invalid. This is not a business target, child assignment or permission change."
    }));
    schema.insert("type".into(), json!("object"));
    let required = schema
        .entry("required")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| fail("required must be an array"))?;
    required.push(json!("work_unit_id"));
    Ok(definition)
}

fn references_root(value: &Value) -> bool {
    match value {
        Value::Object(object) => {
            object
                .get("$ref")
                .and_then(Value::as_str)
                .is_some_and(|reference| {
                    !reference.starts_with("#/$defs/") && !reference.starts_with("#/definitions/")
                })
                || object.contains_key("$dynamicRef")
                || object.contains_key("$recursiveRef")
                || object.values().any(references_root)
        }
        Value::Array(array) => array.iter().any(references_root),
        _ => false,
    }
}

pub struct SchemaMiddleware {
    scope: String,
}

impl SchemaMiddleware {
    pub fn new(scope: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
        }
    }
}

impl ToolMiddleware for SchemaMiddleware {
    fn scope_id(&self) -> &str {
        &self.scope
    }

    fn decorate_definition(&self, definition: ToolDefinition) -> Result<ToolDefinition, ToolError> {
        decorate_definition(definition).map_err(|error| ToolError::ToolCallError(Box::new(error)))
    }
}

pub async fn configure_tool_server(
    handle: &ToolServerHandle,
    scope: impl Into<String>,
) -> Result<(), ToolServerError> {
    handle
        .configure_middleware(Arc::new(crate::tool_admission::AdmissionMiddleware::new(
            scope,
        )))
        .await
}

pub async fn configure_agent_tools<M: CompletionModel + 'static>(
    agent: &Agent<M>,
    scope: impl Into<String>,
) -> Result<(), ToolServerError> {
    configure_tool_server(&agent.tool_server_handle, scope).await
}
