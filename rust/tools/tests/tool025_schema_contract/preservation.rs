use codelet_tools::tool_attribution::decorate_definition;
use proptest::{
    prelude::*,
    test_runner::{Config, TestRunner},
};
use rig::completion::ToolDefinition;
use serde_json::{json, Value};

fn definition(parameters: Value) -> ToolDefinition {
    ToolDefinition {
        name: "strict_remote".into(),
        description: "business".into(),
        parameters,
    }
}

pub fn assert_preservation() -> Result<(), Box<dyn std::error::Error>> {
    let mut runner = TestRunner::new(Config {
        cases: 64,
        failure_persistence: None,
        ..Config::default()
    });
    runner.run(&("field_[a-z]{1,12}", any::<bool>()), |(name, strict)| {
        let original = definition(json!({
            "type":"object", "additionalProperties":!strict,
            "$defs":{"business":{"type":"string","minLength":2}},
            "properties":{name.clone():{"$ref":"#/$defs/business"},"target_work_unit_id":{"type":"string"}},
            "required":[name,"target_work_unit_id"]
        }));
        let first = decorate_definition(original.clone()).map_err(|error| TestCaseError::fail(error.to_string()))?;
        let second = decorate_definition(original.clone()).map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(&first, &second);
        prop_assert_eq!(&first.parameters["properties"][&name], &original.parameters["properties"][&name]);
        prop_assert_eq!(&first.parameters["$defs"], &original.parameters["$defs"]);
        prop_assert_eq!(&first.parameters["additionalProperties"], &original.parameters["additionalProperties"]);
        prop_assert_eq!(&first.parameters["properties"]["target_work_unit_id"], &original.parameters["properties"]["target_work_unit_id"]);
        prop_assert!(decorate_definition(first).is_err());
        Ok(())
    })?;
    Ok(())
}

pub fn assert_unsupported_shapes_fail_closed() {
    for schema in [
        json!({"type":"object","nullable":true,"properties":{"business":{"type":"string"}}}),
        json!({"$ref":"#/$defs/business","$defs":{"business":{"type":"object"}}}),
        json!({"type":"object","allOf":[{"additionalProperties":false}]}),
        json!({"type":"object","oneOf":[{"required":["business"]}]}),
        json!({"type":"object","patternProperties":{"^_":{"type":"integer"}}}),
        json!({"type":"object","propertyNames":{"pattern":"^[a-z]+$"}}),
        json!({"type":"object","unevaluatedProperties":false}),
        json!({"type":"object","dependentRequired":{"business":["work_unit_id"]}}),
        json!({"type":"object","properties":{"business":{"$ref":"#"}}}),
        json!({"type":"object","$anchor":"root","properties":{"business":{"$ref":"#root"}}}),
        json!({"type":"object","properties":{"business":{"$ref":""}}}),
        json!({"type":"object","required":["work_unit_id"]}),
    ] {
        let result = decorate_definition(definition(schema.clone()));
        assert!(result.is_err(), "unsafe extension must reject {schema}");
        assert!(result
            .err()
            .is_some_and(|error| error.to_string().contains("strict_remote")));
    }
}
