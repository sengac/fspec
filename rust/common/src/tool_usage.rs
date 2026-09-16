//! TOOL-023: shared "full usage" text for tool-argument errors.
//!
//! When a tool call fails because of wrong or missing arguments, the error
//! text that lands in the tool result is the LLM's ONLY recovery surface.
//! This module renders:
//!
//! * a compact **accepted-parameter reference** for a tool, derived from
//!   its JSON schema (required vs optional, types, enums/oneOf, defaults),
//! * a **canonical example call** synthesized from the same schema
//!   (const/enum values where known, `<field>` placeholders for strings),
//!
//! so every tool — regardless of provider facade or registration path —
//! returns a self-explanatory argument error.
//!
//! Consumers:
//! * the patched rig-core `ToolDyn` blanket impl (direct tools: WebSearch,
//!   Read, Write, Bash, …) — wraps the serde arg-deserialization failure,
//! * the codelet-tools facade wrappers — appends the usage block to
//!   `ToolError::Validation` raised by `map_params`.

use serde_json::{Map, Value};

/// Error type carrying a pre-rendered argument-recovery message.
///
/// The patched rig-core `ToolDyn` blanket impl boxes this into
/// `ToolError::ToolCallError` so its `Display` output (the recovery text,
/// verbatim) becomes the LLM-visible tool-result error.
#[derive(Debug, Clone, thiserror::Error)]
#[error("{message}")]
pub struct ToolArgRecoveryError {
    /// The fully rendered recovery message (tool name, reason, parameter
    /// reference, example call).
    pub message: String,
}

/// Build the complete recovery message for an argument-deserialization
/// failure on a direct tool (the rig `ToolDyn` path).
///
/// The message names the tool, keeps the original serde reason verbatim
/// (preserving any existing error substrings), then appends the accepted
/// parameter reference and a canonical example call.
pub fn arg_error_recovery(tool_name: &str, reason: &str, schema: &Value) -> String {
    format!(
        "{tool_name} tool: invalid arguments: {reason}\n{usage}",
        usage = usage_block(tool_name, schema)
    )
}

/// Append the accepted-parameter reference and example call to an existing
/// validation message (the facade path, where the original `map_params`
/// message is kept verbatim and the usage block is appended — never
/// replacing).
pub fn append_usage_to_message(tool_name: &str, schema: &Value, message: &str) -> String {
    format!("{message}\n{usage}", usage = usage_block(tool_name, schema))
}

/// Append the accepted-parameter reference and a caller-supplied example
/// call to an existing validation message.
///
/// Some tools have conditionally-required parameters (e.g. Schedule:
/// `action` is always required, but the add action additionally needs
/// `cron`, `timezone`, …). The synthesized example only covers schema
/// `required` fields, so those call sites supply an explicit canonical
/// example that exercises the action in question.
pub fn append_usage_to_message_with_example(
    tool_name: &str,
    schema: &Value,
    message: &str,
    example: &str,
) -> String {
    format!(
        "{message}\n{reference}\nExample call: {example}",
        reference = render_parameter_reference(tool_name, schema)
    )
}

fn usage_block(tool_name: &str, schema: &Value) -> String {
    format!(
        "{}\nExample call: {}",
        render_parameter_reference(tool_name, schema),
        synthesize_example_call(schema)
    )
}

/// Render a compact accepted-parameter reference from a JSON schema.
///
/// Format:
/// ```text
/// Accepted parameters for Read:
///   - file_path (string, required)
///   - limit (integer)
/// ```
/// Properties with an `enum` list their valid values; properties with a
/// `oneOf` list each variant as a synthesized example object.
pub fn render_parameter_reference(tool_name: &str, schema: &Value) -> String {
    let mut out = format!("Accepted parameters for {tool_name}:");
    let Some(props) = schema.get("properties").and_then(Value::as_object) else {
        out.push_str("\n  (no parameter schema available)");
        return out;
    };
    if props.is_empty() {
        out.push_str("\n  (none)");
        return out;
    }

    let required: Vec<String> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    for (name, prop) in props {
        let required_marker = if required.iter().any(|r| r == name) {
            ", required"
        } else {
            ""
        };
        let mut line = format!(
            "\n  - {name} ({type_label}{required_marker})",
            type_label = prop_type(prop, schema)
        );
        if let Some(default) = prop.get("default") {
            line.push_str(&format!(", default: {}", compact_value(default)));
        }
        if let Some(vals) = prop.get("enum").and_then(Value::as_array) {
            let vals: Vec<&str> = vals.iter().filter_map(Value::as_str).collect();
            if !vals.is_empty() {
                line.push_str(&format!(" — one of: {}", vals.join(", ")));
            }
        }
        if let Some(max) = prop.get("maxItems").and_then(Value::as_u64) {
            line.push_str(&format!(", max {max} items"));
        }
        out.push_str(&line);
        if let Some(variants) = prop.get("oneOf").and_then(Value::as_array) {
            out.push_str(" — one of:");
            for variant in variants {
                out.push_str(&format!(
                    "\n    * {}",
                    union_variant_display(variant, schema)
                ));
            }
        }
        // Array properties whose items are objects list the item's fields —
        // the LLM needs the item shape to build a valid call (e.g.
        // request_user_input's `questions` array of question objects).
        if let Some(items_props) = prop
            .get("items")
            .and_then(|i| i.get("properties"))
            .and_then(Value::as_object)
        {
            if !items_props.is_empty() {
                let item_required: Vec<String> = prop
                    .get("items")
                    .and_then(|i| i.get("required"))
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect()
                    })
                    .unwrap_or_default();
                let fields: Vec<String> = items_props
                    .iter()
                    .map(|(n, p)| {
                        let req = if item_required.iter().any(|r| r == n) {
                            ", required"
                        } else {
                            ""
                        };
                        format!("{n} ({}{req})", prop_type(p, schema))
                    })
                    .collect();
                out.push_str(&format!(" — items: {}", fields.join(", ")));
            }
        }
    }
    out
}

/// Synthesize a canonical example call from a JSON schema.
///
/// Only `required` fields are included at the top level; for `oneOf`
/// properties the first variant is chosen and that variant's required
/// fields (plus its optional string fields) are filled in. `const` fields
/// use their const value, `enum` fields use the first enum value, and
/// required strings use a `<field>` placeholder.
pub fn synthesize_example_call(schema: &Value) -> String {
    synthesize_object(schema, false)
}

/// The JSON-schema `type` label for a property, or `"any"`.
///
/// Union properties (`oneOf` / `anyOf` / `allOf`) describe their
/// alternatives instead of collapsing to a bare label: a `oneOf` of
/// object variants (e.g. WebSearch's `action`) reads as `object`, a
/// `oneOf` of bare types (e.g. AgentManager's `session_id`) reads as
/// `string | array of string`, and `anyOf`/`allOf` alternatives are
/// resolved through the schema's `definitions` (e.g. ConnectMCP's
/// `action` reads as `connect | disconnect | list`).
fn prop_type(prop: &Value, schema: &Value) -> String {
    if prop.get("oneOf").is_some() {
        let variants = prop
            .get("oneOf")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let labels: Vec<String> = variants.iter().map(|v| variant_label(v, schema)).collect();
        if labels.iter().all(|l| l == "object") {
            // A `oneOf` of object variants is an object discriminated by
            // variant (e.g. WebSearch's `action`).
            return "object".to_string();
        }
        return labels.join(" | ");
    }
    if let Some(alts) = prop
        .get("anyOf")
        .or_else(|| prop.get("allOf"))
        .and_then(Value::as_array)
    {
        let labels: Vec<String> = alts.iter().map(|v| variant_label(v, schema)).collect();
        if !labels.is_empty() {
            return labels.join(" | ");
        }
    }
    if let Some(t) = prop.get("type").and_then(Value::as_str) {
        return t.to_string();
    }
    prop.get("type")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join("|")
        })
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "any".to_string())
}

/// The compact display label for a union variant.
///
/// A bare-type variant (`{"type":"string"}`) renders its type; an
/// array variant renders `array of <item type>`; an enum variant
/// renders its enum values joined; anything with a `oneOf` (e.g. a
/// `$ref` to a oneOf enum definition) resolves to that union's labels;
/// an object variant renders as `object`. Never an empty object
/// placeholder.
fn variant_label(variant: &Value, schema: &Value) -> String {
    // Resolve $ref (e.g. schemars' `{"$ref": "#/definitions/Foo"}`).
    let resolved = resolve_ref(variant, schema);
    if let Some(inner) = resolved.get("oneOf").and_then(Value::as_array) {
        let labels: Vec<String> = inner.iter().map(|v| variant_label(v, schema)).collect();
        if !labels.is_empty() {
            return labels.join(" | ");
        }
    }
    if let Some(vals) = resolved.get("enum").and_then(Value::as_array) {
        let strs: Vec<&str> = vals.iter().filter_map(Value::as_str).collect();
        if !strs.is_empty() {
            return strs.join(" | ");
        }
    }
    if resolved.get("properties").is_some() {
        return "object".to_string();
    }
    match resolved.get("type").and_then(Value::as_str) {
        Some("array") => match resolved
            .get("items")
            .and_then(|i| i.get("type"))
            .and_then(Value::as_str)
        {
            Some(item) => format!("array of {item}"),
            None => "array".to_string(),
        },
        Some(t) => t.to_string(),
        None => match resolved.get("type").and_then(Value::as_array) {
            Some(types) => {
                let joined = types
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("|");
                if joined.is_empty() {
                    "any".to_string()
                } else {
                    joined
                }
            }
            None => "any".to_string(),
        },
    }
}

/// Render the display text for a `oneOf` variant bullet: a synthesized
/// example object for object variants, a compact type/enum label for
/// bare-type variants — never an empty `{}` placeholder.
fn union_variant_display(variant: &Value, schema: &Value) -> String {
    if variant.get("properties").is_some() {
        return synthesize_object(variant, true);
    }
    variant_label(variant, schema)
}

/// Resolve a JSON-schema `$ref` (e.g. `#/definitions/Foo`) against the
/// schema's top-level `definitions` map. Returns `self` when the value
/// has no `$ref` or the reference cannot be resolved.
fn resolve_ref<'a>(value: &'a Value, schema: &'a Value) -> &'a Value {
    const MAX_DEPTH: usize = 8;
    let mut current = value;
    for _ in 0..MAX_DEPTH {
        let Some(ref_path) = current.get("$ref").and_then(Value::as_str) else {
            return current;
        };
        let Some(name) = ref_path.strip_prefix("#/definitions/") else {
            return current;
        };
        let Some(def) = schema.get("definitions").and_then(|d| d.get(name)) else {
            return current;
        };
        current = def;
    }
    current
}

fn synthesize_object(schema: &Value, include_optional_strings: bool) -> String {
    let mut obj = Map::new();
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        let required: Vec<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for (name, prop) in props {
            let is_required = required.contains(&name.as_str());
            if let Some(value) = synthesize_value(prop, name, is_required, include_optional_strings)
            {
                obj.insert(name.clone(), value);
            }
        }
    }
    match serde_json::to_string(&Value::Object(obj)) {
        Ok(s) => s,
        Err(_) => "{}".to_string(),
    }
}

fn synthesize_value(
    prop: &Value,
    name: &str,
    is_required: bool,
    include_optional_strings: bool,
) -> Option<Value> {
    // oneOf: pick the first variant and synthesize from it.
    if let Some(variants) = prop.get("oneOf").and_then(Value::as_array) {
        return variants
            .first()
            .and_then(|v| synthesize_value(v, name, is_required, true));
    }
    // A const value is the canonical example value.
    if let Some(c) = prop.get("const") {
        return Some(c.clone());
    }
    // An enum's first value is the canonical example value.
    if let Some(e) = prop
        .get("enum")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
    {
        return Some(e.clone());
    }
    if !is_required {
        // Optional: only include explicit defaults, or string placeholders
        // when the caller opts in (variant-level examples).
        if let Some(d) = prop.get("default") {
            return Some(d.clone());
        }
        if include_optional_strings && prop.get("type").and_then(Value::as_str) == Some("string") {
            return Some(Value::String(format!("<{name}>")));
        }
        return None;
    }
    // Required: synthesize a placeholder by type.
    match prop.get("type").and_then(Value::as_str) {
        Some("string") => Some(Value::String(format!("<{name}>"))),
        Some("integer") | Some("number") => Some(Value::from(0)),
        Some("boolean") => Some(Value::Bool(true)),
        Some("array") => Some(Value::Array(Vec::new())),
        Some("object") => Some(Value::Object(synthesized_map(
            prop,
            include_optional_strings,
        ))),
        _ => Some(Value::String(format!("<{name}>"))),
    }
}

fn synthesized_map(schema: &Value, include_optional_strings: bool) -> Map<String, Value> {
    let mut obj = Map::new();
    if let Some(props) = schema.get("properties").and_then(Value::as_object) {
        let required: Vec<&str> = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default();
        for (name, prop) in props {
            let is_required = required.contains(&name.as_str());
            if let Some(value) = synthesize_value(prop, name, is_required, include_optional_strings)
            {
                obj.insert(name.clone(), value);
            }
        }
    }
    obj
}

fn compact_value(v: &Value) -> String {
    match serde_json::to_string(v) {
        Ok(s) => s,
        Err(_) => "?".to_string(),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    // A WebSearch-style schema: single required `action` property with a
    // oneOf of four variants, each with a const `type` discriminator.
    fn web_search_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "object",
                    "oneOf": [
                        {
                            "type": "object",
                            "properties": {
                                "type": {"const": "search"},
                                "query": {"type": "string"}
                            },
                            "required": ["type"]
                        },
                        {
                            "type": "object",
                            "properties": {
                                "type": {"const": "open_page"},
                                "url": {"type": "string"}
                            },
                            "required": ["type", "url"]
                        }
                    ]
                }
            },
            "required": ["action"]
        })
    }

    // A Read-style schema: one required string, optional number/string.
    fn read_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"},
                "offset": {"type": "integer"},
                "limit": {"type": "integer"},
                "pdf_mode": {"type": "string"}
            },
            "required": ["file_path"]
        })
    }

    #[test]
    fn render_reference_lists_required_and_optional_with_types() {
        let out = render_parameter_reference("Read", &read_schema());
        assert!(out.starts_with("Accepted parameters for Read:"), "{out}");
        assert!(out.contains("file_path (string, required)"), "{out}");
        assert!(out.contains("limit (integer)"), "{out}");
        assert!(out.contains("pdf_mode (string)"), "{out}");
        assert!(!out.contains("limit (string, required)"), "{out}");
    }

    #[test]
    fn render_reference_lists_enum_values() {
        let schema = json!({
            "type": "object",
            "properties": {
                "action_type": {"type": "string", "enum": ["search", "open_page"]}
            },
            "required": ["action_type"]
        });
        let out = render_parameter_reference("WebSearch", &schema);
        assert!(out.contains("one of: search, open_page"), "{out}");
    }

    #[test]
    fn render_reference_lists_oneof_variants() {
        let out = render_parameter_reference("WebSearch", &web_search_schema());
        assert!(out.contains("action (object, required)"), "{out}");
        assert!(
            out.contains("\"const\": \"search\"") || out.contains("search"),
            "{out}"
        );
        assert!(out.contains("open_page"), "{out}");
    }

    #[test]
    fn example_call_uses_const_and_placeholder_values() {
        let example = synthesize_example_call(&web_search_schema());
        // First variant chosen: const `type` value + `<query>` placeholder.
        assert!(example.contains("\"search\""), "{example}");
        assert!(example.contains("<query>"), "{example}");
        assert!(example.contains("action"), "{example}");
    }

    #[test]
    fn example_call_includes_only_required_fields_at_top_level() {
        let example = synthesize_example_call(&read_schema());
        assert!(example.contains("\"file_path\""), "{example}");
        assert!(!example.contains("\"offset\""), "{example}");
        assert!(!example.contains("\"limit\""), "{example}");
    }

    #[test]
    fn arg_error_recovery_names_tool_and_keeps_reason() {
        let msg = arg_error_recovery(
            "WebSearch",
            "missing field `action` at line 1 column 2",
            &web_search_schema(),
        );
        assert!(msg.contains("WebSearch tool: invalid arguments:"), "{msg}");
        assert!(
            msg.contains("missing field `action` at line 1 column 2"),
            "{msg}"
        );
        assert!(msg.contains("Accepted parameters for WebSearch:"), "{msg}");
        assert!(msg.contains("Example call:"), "{msg}");
    }

    #[test]
    fn append_usage_preserves_original_message_verbatim() {
        let original = "Missing 'file_path' field".to_string();
        let msg = append_usage_to_message("read_file", &read_schema(), &original);
        assert!(
            msg.starts_with("Missing 'file_path' field"),
            "original message must stay the prefix: {msg}"
        );
        assert!(msg.contains("Accepted parameters for read_file:"), "{msg}");
        assert!(msg.contains("Example call:"), "{msg}");
    }

    #[test]
    fn schemaless_input_yields_graceful_reference() {
        let out = render_parameter_reference("Mystery", &json!({}));
        assert!(out.contains("(no parameter schema available)"), "{out}");
        assert_eq!(synthesize_example_call(&json!({})), "{}");
    }

    // TOOL-024: the schema shape below mirrors AgentManager's registered
    // schema — a required `session_id` property with a oneOf of a bare
    // string and an array of strings.
    fn agent_manager_style_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn", "get_status"]
                },
                "session_id": {
                    "oneOf": [
                        { "type": "string" },
                        { "type": "array", "items": { "type": "string" } }
                    ]
                }
            },
            "required": ["action"]
        })
    }

    // TOOL-024: mirrors the schemars-generated ConnectMCP schema — an
    // `allOf`/$ref property and an `anyOf` property whose alternatives
    // resolve to oneOf enum definitions.
    fn connect_mcp_style_schema() -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "default": "connect",
                    "allOf": [{ "$ref": "#/definitions/Action" }]
                },
                "transport": {
                    "anyOf": [
                        { "$ref": "#/definitions/Transport" },
                        { "type": "null" }
                    ]
                }
            },
            "definitions": {
                "Action": {
                    "oneOf": [
                        { "type": "string", "enum": ["connect"] },
                        { "type": "string", "enum": ["disconnect"] },
                        { "type": "string", "enum": ["list"] }
                    ]
                },
                "Transport": {
                    "oneOf": [
                        { "type": "string", "enum": ["stdio"] },
                        { "type": "string", "enum": ["http"] }
                    ]
                }
            }
        })
    }

    #[test]
    fn oneof_non_object_variants_render_type_labels_not_empty_objects() {
        // Feature: spec/features/post-deserialization-arg-errors-include-recovery-guidance-tool-023-follow-up.feature
        // Scenario: oneOf with non-object variants renders each variant type
        // @step Given a tool whose schema declares a required property 'session_id' as oneOf string or array of string
        let schema = agent_manager_style_schema();
        assert!(
            schema["properties"]["session_id"]["oneOf"].is_array(),
            "fixture must declare a oneOf union"
        );

        // @step When I call the tool with session_id missing
        // (the parameter reference is the recovery block's parameter list)
        let out = render_parameter_reference("AgentManager", &schema);

        // @step Then the parameter reference lists session_id with both allowed types (string and array of string)
        assert!(
            out.contains("session_id (string | array of string"),
            "both allowed types must be listed: {out}"
        );

        // @step And the parameter reference contains no empty object placeholder for a variant that has no properties
        assert!(!out.contains("* {}"), "no empty-object placeholder: {out}");
    }

    #[test]
    fn anyof_and_ref_properties_render_alternatives() {
        // Feature: spec/features/post-deserialization-arg-errors-include-recovery-guidance-tool-023-follow-up.feature
        // Scenario: anyOf properties render their alternatives
        // @step Given a tool whose schema declares properties 'action' and 'transport' as anyOf unions
        let schema = connect_mcp_style_schema();

        // @step When I call the tool with an invalid 'action' value
        // (invalid-enum failures surface the same parameter reference)
        let out = render_parameter_reference("ConnectMCP", &schema);

        // @step Then the parameter reference describes each alternative of 'action'
        assert!(
            out.contains("connect") && out.contains("disconnect") && out.contains("list"),
            "each action alternative must be described: {out}"
        );
        assert!(
            out.contains("stdio") && out.contains("http"),
            "each transport alternative must be described: {out}"
        );

        // @step And the parameter reference does not render 'action' or 'transport' as bare '(any)' with no sub-values
        assert!(!out.contains("(any)"), "no bare '(any)': {out}");
    }

    #[test]
    fn array_property_with_object_items_lists_item_fields() {
        // TOOL-024: request_user_input's `questions` — an array whose items
        // are objects with required fields. The recovery surface must show
        // the item shape so the LLM can build a valid call.
        let schema = json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 3,
                    "items": {
                        "type": "object",
                        "required": ["id", "header", "question"],
                        "properties": {
                            "id": {"type": "string"},
                            "header": {"type": "string"},
                            "question": {"type": "string"},
                            "options": {"type": "array"}
                        }
                    }
                }
            },
            "required": ["questions"]
        });
        let out = render_parameter_reference("request_user_input", &schema);
        assert!(
            out.contains("questions (array, required)"),
            "array type must be listed: {out}"
        );
        assert!(
            out.contains("max 3 items"),
            "maxItems must be surfaced: {out}"
        );
        assert!(
            out.contains("items: id (string, required)"),
            "item fields must be listed: {out}"
        );
        assert!(
            out.contains("options (array)"),
            "optional item fields must be listed: {out}"
        );
    }
}
