use codelet_tools::ToolDefinition;
use serde_json::Value;

use crate::ProviderError;

pub(crate) fn attributed_tools(
    tools: &[ToolDefinition],
) -> Result<Vec<ToolDefinition>, ProviderError> {
    crate::convert_tools_to_rig(tools).map(|definitions| {
        definitions
            .into_iter()
            .map(|definition| ToolDefinition {
                name: definition.name,
                description: definition.description,
                input_schema: definition.parameters,
            })
            .collect()
    })
}

pub(crate) fn validate_catalog(
    body: &Value,
    expected: &[ToolDefinition],
) -> Result<(), ProviderError> {
    if body.get("functions").is_some() || body.get("web_search_options").is_some() {
        return Err(invalid(
            "unsupported executable catalog outside attributed tools",
        ));
    }
    let Some(catalog) = body.get("tools") else {
        return if expected.is_empty() {
            Ok(())
        } else {
            Err(invalid("missing tool catalog"))
        };
    };
    let mut definitions = Vec::new();
    collect(catalog, &mut definitions)?;
    if definitions.len() != expected.len() {
        return Err(invalid("tool catalog changed after schema decoration"));
    }
    let mut seen = std::collections::HashSet::new();
    for (name, parameters) in definitions {
        let original = expected
            .iter()
            .find(|tool| tool.name == name)
            .ok_or_else(|| invalid(format!("unsupported executable tool {name}")))?;
        if !seen.insert(name) {
            return Err(invalid(format!("duplicate executable tool {name}")));
        }
        validate_parameters(parameters)?;
        if parameters != &original.input_schema {
            return Err(invalid(format!(
                "tool {name} modified its business or attribution schema; supported dialect conversions may change catalog wrappers, not parameter schemas"
            )));
        }
    }
    Ok(())
}

pub(crate) fn validate_parameters(parameters: &Value) -> Result<(), ProviderError> {
    let attribution = &parameters["properties"]["work_unit_id"];
    if parameters["type"] != "object"
        || parameters["nullable"] == true
        || !requires(parameters, "work_unit_id")
        || attribution["type"] != "string"
        || attribution["nullable"] == true
    {
        return Err(invalid(
            "tool lost mandatory non-null top-level work_unit_id attribution",
        ));
    }
    Ok(())
}

fn requires(schema: &Value, name: &str) -> bool {
    schema["required"]
        .as_array()
        .is_some_and(|required| required.iter().any(|value| value == name))
}

fn collect<'a>(
    catalog: &'a Value,
    output: &mut Vec<(&'a str, &'a Value)>,
) -> Result<(), ProviderError> {
    let entries = catalog
        .as_array()
        .ok_or_else(|| invalid("tool catalog must be an array"))?;
    for entry in entries {
        if let Some(functions) = entry
            .get("functionDeclarations")
            .or_else(|| entry.get("function_declarations"))
        {
            if entry.as_object().is_none_or(|object| object.len() != 1) {
                return Err(invalid(
                    "unsupported native capability alongside function declarations",
                ));
            }
            collect(functions, output)?;
            continue;
        }
        if entry.get("type").is_some_and(|kind| kind != "function") {
            return Err(invalid(
                "unsupported executable tool catalog; use attributed JSON functions",
            ));
        }
        let definition = entry.get("function").unwrap_or(entry);
        let name = definition["name"]
            .as_str()
            .ok_or_else(|| invalid("tool name is missing"))?;
        let mut schemas = ["parameters", "input_schema", "parametersJsonSchema"]
            .into_iter()
            .filter_map(|key| definition.get(key));
        let parameters = schemas
            .next()
            .ok_or_else(|| invalid(format!("tool {name} has no JSON schema")))?;
        if schemas.next().is_some() {
            return Err(invalid(format!(
                "tool {name} has ambiguous parameter schemas"
            )));
        }
        output.push((name, parameters));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> ProviderError {
    ProviderError::config("tool attribution", message)
}
