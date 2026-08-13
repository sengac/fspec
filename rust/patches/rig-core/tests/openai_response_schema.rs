use rig::completion::ToolDefinition;
use rig::providers::openai::responses_api::ResponsesToolDefinition;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

//************** For the first test **************
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Person {
    #[schemars(required)]
    pub first_name: Option<String>,
    #[schemars(required)]
    pub last_name: Option<String>,
    pub job: Job,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Job {
    inner: String,
    department: Department,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Department {
    name: String,
}
//************** For the second test **************
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Company {
    employees: Vec<Employee>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Employee {
    name: String,
    role: String,
}

//************** For the third test **************
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Product {
    name: String,
    pricing: PricingModel,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
enum PricingModel {
    Fixed,
    Tiered,
}

//************** For ref inlining tests (mirrors AstGrepRefactorArgs pattern) **************

/// Mimics CaseType enum from AstGrepRefactor
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum MockCaseType {
    LowerCase,
    UpperCase,
    CamelCase,
}

/// Mimics Separator enum from AstGrepRefactor
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum MockSeparator {
    CaseChange,
    Underscore,
    Dash,
}

/// Mimics SubstringTransform
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MockSubstringTransform {
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_char: Option<i32>,
}

/// Mimics ReplaceTransform
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MockReplaceTransform {
    source: String,
    replace: String,
    by: String,
}

/// Mimics ConvertTransform (references MockCaseType and MockSeparator)
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct MockConvertTransform {
    source: String,
    to_case: MockCaseType,
    #[serde(skip_serializing_if = "Option::is_none")]
    separated_by: Option<Vec<MockSeparator>>,
}

/// Mimics Transform enum (tagged union with oneOf, references other definitions)
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
enum MockTransform {
    Substring(MockSubstringTransform),
    Replace(MockReplaceTransform),
    Convert(MockConvertTransform),
}

/// Mimics AstGrepRefactorArgs (HashMap<String, Transform> generates $ref + additionalProperties)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct MockRefactorArgs {
    pattern: String,
    language: String,
    source_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    replacement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transforms: Option<HashMap<String, MockTransform>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    batch: Option<bool>,
}

/// checks if all nested objects have additionalProperties set to false
fn check_add_prps(schema: &Value) -> bool {
    match schema {
        Value::Object(obj) => {
            if obj.get("type") == Some(&Value::String("object".to_string()))
                && obj.get("additionalProperties") != Some(&Value::Bool(false))
            {
                return false;
            }

            for (_, value) in obj.iter() {
                if !check_add_prps(value) {
                    return false;
                }
            }
            true
        }
        Value::Array(arr) => arr.iter().all(check_add_prps),
        _ => true,
    }
}

/// Recursively check that no $ref or definitions/$defs remain in the schema
fn has_no_refs(schema: &Value) -> bool {
    match schema {
        Value::Object(obj) => {
            if obj.contains_key("$ref") {
                return false;
            }
            if obj.contains_key("definitions") {
                return false;
            }
            if obj.contains_key("$defs") {
                return false;
            }
            obj.values().all(has_no_refs)
        }
        Value::Array(arr) => arr.iter().all(has_no_refs),
        _ => true,
    }
}

/// Recursively check that no oneOf remains (should all be converted to anyOf)
fn has_no_one_of(schema: &Value) -> bool {
    match schema {
        Value::Object(obj) => {
            if obj.contains_key("oneOf") {
                return false;
            }
            obj.values().all(has_no_one_of)
        }
        Value::Array(arr) => arr.iter().all(has_no_one_of),
        _ => true,
    }
}

/// Recursively check that no allOf remains (must be unwrapped/merged into parent)
fn has_no_all_of(schema: &Value) -> bool {
    match schema {
        Value::Object(obj) => {
            if obj.contains_key("allOf") {
                return false;
            }
            obj.values().all(has_no_all_of)
        }
        Value::Array(arr) => arr.iter().all(has_no_all_of),
        _ => true,
    }
}

/// Recursively check that no $schema or title keys remain (not valid in function params)
#[allow(dead_code)]
fn has_no_meta_keys(schema: &Value) -> bool {
    match schema {
        Value::Object(obj) => {
            if obj.contains_key("$schema") {
                return false;
            }
            // Only check root-level title (nested titles in definitions are fine after inlining)
            obj.values().all(has_no_meta_keys)
        }
        Value::Array(arr) => arr.iter().all(has_no_meta_keys),
        _ => true,
    }
}

fn make_responses_tool(schema: serde_json::Value) -> ResponsesToolDefinition {
    let tool_def = ToolDefinition {
        name: "test".to_string(),
        description: "Test tool".to_string(),
        parameters: schema,
    };
    ResponsesToolDefinition::from(tool_def)
}

#[test]
fn test_nested_objects() {
    let schema = schema_for!(Person);
    let tool_def = ToolDefinition {
        name: "submit".to_string(),
        description: "Submit".to_string(),
        parameters: serde_json::to_value(schema).unwrap(),
    };
    let response = ResponsesToolDefinition::from(tool_def);

    assert!(
        check_add_prps(&response.parameters),
        "Basic nested objects should have additionalProperties: false"
    );
}

#[test]
fn test_array_items() {
    let schema = schema_for!(Company);
    let tool_def = ToolDefinition {
        name: "submit".to_string(),
        description: "Submit".to_string(),
        parameters: serde_json::to_value(schema).unwrap(),
    };
    let response = ResponsesToolDefinition::from(tool_def);

    assert!(
        check_add_prps(&response.parameters),
        "Array items should have additionalProperties: false"
    );
}

#[test]
fn test_enum_schemas() {
    let schema = schema_for!(Product);
    let tool_def = ToolDefinition {
        name: "submit".to_string(),
        description: "Submit".to_string(),
        parameters: serde_json::to_value(schema).unwrap(),
    };
    let response = ResponsesToolDefinition::from(tool_def);

    assert!(
        check_add_prps(&response.parameters),
        "Enum variants (anyOf/oneOf) should have additionalProperties: false"
    );
}

// ============= New tests for $ref inlining and oneOf conversion =============

#[test]
fn test_ref_inlining_removes_definitions() {
    let schema = serde_json::to_value(schema_for!(MockRefactorArgs)).unwrap();

    // Verify the raw schemars output HAS definitions and $ref
    assert!(
        schema.get("definitions").is_some() || schema.get("$defs").is_some(),
        "schemars should generate definitions for complex types"
    );

    let response = make_responses_tool(schema);

    assert!(
        has_no_refs(&response.parameters),
        "Sanitized schema must not contain $ref or definitions. Got:\n{}",
        serde_json::to_string_pretty(&response.parameters).unwrap()
    );
}

#[test]
fn test_ref_inlining_converts_one_of_to_any_of() {
    let schema = serde_json::to_value(schema_for!(MockRefactorArgs)).unwrap();
    let response = make_responses_tool(schema);

    assert!(
        has_no_one_of(&response.parameters),
        "Sanitized schema must not contain oneOf (should be anyOf). Got:\n{}",
        serde_json::to_string_pretty(&response.parameters).unwrap()
    );
}

#[test]
fn test_ref_inlining_removes_meta_keys() {
    let schema = serde_json::to_value(schema_for!(MockRefactorArgs)).unwrap();

    // Verify raw schema has $schema
    assert!(
        schema.get("$schema").is_some(),
        "schemars should generate $schema key"
    );

    let response = make_responses_tool(schema);

    // $schema should be removed from root
    assert!(
        response.parameters.get("$schema").is_none(),
        "Sanitized schema must not contain $schema at root"
    );
}

#[test]
fn test_tagged_union_enum_inlined_correctly() {
    // MockTransform uses oneOf with object variants (tagged union pattern)
    let schema = serde_json::to_value(schema_for!(MockRefactorArgs)).unwrap();
    let response = make_responses_tool(schema);

    // The transforms field should have its HashMap<String, MockTransform> inlined
    // Navigate to: properties.transforms -> the inner type should have anyOf (not oneOf)
    // and no $ref
    let params = &response.parameters;
    assert!(
        has_no_refs(params),
        "No $ref should remain after inlining"
    );
    assert!(
        has_no_one_of(params),
        "No oneOf should remain after conversion"
    );
}

#[test]
fn test_nested_ref_resolution() {
    // MockConvertTransform references MockCaseType and MockSeparator
    // These are nested $ref (definition referencing another definition)
    let schema = serde_json::to_value(schema_for!(MockRefactorArgs)).unwrap();
    let response = make_responses_tool(schema);

    // After inlining, MockCaseType and MockSeparator should be fully resolved
    // within MockConvertTransform which is within MockTransform
    assert!(
        has_no_refs(&response.parameters),
        "Nested $ref (definition referencing definition) should be fully resolved. Got:\n{}",
        serde_json::to_string_pretty(&response.parameters).unwrap()
    );
}

#[test]
fn test_simple_schema_unchanged() {
    // A simple schema without definitions should pass through unchanged
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "count": {"type": "integer"}
        }
    });

    let response = make_responses_tool(schema.clone());

    // Should still have the same properties
    assert!(response.parameters.get("properties").is_some());
    assert_eq!(
        response.parameters["properties"]["name"]["type"],
        "string"
    );
}

#[test]
fn test_schema_with_only_defs_key() {
    // Test $defs (JSON Schema 2020-12 style) is also handled
    let schema = serde_json::json!({
        "type": "object",
        "$defs": {
            "Color": {
                "type": "string",
                "enum": ["red", "green", "blue"]
            }
        },
        "properties": {
            "favorite": {"$ref": "#/$defs/Color"}
        }
    });

    let response = make_responses_tool(schema);

    assert!(
        has_no_refs(&response.parameters),
        "$defs style references should also be inlined"
    );

    // The inlined value should be the enum directly
    let favorite = &response.parameters["properties"]["favorite"];
    assert_eq!(
        favorite["type"], "string",
        "Inlined $ref should contain the actual type"
    );
}

#[test]
fn test_all_of_unwrapped_for_codex_compatibility() {
    // schemars generates allOf: [{$ref: ...}] for fields referencing other types.
    // After ref resolution the $ref is inlined but the allOf wrapper remains.
    // OpenAI/Codex APIs reject allOf — it must be unwrapped/merged into parent.
    // This is the exact pattern that caused GPT-5.3 Codex to reject AstGrepRefactor's schema:
    //   "toCase": { "allOf": [{"$ref": "#/definitions/CaseType"}] }
    let schema = serde_json::to_value(schema_for!(MockRefactorArgs)).unwrap();
    let response = make_responses_tool(schema);

    assert!(
        has_no_all_of(&response.parameters),
        "Sanitized schema must not contain allOf (must be unwrapped/merged). Got:\n{}",
        serde_json::to_string_pretty(&response.parameters).unwrap()
    );

    // Also verify refs and oneOf are still handled
    assert!(has_no_refs(&response.parameters), "No $ref should remain");
    assert!(has_no_one_of(&response.parameters), "No oneOf should remain");
}

#[test]
fn test_all_of_preserves_sibling_keys() {
    // When allOf is unwrapped, existing sibling keys like "description" must be preserved
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "color": {
                "description": "The color to use",
                "allOf": [
                    {
                        "type": "string",
                        "enum": ["red", "green", "blue"]
                    }
                ]
            }
        }
    });

    let response = make_responses_tool(schema);

    let color = &response.parameters["properties"]["color"];
    assert!(
        !color.as_object().unwrap().contains_key("allOf"),
        "allOf should be removed"
    );
    assert_eq!(color["description"], "The color to use", "description must be preserved");
    assert_eq!(color["type"], "string", "type from allOf item must be merged in");
    assert!(color["enum"].is_array(), "enum from allOf item must be merged in");
}
